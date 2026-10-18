package application

import (
	"context"
	"crypto/sha512"
	"encoding/hex"
	"errors"
	"fmt"
	"github.com/go-resty/resty/v2"
	"github.com/google/uuid"
	"golang.org/x/sync/errgroup"
	"io"
	"net/http"
	"net/url"
	"os"
	"packy/internal/client/infrastructure"
	"packy/pkg/protocol"
	"path/filepath"
	"regexp"
	"strings"
	"time"
)

var errOffline = errors.New("检查更新网络不可用")

func httpClient() *resty.Client {
	return resty.New().SetTimeout(10 * time.Minute).SetRetryCount(0).SetRedirectPolicy(resty.RedirectPolicyFunc(func(req *http.Request, via []*http.Request) error {
		if len(via) > 5 {
			return fmt.Errorf("重定向过多")
		}
		if _, e := protocol.HTTPURL(req.URL.String()); e != nil {
			return e
		}
		if len(via) > 0 && via[len(via)-1].URL.Scheme == "https" && req.URL.Scheme == "http" {
			return fmt.Errorf("禁止 HTTPS 降级")
		}
		req.Header.Del("Authorization")
		req.Header.Del("Cookie")
		return nil
	}))
}
func (c *Client) requestPlan(ctx context.Context, installed string, out *protocol.Plan) error {
	network := newDownloadNetwork(c.frozenSettings(ctx))
	defer network.close()
	response, e := network.clients["server"].SetTimeout(time.Duration(network.settings.CheckTimeout) * time.Second).R().SetContext(ctx).SetDoNotParseResponse(true).SetBody(map[string]string{"installed_release_id": installed, "client_version": "0.2.0"}).Post(strings.TrimRight(c.config.ServerURL, "/") + "/api/v1/packs/" + c.config.PackID + "/update-plan")
	if e != nil {
		return fmt.Errorf("%w: %s", errOffline, safeNetworkError(e))
	}
	defer response.RawBody().Close()
	if response.StatusCode() != 200 {
		return fmt.Errorf("检查更新失败 (%d)", response.StatusCode())
	}
	data, e := io.ReadAll(io.LimitReader(response.RawBody(), 8*1024*1024+1))
	if e != nil {
		return e
	}
	if len(data) > 8*1024*1024 {
		return fmt.Errorf("计划过大")
	}
	return protocol.Decode(data, out)
}
func verify(root *os.Root, path string, target protocol.Target) error {
	if e := infrastructure.Check(root, path); e != nil {
		return e
	}
	f, e := root.Open(path)
	if e != nil {
		return e
	}
	defer f.Close()
	hash := sha512.New()
	n, e := io.Copy(hash, f)
	if e != nil {
		return e
	}
	if target.Size != nil && n != *target.Size {
		return fmt.Errorf("文件大小不匹配")
	}
	if hex.EncodeToString(hash.Sum(nil)) != target.SHA512 {
		return fmt.Errorf("文件 SHA-512 不匹配")
	}
	return nil
}
func (c *Client) download(ctx context.Context, path string, target protocol.Target) error {
	if verify(c.Root, path, target) == nil {
		info, e := c.Root.Stat(path)
		if e != nil {
			return e
		}
		c.mu.Lock()
		c.status.DownloadedBytes += info.Size()
		c.status.Sequence++
		c.mu.Unlock()
		return nil
	}
	network, cleanup := c.network(ctx)
	defer cleanup()
	ctx = context.WithValue(ctx, networkKey{}, network)
	var lastError string
	for attempt := 0; attempt < network.settings.MaxAttempts; attempt++ {
		if e := ctx.Err(); e != nil {
			return e
		}
		source := target.Sources[attempt%len(target.Sources)]
		e := c.downloadFrom(ctx, path, target, source.URL, source.Type)
		if e == nil {
			return nil
		}
		if ctx.Err() != nil {
			return ctx.Err()
		}
		lastError = safeNetworkError(e)
		fmt.Fprintf(os.Stderr, "下载失败 file=%q source=%q attempt=%d/%d error=%q\n", target.Path, source.Type, attempt+1, network.settings.MaxAttempts, lastError)
		if attempt+1 < network.settings.MaxAttempts {
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(time.Duration(attempt+1) * 500 * time.Millisecond):
			}
		}
	}
	return fmt.Errorf("文件 %s 下载或校验失败: %s；请重试原任务", target.Path, lastError)
}
func (c *Client) downloadFrom(ctx context.Context, path string, target protocol.Target, address, source string) error {
	network, cleanup := c.network(ctx)
	defer cleanup()
	ctx, cancel := context.WithTimeout(ctx, time.Duration(network.settings.DownloadTimeout)*time.Second)
	defer cancel()
	client, ok := network.clients[source]
	if !ok {
		return fmt.Errorf("未知下载渠道")
	}
	response, e := client.R().SetContext(ctx).SetHeader("Accept-Encoding", "identity").SetDoNotParseResponse(true).Get(address)
	if e != nil {
		return e
	}
	defer response.RawBody().Close()
	if response.StatusCode() != 200 {
		return fmt.Errorf("下载返回 %d", response.StatusCode())
	}
	tmp := path + "." + uuid.NewString() + ".part"
	if e = infrastructure.Check(c.Root, tmp); e != nil {
		return e
	}
	file, e := c.Root.OpenFile(tmp, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
	if e != nil {
		return e
	}
	defer c.Root.Remove(tmp)
	hash := sha512.New()
	started := time.Now()
	reader := io.TeeReader(response.RawBody(), hash)
	buffer := make([]byte, 64*1024)
	var total int64
	var counted int64
	committed := false
	defer func() {
		if !committed {
			c.mu.Lock()
			c.status.DownloadedBytes -= counted
			c.status.Sequence++
			c.mu.Unlock()
		}
	}()
	for {
		n, readErr := reader.Read(buffer)
		if n > 0 {
			if network.limiter != nil {
				if e := network.limiter.WaitN(ctx, n); e != nil {
					file.Close()
					return e
				}
			}
			total += int64(n)
			if target.Size != nil && total > *target.Size {
				file.Close()
				return fmt.Errorf("下载超出声明大小")
			}
			if _, e = file.Write(buffer[:n]); e != nil {
				file.Close()
				return e
			}
			c.mu.Lock()
			c.status.DownloadedBytes += int64(n)
			counted += int64(n)
			c.status.BytesPerSecond = int64(float64(total) / time.Since(started).Seconds())
			c.status.Sequence++
			c.mu.Unlock()
		}
		if readErr == io.EOF {
			break
		}
		if readErr != nil {
			file.Close()
			return readErr
		}
		if ctx.Err() != nil {
			file.Close()
			return ctx.Err()
		}
	}
	if target.Size != nil && total != *target.Size {
		file.Close()
		return fmt.Errorf("下载长度不符")
	}
	if hex.EncodeToString(hash.Sum(nil)) != target.SHA512 {
		file.Close()
		return fmt.Errorf("下载哈希不符")
	}
	if e = file.Sync(); e != nil {
		file.Close()
		return e
	}
	if e = file.Close(); e != nil {
		return e
	}
	if e = infrastructure.Replace(c.Root, tmp, path); e != nil {
		return e
	}
	committed = true
	return nil
}
func (c *Client) execute(ctx context.Context, record Record) error {
	network := newDownloadNetwork(c.frozenSettings(ctx))
	defer network.close()
	ctx = context.WithValue(ctx, networkKey{}, network)
	if e := c.phase("downloading"); e != nil {
		return e
	}
	stage := filepath.Join(".updater", "staging", record.TaskID)
	if e := infrastructure.Check(c.Root, stage); e != nil {
		return e
	}
	if e := c.Root.MkdirAll(stage, 0700); e != nil {
		return e
	}
	type file struct {
		path   string
		target protocol.Target
	}
	files := []file{}
	for i, op := range record.Plan.Operations {
		if op.Target != nil {
			files = append(files, file{filepath.Join(stage, fmt.Sprintf("%d.jar", i)), *op.Target})
		}
	}
	group, groupCtx := errgroup.WithContext(ctx)
	group.SetLimit(network.settings.MaxConcurrent)
	for _, f := range files {
		f := f
		group.Go(func() error {
			if e := c.download(groupCtx, f.path, f.target); e != nil {
				return e
			}
			c.mu.Lock()
			c.status.CompletedFiles++
			c.status.Sequence++
			c.mu.Unlock()
			return nil
		})
	}
	if e := group.Wait(); e != nil {
		return e
	}
	if e := c.phase("verifying"); e != nil {
		return e
	}
	for _, f := range files {
		if e := ctx.Err(); e != nil {
			return e
		}
		if e := verify(c.Root, f.path, f.target); e != nil {
			return e
		}
	}
	for _, op := range record.Plan.Operations {
		if op.OldPath != "" {
			if e := regularOrMissing(c.Game, op.OldPath); e != nil {
				return e
			}
		}
		if op.Target != nil {
			if e := regularOrMissing(c.Game, op.Target.Path); e != nil {
				return e
			}
		}
	}
	c.mu.Lock()
	if e := ctx.Err(); e != nil {
		c.mu.Unlock()
		return e
	}
	c.status.Phase = "installing"
	c.status.Sequence++
	c.record.Phase = "installing"
	e := infrastructure.WriteJSON(c.Root, ".updater/task.json", c.record)
	c.mu.Unlock()
	if e != nil {
		return e
	}
	targets := map[string]bool{}
	for _, f := range files {
		targets[protocol.PathKey(f.target.Path)] = true
	}
	for _, op := range record.Plan.Operations {
		if op.OldPath != "" && !targets[protocol.PathKey(op.OldPath)] {
			if e = infrastructure.Check(c.Game, op.OldPath); e != nil {
				return e
			}
			if e = c.Game.Remove(filepath.FromSlash(op.OldPath)); e != nil && !os.IsNotExist(e) {
				return e
			}
		}
	}
	for _, f := range files {
		if e = c.install(f.path, f.target); e != nil {
			return e
		}
	}
	if e = c.phase("saving_state"); e != nil {
		return e
	}
	state := State{record.Plan.TargetReleaseID}
	if e = infrastructure.WriteJSON(c.Root, ".updater/state.json", state); e != nil {
		return e
	}
	c.mu.Lock()
	c.state = state
	c.mu.Unlock()
	if e = c.phase("completed"); e != nil {
		return e
	}
	if e = infrastructure.Check(c.Root, stage); e != nil {
		fmt.Fprintf(os.Stderr, "暂存清理跳过: %v\n", e)
	} else if e = c.Root.RemoveAll(stage); e != nil {
		fmt.Fprintf(os.Stderr, "暂存清理失败: %v\n", e)
	}
	return nil
}
func (c *Client) install(stage string, target protocol.Target) error {
	destination := filepath.FromSlash(target.Path)
	if e := infrastructure.Check(c.Game, destination); e != nil {
		return e
	}
	parent := filepath.Dir(destination)
	if e := c.Game.MkdirAll(parent, 0755); e != nil {
		return e
	}
	tmp := filepath.Join(parent, ".packy-"+uuid.NewString()+".tmp")
	output, e := c.Game.OpenFile(tmp, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0644)
	if e != nil {
		return e
	}
	defer c.Game.Remove(tmp)
	input, e := c.Root.Open(stage)
	if e != nil {
		output.Close()
		return e
	}
	_, e = io.Copy(output, input)
	input.Close()
	if e != nil {
		output.Close()
		return e
	}
	if e = output.Sync(); e != nil {
		output.Close()
		return e
	}
	if e = output.Close(); e != nil {
		return e
	}
	if e = verify(c.Game, tmp, target); e != nil {
		return e
	}
	for attempt := 0; attempt < 3; attempt++ {
		if e = infrastructure.Check(c.Game, destination); e != nil {
			return e
		}
		if info, x := c.Game.Lstat(destination); x == nil && !info.Mode().IsRegular() {
			return fmt.Errorf("目标不是普通文件")
		}
		e = infrastructure.Replace(c.Game, tmp, destination)
		if e == nil {
			return nil
		}
		time.Sleep(200 * time.Millisecond)
	}
	return fmt.Errorf("替换失败，请关闭游戏或启动器后重试: %w", e)
}

// regularOrMissing 仅允许名单指向普通文件，目录不能被当作旧 Mod 删除
func regularOrMissing(root *os.Root, path string) error {
	if e := infrastructure.Check(root, path); e != nil {
		return e
	}
	info, e := root.Lstat(filepath.FromSlash(path))
	if os.IsNotExist(e) {
		return nil
	}
	if e != nil {
		return e
	}
	if !info.Mode().IsRegular() {
		return fmt.Errorf("Mod 路径不是普通文件")
	}
	return nil
}

var networkURL = regexp.MustCompile(`(?i)https?://[^\s"'<>]+`)

// 网络诊断保留错误原因，URL 的凭据与查询参数不进入日志或状态记录
func safeNetworkError(err error) string {
	return networkURL.ReplaceAllStringFunc(err.Error(), func(raw string) string {
		u, e := url.Parse(raw)
		if e != nil {
			return "[URL]"
		}
		u.User = nil
		u.RawQuery = ""
		u.Fragment = ""
		return u.String()
	})
}
