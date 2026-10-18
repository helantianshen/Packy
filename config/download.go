package config

import (
	"fmt"
	"net/url"
	"strings"
)

// Download 是每次网络操作开始时冻结的有效设置，代理按发布来源隔离
type Download struct {
	MaxConcurrent   int               `json:"max_concurrent_downloads" mapstructure:"max_concurrent_downloads"`
	CheckTimeout    int               `json:"check_timeout_seconds" mapstructure:"check_timeout_seconds"`
	DownloadTimeout int               `json:"download_timeout_seconds" mapstructure:"download_timeout_seconds"`
	MaxAttempts     int               `json:"max_attempts" mapstructure:"max_attempts"`
	LimitKiB        int               `json:"speed_limit_kib" mapstructure:"speed_limit_kib"`
	Proxies         map[string]string `json:"proxies" mapstructure:"proxies"`
}

// DownloadOverrides 的缺失字段继承默认值，显式空代理表示直连
type DownloadOverrides struct {
	MaxConcurrent   *int              `json:"max_concurrent_downloads,omitempty"`
	CheckTimeout    *int              `json:"check_timeout_seconds,omitempty"`
	DownloadTimeout *int              `json:"download_timeout_seconds,omitempty"`
	MaxAttempts     *int              `json:"max_attempts,omitempty"`
	LimitKiB        *int              `json:"speed_limit_kib,omitempty"`
	Proxies         map[string]string `json:"proxies,omitempty"`
}

// DefaultDownload 为旧配置提供与发行默认值一致的兼容设置
func DefaultDownload() Download {
	return Download{4, 30, 600, 3, 0, map[string]string{"modrinth": "", "curseforge": "", "github": "", "server": ""}}
}

// Merge 只覆盖明确指定的字段，并复制代理映射避免修改原默认值
func (d Download) Merge(o DownloadOverrides) Download {
	p := map[string]string{}
	for k, v := range d.Proxies {
		p[k] = v
	}
	d.Proxies = p
	if o.MaxConcurrent != nil {
		d.MaxConcurrent = *o.MaxConcurrent
	}
	if o.CheckTimeout != nil {
		d.CheckTimeout = *o.CheckTimeout
	}
	if o.DownloadTimeout != nil {
		d.DownloadTimeout = *o.DownloadTimeout
	}
	if o.MaxAttempts != nil {
		d.MaxAttempts = *o.MaxAttempts
	}
	if o.LimitKiB != nil {
		d.LimitKiB = *o.LimitKiB
	}
	for k, v := range o.Proxies {
		d.Proxies[k] = v
	}
	return d
}

// Validate 拒绝超范围数值和含凭据的代理地址
func (d Download) Validate() error {
	if d.MaxConcurrent < 1 || d.MaxConcurrent > 16 {
		return fmt.Errorf("最大并发数必须为1–16")
	}
	if d.CheckTimeout < 5 || d.CheckTimeout > 300 {
		return fmt.Errorf("检查超时必须为5–300秒")
	}
	if d.DownloadTimeout < 30 || d.DownloadTimeout > 3600 {
		return fmt.Errorf("下载超时必须为30–3600秒")
	}
	if d.MaxAttempts < 1 || d.MaxAttempts > 10 {
		return fmt.Errorf("最大尝试次数必须为1–10")
	}
	if d.LimitKiB < 0 || d.LimitKiB > 1048576 {
		return fmt.Errorf("限速必须为0–1048576 KiB/s，0表示不限速")
	}
	for k, v := range d.Proxies {
		switch k {
		case "modrinth", "curseforge", "github", "server":
		default:
			return fmt.Errorf("未知代理渠道")
		}
		if v == "" {
			continue
		}
		u, e := url.Parse(v)
		if e != nil || u == nil || u.Hostname() == "" || (u.Scheme != "http" && u.Scheme != "https") || u.User != nil || u.RawQuery != "" || u.Fragment != "" || (u.Path != "" && u.Path != "/") || strings.TrimSpace(v) != v {
			return fmt.Errorf("代理需为无账号密码的 HTTP/HTTPS 地址，留空直连")
		}
	}
	return nil
}
