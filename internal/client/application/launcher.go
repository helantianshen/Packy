package application

import (
	"fmt"
	"os"
	"os/exec"
	"packy/internal/client/infrastructure"
	"path/filepath"
	"runtime"
	"strings"
)

func (c *Client) Launch() error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if !c.canLaunch() {
		return fmt.Errorf("当前状态不允许打开启动器")
	}
	if e := infrastructure.Check(c.Root, c.config.LauncherPath); e != nil {
		return e
	}
	path := filepath.Join(c.Root.Name(), filepath.FromSlash(c.config.LauncherPath))
	var command *exec.Cmd
	switch {
	case strings.HasSuffix(strings.ToLower(path), ".jar"):
		if c.config.JavaPath == "" {
			return fmt.Errorf("JAR 启动器需要构建配置 java_path")
		}
		if e := infrastructure.Check(c.Root, c.config.JavaPath); e != nil {
			return e
		}
		command = exec.Command(filepath.Join(c.Root.Name(), c.config.JavaPath), "-jar", path)
	case strings.HasSuffix(path, ".sh") && runtime.GOOS != "windows":
		command = exec.Command("/bin/sh", path)
	case strings.HasSuffix(path, ".app") && runtime.GOOS == "darwin":
		command = exec.Command("/usr/bin/open", path)
	case strings.HasSuffix(strings.ToLower(path), ".exe") && runtime.GOOS == "windows":
		command = exec.Command(path)
	default:
		return fmt.Errorf("启动器格式不适用于当前平台")
	}
	command.Dir = c.Root.Name()
	if e := command.Start(); e != nil {
		return e
	}
	go func() {
		if e := command.Wait(); e != nil {
			fmt.Fprintf(os.Stderr, "启动器进程结束: %v\n", e)
		}
	}()
	return nil
}

// canLaunch 只能在持有状态锁时读取
func (c *Client) canLaunch() bool {
	return !(c.done != nil || (c.record != nil && c.record.Phase != "completed") || (!c.offline && c.status.Phase != "ready" && c.status.Phase != "completed"))
}
