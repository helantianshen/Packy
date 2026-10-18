package application

import (
	"context"
	"net/url"
	"packy/internal/server/domain"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
)

func (s *Service) List(ctx context.Context, kind, pack string, page, size int, params url.Values) (any, error) {
	if page < 1 || size < 1 || size > 100 {
		return nil, protocol.Invalid("分页参数无效")
	}
	db := s.DB.WithContext(ctx)
	var model, out any
	switch kind {
	case "packs":
		model = &infrastructure.Pack{}
		v := []infrastructure.Pack{}
		out = &v
	case "mods":
		model = &infrastructure.Mod{}
		v := []infrastructure.Mod{}
		out = &v
		if q := params.Get("keyword"); q != "" {
			db = db.Where("instr(name, ?) > 0 OR instr(modid, ?) > 0", q, q)
		}
	case "artifacts":
		model = &infrastructure.Artifact{}
		v := []infrastructure.Artifact{}
		out = &v
		if id := params.Get("mod_id"); id != "" {
			db = db.Where("mod_id = ?", id)
		}
	case "releases":
		model = &infrastructure.Release{}
		v := []infrastructure.Release{}
		out = &v
		db = db.Where("pack_id = ?", pack)
	case "audit":
		model = &infrastructure.Audit{}
		v := []infrastructure.Audit{}
		out = &v
		for _, field := range []string{"pack_id", "action", "result", "resource_id"} {
			if value := params.Get(field); value != "" {
				db = db.Where(field+" = ?", value)
			}
		}
		if value := params.Get("from"); value != "" {
			db = db.Where("occurred_at >= ?", value)
		}
		if value := params.Get("to"); value != "" {
			db = db.Where("occurred_at <= ?", value)
		}
		db = db.Order("occurred_at DESC")
	}
	var count int64
	if e := db.Model(model).Count(&count).Error; e != nil {
		return nil, e
	}
	e := db.Model(model).Order("id DESC").Offset((page - 1) * size).Limit(size).Find(out).Error
	return map[string]any{"items": out, "total": count, "page": page, "page_size": size}, e
}
func (s *Service) Pack(ctx context.Context, id string) (infrastructure.Pack, error) {
	var v infrastructure.Pack
	e := s.DB.WithContext(ctx).First(&v, "id = ?", id).Error
	return v, e
}
func (s *Service) Release(ctx context.Context, pack, id string) (infrastructure.Release, error) {
	var v infrastructure.Release
	e := s.DB.WithContext(ctx).First(&v, "id = ? AND pack_id = ?", id, pack).Error
	return v, e
}
func (s *Service) Audit(ctx context.Context, id string) (infrastructure.Audit, error) {
	var v infrastructure.Audit
	e := s.DB.WithContext(ctx).First(&v, "id = ?", id).Error
	return v, e
}
func (s *Service) SourceTypes(ctx context.Context) ([]infrastructure.SourceType, error) {
	v := []infrastructure.SourceType{}
	e := s.DB.WithContext(ctx).Order("sort_order, code").Find(&v).Error
	return v, e
}
func (s *Service) HostedFile(ctx context.Context, id string) (infrastructure.HostedFile, error) {
	var v infrastructure.HostedFile
	e := s.DB.WithContext(ctx).First(&v, "id = ?", id).Error
	return v, e
}

// ReleaseDiff 使用冻结快照比较真实发布前驱，不受当前来源开关影响
func (s *Service) ReleaseDiff(ctx context.Context, pack, id string) (protocol.Plan, error) {
	r, e := s.Release(ctx, pack, id)
	if e != nil {
		return protocol.Plan{}, e
	}
	if r.Status != "published" {
		return protocol.Plan{}, Failure("CONFLICT", "仅正式版可查看历史差异")
	}
	var old infrastructure.Release
	if r.BaseReleaseID != nil {
		old, e = s.Release(ctx, pack, *r.BaseReleaseID)
		if e != nil {
			return protocol.Plan{}, e
		}
	}
	return protocol.Plan{PackID: pack, TargetReleaseID: id, TargetVersion: r.Version, Changelog: r.Changelog, Operations: domain.Diff(old.Items, r.Items)}, nil
}
