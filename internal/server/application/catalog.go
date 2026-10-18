package application

import (
	"context"
	"github.com/google/uuid"
	"gorm.io/gorm"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
	"path"
	"strings"
)

type PackInput struct {
	Name             string `json:"name"`
	MinecraftVersion string `json:"minecraft_version"`
	Loader           string `json:"loader"`
}
type ModInput struct {
	Modid               string  `json:"modid"`
	Name                string  `json:"name"`
	ModrinthProjectID   *string `json:"modrinth_project_id"`
	CurseforgeProjectID *string `json:"curseforge_project_id"`
}
type ArtifactInput struct {
	ModID        string            `json:"mod_id"`
	Version      string            `json:"version"`
	Filename     string            `json:"filename"`
	Size         *int64            `json:"size"`
	SHA512       string            `json:"sha512"`
	HostedFileID *string           `json:"hosted_file_id"`
	Sources      []protocol.Source `json:"sources"`
}

func (s *Service) SavePack(ctx context.Context, a Actor, id string, in PackInput) (infrastructure.Pack, error) {
	if strings.TrimSpace(in.Name) == "" || in.MinecraftVersion == "" || (in.Loader != "neoforge" && in.Loader != "forge" && in.Loader != "fabric") {
		return infrastructure.Pack{}, protocol.Invalid("整合包名称、MC 版本或加载器无效")
	}
	create := id == ""
	if create {
		id = uuid.NewString()
	}
	var out infrastructure.Pack
	e := s.Change(ctx, a, "pack.save", "pack", id, &id, func(tx *gorm.DB) (any, any, error) {
		var before any
		if !create {
			if e := tx.First(&out, "id = ?", id).Error; e != nil {
				return nil, nil, e
			}
			before = out
		}
		out.ID = id
		out.Name = in.Name
		out.MinecraftVersion = in.MinecraftVersion
		out.Loader = in.Loader
		return before, out, tx.Save(&out).Error
	})
	return out, e
}
func (s *Service) SaveMod(ctx context.Context, a Actor, id string, in ModInput) (infrastructure.Mod, error) {
	if strings.TrimSpace(in.Modid) == "" || strings.TrimSpace(in.Name) == "" {
		return infrastructure.Mod{}, protocol.Invalid("名称和 modid 不能为空")
	}
	create := id == ""
	if create {
		id = uuid.NewString()
	}
	var out infrastructure.Mod
	e := s.Change(ctx, a, "mod.save", "mod", id, nil, func(tx *gorm.DB) (any, any, error) {
		var before any
		if !create {
			if e := tx.First(&out, "id = ?", id).Error; e != nil {
				return nil, nil, e
			}
			before = out
		}
		out = infrastructure.Mod{ID: id, Modid: in.Modid, Name: in.Name, ModrinthProjectID: in.ModrinthProjectID, CurseforgeProjectID: in.CurseforgeProjectID}
		return before, out, tx.Save(&out).Error
	})
	return out, e
}
func (s *Service) CreateArtifact(ctx context.Context, a Actor, in ArtifactInput) (infrastructure.Artifact, error) {
	out := infrastructure.Artifact{ID: uuid.NewString(), ModID: in.ModID, Version: in.Version, Filename: in.Filename, Size: in.Size, SHA512: in.SHA512, HostedFileID: in.HostedFileID, Sources: in.Sources}
	e := s.Change(ctx, a, "artifact.create", "artifact", out.ID, nil, func(tx *gorm.DB) (any, any, error) {
		if in.Version == "" || path.Base(in.Filename) != in.Filename {
			return nil, nil, protocol.Invalid("版本或文件名无效")
		}
		var mod infrastructure.Mod
		if e := tx.First(&mod, "id = ?", in.ModID).Error; e != nil {
			return nil, nil, e
		}
		if in.HostedFileID != nil {
			var file infrastructure.HostedFile
			if e := tx.First(&file, "id = ?", *in.HostedFileID).Error; e != nil {
				return nil, nil, e
			}
			if file.SHA512 != in.SHA512 {
				return nil, nil, protocol.Invalid("托管文件哈希不一致")
			}
			out.Size = &file.Size
		}
		if e := protocol.ValidateTarget(protocol.Target{Path: "mods/" + in.Filename, Size: out.Size, SHA512: out.SHA512, Sources: out.Sources}); e != nil {
			return nil, nil, e
		}
		return nil, out, tx.Create(&out).Error
	})
	return out, e
}
func (s *Service) SaveSource(ctx context.Context, a Actor, code string, in infrastructure.SourceType) (infrastructure.SourceType, error) {
	var out infrastructure.SourceType
	e := s.Change(ctx, a, "source.update", "source", code, nil, func(tx *gorm.DB) (any, any, error) {
		if e := tx.First(&out, "code = ?", code).Error; e != nil {
			return nil, nil, e
		}
		before := out
		if in.Name == "" || in.SortOrder < 0 {
			return nil, nil, protocol.Invalid("来源设置无效")
		}
		out.Name = in.Name
		out.Enabled = in.Enabled
		out.SortOrder = in.SortOrder
		return before, out, tx.Save(&out).Error
	})
	return out, e
}
