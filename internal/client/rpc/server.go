package rpc

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"github.com/sourcegraph/jsonrpc2"
	"io"
	"os"
	"packy/config"
	"packy/internal/client/application"
	"packy/pkg/protocol"
	"runtime"
	"strconv"
	"strings"
	"sync"
)

type stdio struct{}

func (stdio) Read(p []byte) (int, error)  { return os.Stdin.Read(p) }
func (stdio) Write(p []byte) (int, error) { return os.Stdout.Write(p) }
func (stdio) Close() error                { return os.Stdin.Close() }

// boundedCodec 为成熟 RPC 流增加帧大小限制，消息关联仍由 jsonrpc2 管理
type boundedCodec struct{ jsonrpc2.VSCodeObjectCodec }

func (boundedCodec) ReadObject(r *bufio.Reader, v any) error {
	length := -1
	headers := 0
	for {
		raw, e := r.ReadSlice('\n')
		line := string(raw)
		headers += len(line)
		if e != nil {
			return e
		}
		if headers > 4096 {
			return fmt.Errorf("RPC header too large")
		}
		if line == "\r\n" {
			break
		}
		if strings.HasPrefix(line, "Content-Length: ") {
			length, e = strconv.Atoi(strings.TrimSpace(strings.TrimPrefix(line, "Content-Length: ")))
			if e != nil {
				return e
			}
		}
	}
	if length <= 0 || length > 8*1024*1024 {
		return fmt.Errorf("RPC frame too large")
	}
	data := make([]byte, length)
	if _, e := io.ReadFull(r, data); e != nil {
		return e
	}
	return json.Unmarshal(data, v)
}

type handler struct {
	mu      sync.Mutex
	active  sync.WaitGroup
	closing bool
	client  *application.Client
	config  config.Client
}

func (h *handler) Handle(ctx context.Context, conn *jsonrpc2.Conn, request *jsonrpc2.Request) {
	h.mu.Lock()
	if h.closing {
		h.mu.Unlock()
		return
	}
	h.active.Add(1)
	defer h.active.Done()
	var result any
	var e error
	if request.Method == "initialize" {
		if h.client != nil {
			e = fmt.Errorf("已经初始化")
		} else {
			var p struct {
				PackRoot        string `json:"pack_root"`
				ProtocolVersion int    `json:"protocol_version"`
			}
			if request.Params == nil {
				e = fmt.Errorf("缺少初始化参数")
			} else {
				e = protocol.Decode(*request.Params, &p)
			}
			if e == nil && p.ProtocolVersion != 1 {
				e = fmt.Errorf("协议不兼容")
			}
			if e == nil {
				h.client, e = application.Open(p.PackRoot, h.config)
			}
			if e == nil {
				result = h.client.Snapshot()
			}
		}
		h.mu.Unlock()
	} else if h.client == nil {
		h.mu.Unlock()
		e = fmt.Errorf("尚未初始化")
	} else {
		h.mu.Unlock()
		switch request.Method {
		case "get_status":
			result = h.client.Snapshot()
		case "check_update":
			e = h.client.Check(ctx)
			result = h.client.Snapshot()
		case "start_update":
			result, e = h.client.Start("")
		case "retry_update", "cancel_update":
			var p struct {
				TaskID string `json:"task_id"`
			}
			if request.Params == nil {
				e = fmt.Errorf("缺少任务 ID")
			} else {
				e = protocol.Decode(*request.Params, &p)
			}
			if e == nil {
				if request.Method == "retry_update" {
					result, e = h.client.Start(p.TaskID)
				} else {
					e = h.client.Cancel(p.TaskID)
				}
			}
		case "set_download_settings":
			var p config.DownloadOverrides
			if request.Params == nil {
				e = fmt.Errorf("缺少下载设置")
			} else {
				e = protocol.Decode(*request.Params, &p)
			}
			if e == nil {
				e = h.client.SetDownload(p)
			}
			result = h.client.Snapshot()
		case "set_background_enabled":
			var p struct {
				Enabled bool `json:"enabled"`
			}
			if request.Params == nil {
				e = fmt.Errorf("缺少偏好参数")
			} else {
				e = protocol.Decode(*request.Params, &p)
			}
			if e == nil {
				e = h.client.SetBackground(p.Enabled)
			}
			result = h.client.Snapshot()
		case "launch_game":
			e = h.client.Launch()
		case "shutdown":
			e = h.client.Shutdown(false)
		default:
			e = fmt.Errorf("未知方法")
		}
	}
	if request.Notif {
		return
	}
	if e != nil {
		_ = conn.ReplyWithError(ctx, request.ID, &jsonrpc2.Error{Code: -32000, Message: e.Error()})
	} else {
		_ = conn.Reply(ctx, request.ID, result)
	}
}
func Run() error {
	c, e := config.Embedded()
	if e != nil {
		return e
	}
	h := &handler{config: c}
	conn := jsonrpc2.NewConn(context.Background(), jsonrpc2.NewBufferedStream(stdio{}, boundedCodec{}), jsonrpc2.AsyncHandler(h))
	_ = conn.Notify(context.Background(), "hello", map[string]any{"protocol_version": 1, "version": "0.2.0", "os": runtime.GOOS, "arch": runtime.GOARCH})
	<-conn.DisconnectNotify()
	h.mu.Lock()
	h.closing = true
	h.mu.Unlock()
	h.active.Wait()
	if h.client != nil {
		return h.client.Close()
	}
	return nil
}
