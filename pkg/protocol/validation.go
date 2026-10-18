package protocol

import (
	"bytes"
	"encoding/json"
	"fmt"
	"github.com/google/uuid"
	"golang.org/x/text/unicode/norm"
	"io"
	"net/url"
	"regexp"
	"strings"
	"unicode"
)

var hashPattern = regexp.MustCompile(`^[0-9a-f]{128}$`)

func ID(s string) bool   { _, e := uuid.Parse(s); return e == nil }
func Hash(s string) bool { return hashPattern.MatchString(s) }
func HTTPURL(s string) (*url.URL, error) {
	u, e := url.Parse(s)
	if e != nil || (u.Scheme != "http" && u.Scheme != "https") || u.Host == "" || u.User != nil || u.Fragment != "" {
		return nil, Invalid("地址必须是无凭据的 HTTP/HTTPS URL")
	}
	return u, nil
}

// Relative 在拼接前检查原始协议路径，所有系统使用相同的 Windows 兼容限制
func Relative(s string) error {
	if s == "" || strings.ContainsAny(s, `\:<>"|?*`) || strings.HasPrefix(s, "/") {
		return Invalid("非法相对路径")
	}
	for _, part := range strings.Split(s, "/") {
		if part == "" || part == "." || part == ".." || strings.HasSuffix(part, ".") || strings.HasSuffix(part, " ") {
			return Invalid("路径包含非法段")
		}
		for _, c := range part {
			if unicode.IsControl(c) {
				return Invalid("路径包含控制字符")
			}
		}
		base := strings.ToUpper(strings.SplitN(part, ".", 2)[0])
		if base == "CON" || base == "PRN" || base == "AUX" || base == "NUL" || regexp.MustCompile(`^(COM|LPT)[1-9¹²³]$`).MatchString(base) {
			return Invalid("路径使用保留设备名")
		}
	}
	return nil
}
func ModPath(s string) error {
	if e := Relative(s); e != nil {
		return e
	}
	if !strings.HasPrefix(s, "mods/") || !strings.HasSuffix(strings.ToLower(s), ".jar") {
		return Invalid("只允许 mods 下的 JAR 文件")
	}
	return nil
}
func PathKey(s string) string { return strings.ToLower(norm.NFC.String(s)) }
func ValidateTarget(t Target) error {
	if e := ModPath(t.Path); e != nil {
		return e
	}
	if !Hash(t.SHA512) {
		return Invalid("SHA-512 格式错误")
	}
	if t.Size != nil && (*t.Size < 0 || *t.Size > 9007199254740991) {
		return Invalid("文件大小无效")
	}
	if len(t.Sources) == 0 {
		return Invalid("文件缺少下载来源")
	}
	for _, s := range t.Sources {
		if s.Type != "modrinth" && s.Type != "curseforge" && s.Type != "github" && s.Type != "server" {
			return Invalid("未知来源")
		}
		if s.Priority < 0 {
			return Invalid("来源优先级无效")
		}
		if _, e := HTTPURL(s.URL); e != nil {
			return e
		}
	}
	return nil
}
func ValidatePlan(p Plan) error {
	if !ID(p.PackID) || !ID(p.TargetReleaseID) {
		return Invalid("更新计划身份无效")
	}
	targets := map[string]string{}
	oldPaths := map[string]string{}
	mods := map[string]bool{}
	for _, o := range p.Operations {
		if !ID(o.ModID) || mods[o.ModID] {
			return Invalid("重复或无效 Mod 身份")
		}
		mods[o.ModID] = true
		switch o.Action {
		case "ADD":
			if o.OldPath != "" || o.Target == nil {
				return Invalid("ADD 字段无效")
			}
		case "REPLACE":
			if o.OldPath == "" || o.Target == nil {
				return Invalid("REPLACE 字段无效")
			}
		case "REMOVE":
			if o.OldPath == "" || o.Target != nil {
				return Invalid("REMOVE 字段无效")
			}
		default:
			return Invalid("未知操作")
		}
		if o.OldPath != "" {
			if prior, ok := oldPaths[PathKey(o.OldPath)]; ok {
				return Invalid("旧路径冲突: " + prior)
			}
			oldPaths[PathKey(o.OldPath)] = o.OldPath
			if e := ModPath(o.OldPath); e != nil {
				return e
			}
		}
		if o.Target != nil {
			if e := ValidateTarget(*o.Target); e != nil {
				return e
			}
			k := PathKey(o.Target.Path)
			if _, exists := targets[k]; exists {
				return Invalid("目标路径冲突")
			}
			targets[k] = o.Target.Path
		}
	}
	for key, actual := range targets {
		if old, exists := oldPaths[key]; exists && old != actual {
			return Invalid("不支持仅大小写或 Unicode 规范不同的路径改名")
		}
		for other := range targets {
			if strings.HasPrefix(key, other+"/") {
				return Invalid("目标路径存在父子冲突")
			}
		}
	}
	return nil
}

// Decode 拒绝重复键和未知字段，避免不同语言对同一输入作不同解释
func Decode(data []byte, out any) error {
	d := json.NewDecoder(bytes.NewReader(data))
	if e := uniqueJSON(d); e != nil {
		return Invalid("JSON 内容无效或包含重复字段")
	}
	if _, e := d.Token(); e != io.EOF {
		return Invalid("只能包含一个 JSON 值")
	}
	d = json.NewDecoder(bytes.NewReader(data))
	d.DisallowUnknownFields()
	if e := d.Decode(out); e != nil {
		return Invalid("请求字段或类型无效")
	}
	return nil
}
func uniqueJSON(d *json.Decoder) error {
	t, e := d.Token()
	if e != nil {
		return e
	}
	delim, ok := t.(json.Delim)
	if !ok {
		return nil
	}
	switch delim {
	case '{':
		seen := map[string]bool{}
		for d.More() {
			k, e := d.Token()
			if e != nil {
				return e
			}
			key, ok := k.(string)
			if !ok || seen[key] {
				return fmt.Errorf("duplicate key")
			}
			seen[key] = true
			if e = uniqueJSON(d); e != nil {
				return e
			}
		}
	case '[':
		for d.More() {
			if e = uniqueJSON(d); e != nil {
				return e
			}
		}
	default:
		return fmt.Errorf("unexpected delimiter")
	}
	_, e = d.Token()
	return e
}
