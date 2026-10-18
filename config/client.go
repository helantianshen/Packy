// Package config 提供构建目标对应的内嵌客户端配置
package config

import (
	"bytes"
	"embed"
	"fmt"
	"github.com/spf13/viper"
	"packy/pkg/protocol"
	"runtime"
)

//go:embed client.*.yaml
var files embed.FS

type Client struct {
	Download     *Download `mapstructure:"download" json:"-"`
	PackName     string    `mapstructure:"pack_name" json:"pack_name"`
	PackID       string    `mapstructure:"pack_id" json:"pack_id"`
	ServerURL    string    `mapstructure:"server_url" json:"server_url"`
	GameDir      string    `mapstructure:"game_dir" json:"game_dir"`
	LauncherPath string    `mapstructure:"launcher_path" json:"launcher_path"`
	JavaPath     string    `mapstructure:"java_path" json:"java_path"`
}

func Embedded() (Client, error) {
	platform := runtime.GOOS
	if platform == "darwin" {
		platform = "macos"
	}
	data, e := files.ReadFile("client." + platform + ".yaml")
	if e != nil {
		return Client{}, e
	}
	return decodeClient(data)
}

func decodeClient(data []byte) (Client, error) {
	v := viper.New()
	v.SetConfigType("yaml")
	if e := v.ReadConfig(bytes.NewReader(data)); e != nil {
		return Client{}, e
	}
	defaults := DefaultDownload()
	c := Client{Download: &defaults}
	if e := v.UnmarshalExact(&c); e != nil {
		return c, e
	}
	return c, Validate(c)
}
func Validate(c Client) error {
	if c.Download != nil {
		if e := c.Download.Validate(); e != nil {
			return e
		}
	}
	if !protocol.ID(c.PackID) || c.PackName == "" {
		return fmt.Errorf("整合包配置身份无效")
	}
	if u, e := protocol.HTTPURL(c.ServerURL); e != nil || u.RawQuery != "" {
		return fmt.Errorf("服务地址无效或包含查询参数")
	}
	if c.JavaPath != "" {
		if e := protocol.Relative(c.JavaPath); e != nil {
			return e
		}
	}
	if c.GameDir != "." {
		if e := protocol.Relative(c.GameDir); e != nil {
			return e
		}
	}
	return protocol.Relative(c.LauncherPath)
}
