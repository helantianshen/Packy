//go:build !windows

package infrastructure

import (
	"os"
	"path/filepath"
)

func platformCheck(string) error { return nil }
func Replace(root *os.Root, from, to string) error {
	if e := root.Rename(from, to); e != nil {
		return e
	}
	parent, e := root.Open(filepath.Dir(to))
	if e != nil {
		return e
	}
	defer parent.Close()
	return parent.Sync()
}
