package application

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"github.com/gofrs/flock"
	"github.com/google/uuid"
	"os"
	"packy/config"
	"packy/internal/client/infrastructure"
	"packy/pkg/protocol"
	"path/filepath"
	"sync"
)

type State struct {
	InstalledReleaseID string `json:"installed_release_id"`
}
type Preferences struct {
	BackgroundEnabled bool                     `json:"background_enabled"`
	Download          config.DownloadOverrides `json:"download"`
}
type Record struct {
	SchemaVersion      int           `json:"schema_version"`
	TaskID             string        `json:"task_id"`
	InstalledReleaseID string        `json:"installed_release_id"`
	Plan               protocol.Plan `json:"plan"`
	Phase              string        `json:"phase"`
}
type Status struct {
	TaskID          string             `json:"task_id"`
	InstanceID      string             `json:"instance_id"`
	Sequence        uint64             `json:"sequence"`
	Phase           string             `json:"phase"`
	CompletedFiles  int                `json:"completed_files"`
	TotalFiles      int                `json:"total_files"`
	DownloadedBytes int64              `json:"downloaded_bytes"`
	TotalBytes      *int64             `json:"total_bytes"`
	BytesPerSecond  int64              `json:"bytes_per_second"`
	Error           *protocol.APIError `json:"error,omitempty"`
}
type Snapshot struct {
	Download         config.Download `json:"download"`
	DownloadDefaults config.Download `json:"download_defaults"`
	CanLaunch        bool            `json:"can_launch"`
	Config           config.Client   `json:"config"`
	Preferences      Preferences     `json:"preferences"`
	State            State           `json:"state"`
	Status           Status          `json:"status"`
	Plan             *protocol.Plan  `json:"plan"`
}
type Client struct {
	mu      sync.Mutex
	Root    *os.Root
	Game    *os.Root
	Lock    *flock.Flock
	config  config.Client
	state   State
	prefs   Preferences
	status  Status
	plan    *protocol.Plan
	record  *Record
	cancel  context.CancelFunc
	done    chan struct{}
	offline bool
}

func Open(rootPath string, c config.Client) (*Client, error) {
	if e := config.Validate(c); e != nil {
		return nil, e
	}
	rootPath, e := filepath.Abs(rootPath)
	if e != nil {
		return nil, e
	}
	info, e := os.Lstat(rootPath)
	if e != nil {
		return nil, e
	}
	if info.Mode()&os.ModeSymlink != 0 {
		return nil, fmt.Errorf("整合包根不能为链接")
	}
	root, e := os.OpenRoot(rootPath)
	if e != nil {
		return nil, e
	}
	ok := false
	defer func() {
		if !ok {
			root.Close()
		}
	}()
	if e = infrastructure.Check(root, ".updater"); e != nil {
		return nil, e
	}
	if e = root.MkdirAll(".updater", 0700); e != nil {
		return nil, e
	}
	if e = infrastructure.Check(root, ".updater/packy.lock"); e != nil {
		return nil, e
	}
	lock := flock.New(filepath.Join(rootPath, ".updater/packy.lock"))
	held, e := lock.TryLock()
	if e != nil {
		return nil, e
	}
	if !held {
		return nil, fmt.Errorf("此整合包已有更新器运行")
	}
	defer func() {
		if !ok {
			lock.Unlock()
		}
	}()
	if c.GameDir != "." {
		if e = infrastructure.Check(root, c.GameDir); e != nil {
			return nil, e
		}
	}
	game, e := root.OpenRoot(c.GameDir)
	if e != nil {
		return nil, e
	}
	defer func() {
		if !ok {
			game.Close()
		}
	}()
	client := &Client{Root: root, Game: game, Lock: lock, config: c, prefs: Preferences{BackgroundEnabled: true}, status: Status{Phase: "idle", InstanceID: uuid.NewString()}}
	if e = infrastructure.ReadJSON(root, ".updater/state.json", &client.state); e != nil {
		return nil, e
	}
	if !protocol.ID(client.state.InstalledReleaseID) {
		return nil, fmt.Errorf("版本记录无效")
	}
	if e = infrastructure.ReadJSON(root, ".updater/preferences.json", &client.prefs); e != nil && !os.IsNotExist(e) {
		return nil, e
	}
	var record Record
	e = infrastructure.ReadJSON(root, ".updater/task.json", &record)
	if e != nil && !os.IsNotExist(e) {
		return nil, e
	}
	if e == nil {
		if (record.SchemaVersion != 1 && record.SchemaVersion != 2) || !protocol.ID(record.TaskID) || record.Plan.PackID != c.PackID || protocol.ValidatePlan(record.Plan) != nil {
			return nil, fmt.Errorf("任务记录无效，请联系管理员")
		}
		validPhase := map[string]bool{"downloading": true, "verifying": true, "installing": true, "saving_state": true, "completed": true, "failed": true, "cancelled": true}
		if !validPhase[record.Phase] || !protocol.ID(record.InstalledReleaseID) {
			return nil, fmt.Errorf("任务记录阶段或版本无效")
		}
		client.record = &record
		client.plan = &record.Plan
		client.status.TaskID = record.TaskID
		if client.state.InstalledReleaseID == record.Plan.TargetReleaseID {
			if record.Phase != "completed" && record.Phase != "saving_state" && record.Phase != "installing" {
				return nil, fmt.Errorf("任务与版本记录不一致")
			}
			record.Phase = "completed"
			if e = infrastructure.WriteJSON(root, ".updater/task.json", record); e != nil {
				return nil, e
			}
			client.status.Phase = "completed"
		} else if client.state.InstalledReleaseID == record.InstalledReleaseID && record.Phase != "completed" {
			client.status.Phase = "failed"
			client.status.Error = &protocol.APIError{Code: "RECOVERY_REQUIRED", Message: "存在未完成任务，请重试原任务"}
		} else {
			return nil, fmt.Errorf("任务与版本记录不匹配")
		}
	}
	if e := client.downloadDefaults().Merge(client.prefs.Download).Validate(); e != nil {
		return nil, e
	}
	ok = true
	return client, nil
}
func (c *Client) Snapshot() Snapshot {
	c.mu.Lock()
	defer c.mu.Unlock()
	v := Snapshot{Download: c.downloadDefaults().Merge(c.prefs.Download), DownloadDefaults: c.downloadDefaults(), CanLaunch: c.canLaunch(), Config: c.config, Preferences: c.prefs, State: c.state, Status: c.status, Plan: c.plan}
	data, _ := json.Marshal(v)
	var copy Snapshot
	_ = json.Unmarshal(data, &copy)
	return copy
}
func (c *Client) phase(phase string) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if phase == "installing" && c.cancel == nil {
		return fmt.Errorf("任务已取消")
	}
	if c.record != nil {
		next := *c.record
		next.Phase = phase
		if e := infrastructure.WriteJSON(c.Root, ".updater/task.json", next); e != nil {
			return e
		}
		c.record = &next
	}
	c.status.Phase = phase
	c.status.Sequence++
	return nil
}
func (c *Client) Check(ctx context.Context) error {
	c.mu.Lock()
	if c.done != nil || c.status.Phase == "checking" || (c.record != nil && c.record.Phase != "completed") {
		c.mu.Unlock()
		return fmt.Errorf("请先处理当前任务")
	}
	ctx, cancel := context.WithCancel(ctx)
	c.cancel = cancel
	c.done = make(chan struct{})
	c.status.TaskID = ""
	c.status.Phase = "checking"
	c.status.Sequence++
	c.offline = false
	installed := c.state.InstalledReleaseID
	ctx = context.WithValue(ctx, settingsKey{}, c.downloadDefaults().Merge(c.prefs.Download))
	c.mu.Unlock()
	var plan protocol.Plan
	e := c.requestPlan(ctx, installed, &plan)
	c.mu.Lock()
	defer c.mu.Unlock()
	c.status.Sequence++
	defer func() { cancel(); c.cancel = nil; close(c.done); c.done = nil }()
	if ctx.Err() != nil {
		c.status.Phase = "idle"
		c.status.Error = nil
		return ctx.Err()
	}
	if e != nil {
		c.status.Phase = "failed"
		c.status.Error = &protocol.APIError{Code: "CHECK_FAILED", Message: "检查失败，请重试；仅网络不可用时允许离线打开启动器"}
		c.offline = errors.Is(e, errOffline)
		return e
	}
	if plan.PackID != c.config.PackID || protocol.ValidatePlan(plan) != nil {
		c.status.Phase = "failed"
		c.status.Error = &protocol.APIError{Code: "INVALID_PLAN", Message: "更新计划无效"}
		return fmt.Errorf("更新计划无效")
	}
	if len(plan.Operations) == 0 && plan.TargetReleaseID != installed {
		c.status.Phase = "failed"
		return fmt.Errorf("不支持空差异版本推进")
	}
	c.plan = &plan
	c.status.Error = nil
	if len(plan.Operations) == 0 {
		c.status.Phase = "ready"
	} else {
		c.status.Phase = "available"
	}
	return nil
}
func (c *Client) Start(retry string) (string, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.done != nil || c.status.Phase == "checking" {
		return "", fmt.Errorf("已有运行中的任务")
	}
	if retry != "" {
		if c.record == nil || retry != c.record.TaskID || c.record.Phase == "completed" {
			return "", fmt.Errorf("任务 ID 无效")
		}
	} else {
		if c.status.Phase != "available" || c.plan == nil {
			return "", fmt.Errorf("没有可执行计划")
		}
		c.record = &Record{SchemaVersion: 2, TaskID: uuid.NewString(), InstalledReleaseID: c.state.InstalledReleaseID, Plan: *c.plan, Phase: "downloading"}
	}
	if c.state.InstalledReleaseID == c.record.Plan.TargetReleaseID {
		next := *c.record
		next.Phase = "completed"
		if e := infrastructure.WriteJSON(c.Root, ".updater/task.json", next); e != nil {
			return "", e
		}
		c.record = &next
		c.status.Phase = "completed"
		c.status.Error = nil
		c.status.Sequence++
		return next.TaskID, nil
	}
	if e := infrastructure.WriteJSON(c.Root, ".updater/task.json", c.record); e != nil {
		return "", e
	}
	c.offline = false
	ctx, cancel := context.WithCancel(context.Background())
	c.cancel = cancel
	c.done = make(chan struct{})
	c.status.TaskID = c.record.TaskID
	c.status.Error = nil
	c.status.CompletedFiles = 0
	c.status.DownloadedBytes = 0
	c.status.TotalFiles = 0
	var total int64
	known := true
	for _, op := range c.record.Plan.Operations {
		if op.Target != nil {
			c.status.TotalFiles++
			if op.Target.Size == nil {
				known = false
			} else {
				total += *op.Target.Size
			}
		}
	}
	c.status.TotalBytes = nil
	if known {
		c.status.TotalBytes = &total
	}
	record := *c.record
	ctx = context.WithValue(ctx, settingsKey{}, c.downloadDefaults().Merge(c.prefs.Download))
	go c.run(ctx, record)
	return record.TaskID, nil
}
func (c *Client) run(ctx context.Context, record Record) {
	e := c.execute(ctx, record)
	c.mu.Lock()
	defer c.mu.Unlock()
	if e != nil {
		phase := "failed"
		if ctx.Err() != nil && c.status.Phase != "installing" && c.status.Phase != "saving_state" {
			phase = "cancelled"
		}
		c.status.Phase = phase
		c.status.Error = &protocol.APIError{Code: "UPDATE_FAILED", Message: e.Error()} /* 失败时保留持久化阶段，重启仍能识别安装是否已开始 */
	} else {
		c.status.Phase = "completed"
		c.status.Error = nil
	}
	c.status.Sequence++
	c.cancel()
	c.cancel = nil
	close(c.done)
	c.done = nil
}
func (c *Client) Cancel(id string) error {
	c.mu.Lock()
	if c.done == nil || c.status.TaskID != id {
		c.mu.Unlock()
		return fmt.Errorf("任务未运行")
	}
	if c.status.Phase == "installing" || c.status.Phase == "saving_state" {
		c.mu.Unlock()
		return fmt.Errorf("正在安装或保存，不能取消")
	}
	cancel, done := c.cancel, c.done
	cancel()
	c.mu.Unlock()
	<-done
	return nil
}
func (c *Client) SetBackground(value bool) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	p := c.prefs
	p.BackgroundEnabled = value
	if e := infrastructure.WriteJSON(c.Root, ".updater/preferences.json", p); e != nil {
		return e
	}
	c.prefs = p
	return nil
}
func (c *Client) Shutdown(orphan bool) error {
	c.mu.Lock()
	done := c.done
	if done != nil {
		critical := c.status.Phase == "installing" || c.status.Phase == "saving_state"
		if critical && !orphan {
			c.mu.Unlock()
			return fmt.Errorf("正在安装或保存，不能退出")
		}
		if !critical {
			c.cancel()
		}
	}
	c.mu.Unlock()
	if done != nil {
		<-done
	}
	return nil
}
func (c *Client) Close() error {
	if e := c.Shutdown(true); e != nil {
		return e
	}
	e := c.Lock.Unlock()
	if x := c.Game.Close(); e == nil {
		e = x
	}
	if x := c.Root.Close(); e == nil {
		e = x
	}
	return e
}

func (c *Client) downloadDefaults() config.Download {
	if c.config.Download != nil {
		return c.config.Download.Merge(config.DownloadOverrides{})
	}
	return config.DefaultDownload()
}
func (c *Client) downloadSettings() config.Download {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.downloadDefaults().Merge(c.prefs.Download)
}

// SetDownload 保存用户覆盖，活动任务仍使用开始时取得的设置副本
func (c *Client) SetDownload(o config.DownloadOverrides) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if e := c.downloadDefaults().Merge(o).Validate(); e != nil {
		return e
	}
	p := c.prefs
	data, e := json.Marshal(o)
	if e != nil {
		return e
	}
	var copy config.DownloadOverrides
	if e = json.Unmarshal(data, &copy); e != nil {
		return e
	}
	p.Download = copy
	if e := infrastructure.WriteJSON(c.Root, ".updater/preferences.json", p); e != nil {
		return e
	}
	c.prefs = p
	c.status.Sequence++
	return nil
}
