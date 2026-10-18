package infrastructure

import (
	"encoding/json"
	"fmt"
	"github.com/google/uuid"
	"io"
	"os"
	"packy/pkg/protocol"
	"path/filepath"
	"strings"
)

// Check 仅检查指定路径及其祖先，拒绝链接、重解析点和特殊文件
func Check(root *os.Root, relative string) error {
	if e := protocol.Relative(filepath.ToSlash(relative)); e != nil {
		return e
	}
	parts := strings.Split(filepath.ToSlash(relative), "/")
	current := ""
	for _, part := range parts {
		current = filepath.Join(current, part)
		info, e := root.Lstat(current)
		if os.IsNotExist(e) {
			continue
		}
		if e != nil {
			return e
		}
		if info.Mode()&os.ModeSymlink != 0 {
			return fmt.Errorf("路径包含符号链接")
		}
		if e = platformCheck(filepath.Join(root.Name(), current)); e != nil {
			return e
		}
		if !info.IsDir() && !info.Mode().IsRegular() {
			return fmt.Errorf("路径包含特殊文件")
		}
	}
	return nil
}
func ReadJSON(root *os.Root, relative string, out any) error {
	if e := Check(root, relative); e != nil {
		return e
	}
	f, e := root.Open(relative)
	if e != nil {
		return e
	}
	defer f.Close()
	data, e := io.ReadAll(io.LimitReader(f, 8*1024*1024+1))
	if e != nil {
		return e
	}
	if len(data) > 8*1024*1024 {
		return fmt.Errorf("本地记录过大")
	}
	return protocol.Decode(data, out)
}

// WriteJSON 只在同目录临时文件同步完成后替换记录
func WriteJSON(root *os.Root, relative string, value any) error {
	if e := Check(root, relative); e != nil {
		return e
	}
	data, e := json.MarshalIndent(value, "", "  ")
	if e != nil {
		return e
	}
	tmp := filepath.Join(filepath.Dir(relative), ".packy-"+uuid.NewString()+".tmp")
	f, e := root.OpenFile(tmp, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
	if e != nil {
		return e
	}
	defer root.Remove(tmp)
	if _, e = f.Write(data); e != nil {
		f.Close()
		return e
	}
	if e = f.Sync(); e != nil {
		f.Close()
		return e
	}
	if e = f.Close(); e != nil {
		return e
	}
	if e = Check(root, relative); e != nil {
		return e
	}
	return Replace(root, tmp, relative)
}
