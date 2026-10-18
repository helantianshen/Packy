package application

import (
	"path"
	"strings"
	"unicode"
	"unicode/utf16"

	"golang.org/x/text/unicode/norm"
	"gorm.io/gorm"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
)

// 安装名称使用发布快照和原始文件名，保留目录且不会在重复保存时叠加前缀
func installPath(target, name, filename string) (string, error) {
	if err := protocol.ModPath(target); err != nil {
		return "", err
	}
	if path.Base(filename) != filename {
		return "", protocol.Invalid("原始文件名不能包含目录")
	}
	if err := protocol.ModPath("mods/" + filename); err != nil {
		return "", err
	}
	name = strings.TrimSpace(norm.NFC.String(name))
	name = strings.Map(func(r rune) rune {
		if unicode.IsControl(r) || strings.ContainsRune(`\/:<>"|?*[]`, r) {
			return '_'
		}
		return r
	}, name)
	if name == "" {
		return "", protocol.Invalid("Mod 名称不能为空")
	}
	basename := "[" + name + "]" + filename
	// 同时限制 UTF-8 字节与 UTF-16 码元，避免常见 Linux 与 Windows 文件名上限
	if len(basename) > 255 || len(utf16.Encode([]rune(basename))) > 255 {
		return "", protocol.Invalid("格式化后的文件名过长，请缩短 Mod 名称或原始文件名")
	}
	result := path.Join(path.Dir(target), basename)
	return result, protocol.ModPath(result)
}

// 返回独立切片，避免修改审计的前值或已发布版本；冲突按最终安装路径检查
func formatReleaseItems(tx *gorm.DB, items []protocol.ReleaseItem) ([]protocol.ReleaseItem, error) {
	result := make([]protocol.ReleaseItem, 0, len(items))
	mods, paths := map[string]bool{}, map[string]bool{}
	for _, item := range items {
		var artifact infrastructure.Artifact
		if err := tx.First(&artifact, "id = ? AND mod_id = ?", item.ArtifactID, item.ModID).Error; err != nil {
			return nil, err
		}
		target, err := installPath(item.Target.Path, item.NameSnapshot, artifact.Filename)
		if err != nil {
			return nil, err
		}
		key := protocol.PathKey(target)
		if mods[item.ModID] || paths[key] {
			return nil, protocol.Invalid("Mod 或格式化后的目标路径重复")
		}
		for existing := range paths {
			if strings.HasPrefix(existing, key+"/") || strings.HasPrefix(key, existing+"/") {
				return nil, protocol.Invalid("目标路径存在父子冲突")
			}
		}
		mods[item.ModID], paths[key] = true, true
		item.Target.Path = target
		result = append(result, item)
	}
	return result, nil
}
