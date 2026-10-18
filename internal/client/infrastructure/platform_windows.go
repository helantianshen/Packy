package infrastructure

import (
	"fmt"
	"golang.org/x/sys/windows"
	"os"
)

func platformCheck(path string) error {
	p, e := windows.UTF16PtrFromString(path)
	if e != nil {
		return e
	}
	attributes, e := windows.GetFileAttributes(p)
	if e != nil {
		return e
	}
	if attributes&windows.FILE_ATTRIBUTE_REPARSE_POINT != 0 {
		return fmt.Errorf("路径包含 Windows 重解析点")
	}
	return nil
}
func Replace(root *os.Root, from, to string) error {
	if e := Check(root, from); e != nil {
		return e
	}
	if e := Check(root, to); e != nil {
		return e
	}
	// os.Root 使用目录句柄执行 Windows 原生重命名，避免重新拼接绝对路径逃逸根目录
	return root.Rename(from, to)
}
