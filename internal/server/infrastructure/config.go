package infrastructure

import (
	"errors"
	"fmt"
	"github.com/spf13/viper"
	"os"
	"packy/pkg/protocol"
	"strings"
)

type Config struct {
	Listen            string `mapstructure:"listen"`
	Database          string `mapstructure:"database"`
	FilesDir          string `mapstructure:"files_dir"`
	PublicURL         string `mapstructure:"public_url"`
	JWTSecret         string `mapstructure:"jwt_secret"`
	ModrinthUserAgent string `mapstructure:"modrinth_user_agent"`
	LogFilter         string `mapstructure:"log_filter"`
}

func LoadConfig(serve bool) (Config, error) {
	var c Config
	v := viper.New()
	v.SetConfigType("yaml")
	v.AllowEmptyEnv(true)
	defaults := map[string]any{"listen": "127.0.0.1:8080", "database": "data/packy-go.db", "files_dir": "data/files", "public_url": "http://127.0.0.1:8080", "jwt_secret": "", "modrinth_user_agent": "Packy/0.2", "log_filter": "info"}
	for k, value := range defaults {
		v.SetDefault(k, value)
		if e := v.BindEnv(k, "PACKY_"+strings.ToUpper(k)); e != nil {
			return c, e
		}
	}
	path, explicit := os.LookupEnv("PACKY_CONFIG")
	if !explicit {
		path = "config/server.yaml"
	}
	v.SetConfigFile(path)
	if e := v.ReadInConfig(); e != nil {
		var absent viper.ConfigFileNotFoundError
		if explicit || (!os.IsNotExist(e) && !errors.As(e, &absent)) {
			return c, fmt.Errorf("配置文件无法读取或解析")
		}
	}
	if e := v.UnmarshalExact(&c); e != nil {
		return c, fmt.Errorf("配置字段或类型错误")
	}
	if c.Database == "" {
		return c, fmt.Errorf("database 不能为空")
	}
	if serve {
		if len(c.JWTSecret) < 32 {
			return c, fmt.Errorf("JWT 密钥至少需要 32 字节")
		}
		u, e := protocol.HTTPURL(c.PublicURL)
		if e != nil || u.RawQuery != "" {
			return c, fmt.Errorf("public_url 无效")
		}
		if c.Listen == "" || c.FilesDir == "" {
			return c, fmt.Errorf("服务路径与监听地址不能为空")
		}
	}
	return c, nil
}
