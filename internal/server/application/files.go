package application

import (
	"context"
	"crypto/sha512"
	"encoding/hex"
	"github.com/google/uuid"
	"gorm.io/gorm"
	"io"
	"os"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
	"path/filepath"
	"strings"
)

const UploadLimit int64 = 512 * 1024 * 1024

func (s *Service) Upload(ctx context.Context, a Actor, name string, input io.Reader) (infrastructure.HostedFile, error) {
	var out infrastructure.HostedFile
	if filepath.Base(name) != name || !strings.HasSuffix(strings.ToLower(name), ".jar") || protocol.Relative(name) != nil {
		return out, protocol.Invalid("只允许上传 JAR 文件")
	}
	if e := os.MkdirAll(s.Config.FilesDir, 0700); e != nil {
		return out, e
	}
	f, e := os.CreateTemp(s.Config.FilesDir, ".upload-*")
	if e != nil {
		return out, e
	}
	tmp := f.Name()
	defer os.Remove(tmp)
	hash := sha512.New()
	n, e := io.Copy(io.MultiWriter(f, hash), io.LimitReader(input, UploadLimit+1))
	if e != nil {
		f.Close()
		return out, e
	}
	if n > UploadLimit {
		f.Close()
		return out, protocol.Invalid("上传文件超过 512 MiB")
	}
	if e = f.Sync(); e != nil {
		f.Close()
		return out, e
	}
	if e = f.Close(); e != nil {
		return out, e
	}
	id := uuid.NewString()
	if e = os.Rename(tmp, filepath.Join(s.Config.FilesDir, id+".jar")); e != nil {
		return out, e
	}
	out = infrastructure.HostedFile{ID: id, Filename: name, Size: n, SHA512: hex.EncodeToString(hash.Sum(nil)), URL: strings.TrimRight(s.Config.PublicURL, "/") + "/api/v1/files/" + id + "/download"}
	e = s.Change(ctx, a, "file.upload", "hosted_file", id, nil, func(tx *gorm.DB) (any, any, error) { return nil, out, tx.Create(&out).Error })
	return out, e
}
