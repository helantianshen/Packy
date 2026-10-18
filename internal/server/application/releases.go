package application

import (
	"context"
	"github.com/google/uuid"
	"gorm.io/gorm"
	"packy/internal/server/domain"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
	"sort"
	"time"
)

type CreateDraft struct {
	Version       string  `json:"version"`
	Changelog     string  `json:"changelog"`
	BaseReleaseID *string `json:"base_release_id"`
}
type DraftItem struct {
	ModID      string `json:"mod_id"`
	ArtifactID string `json:"artifact_id"`
	TargetPath string `json:"target_path"`
}
type UpdateDraft struct {
	Revision  int64       `json:"revision"`
	Version   string      `json:"version"`
	Changelog string      `json:"changelog"`
	Items     []DraftItem `json:"items"`
}

func (s *Service) CreateDraft(ctx context.Context, a Actor, packID string, in CreateDraft) (infrastructure.Release, error) {
	out := infrastructure.Release{ID: uuid.NewString(), PackID: packID, Version: in.Version, Changelog: in.Changelog, Status: "draft", Revision: 1, Items: []protocol.ReleaseItem{}}
	e := s.Change(ctx, a, "draft.create", "release", out.ID, &packID, func(tx *gorm.DB) (any, any, error) {
		if in.Version == "" {
			return nil, nil, protocol.Invalid("版本不能为空")
		}
		var pack infrastructure.Pack
		if e := tx.First(&pack, "id = ?", packID).Error; e != nil {
			return nil, nil, e
		}
		base := in.BaseReleaseID
		if base == nil {
			base = pack.CurrentReleaseID
		}
		if base != nil {
			var r infrastructure.Release
			if e := tx.First(&r, "id = ? AND pack_id = ? AND status = ?", *base, packID, "published").Error; e != nil {
				return nil, nil, e
			}
			out.Items = r.Items
		}
		var e error
		out.Items, e = formatReleaseItems(tx, out.Items)
		if e != nil {
			return nil, nil, e
		}
		out.BaseReleaseID = base
		out.MinecraftVersion = pack.MinecraftVersion
		out.Loader = pack.Loader
		return nil, out, tx.Create(&out).Error
	})
	return out, e
}
func draft(tx *gorm.DB, pack, id string, revision int64) (infrastructure.Release, error) {
	var r infrastructure.Release
	e := tx.First(&r, "id = ? AND pack_id = ?", id, pack).Error
	if e == nil && (r.Status != "draft" || r.Revision != revision) {
		e = Failure("CONFLICT", "草稿已变化或已经发布，请刷新")
	}
	return r, e
}
func (s *Service) UpdateDraft(ctx context.Context, a Actor, pack, id string, in UpdateDraft) (infrastructure.Release, error) {
	var out infrastructure.Release
	e := s.Change(ctx, a, "draft.update", "release", id, &pack, func(tx *gorm.DB) (any, any, error) {
		r, e := draft(tx, pack, id, in.Revision)
		if e != nil {
			return nil, nil, e
		}
		before := r
		items := []protocol.ReleaseItem{}
		for _, item := range in.Items {
			var m infrastructure.Mod
			var f infrastructure.Artifact
			if e = tx.First(&m, "id = ?", item.ModID).Error; e != nil {
				return nil, nil, e
			}
			if e = tx.First(&f, "id = ? AND mod_id = ?", item.ArtifactID, item.ModID).Error; e != nil {
				return nil, nil, e
			}
			target := protocol.Target{Path: item.TargetPath, Size: f.Size, SHA512: f.SHA512, Sources: f.Sources}
			if e = protocol.ValidateTarget(target); e != nil {
				return nil, nil, e
			}
			items = append(items, protocol.ReleaseItem{ModID: m.ID, ArtifactID: f.ID, ModidSnapshot: m.Modid, NameSnapshot: m.Name, Target: target})
		}
		if in.Version == "" {
			return nil, nil, protocol.Invalid("版本不能为空")
		}
		items, e = formatReleaseItems(tx, items)
		if e != nil {
			return nil, nil, e
		}
		r.Items = items
		r.Version = in.Version
		r.Changelog = in.Changelog
		r.Revision++
		out = r
		return before, r, tx.Save(&r).Error
	})
	return out, e
}
func (s *Service) Publish(ctx context.Context, a Actor, pack, id string, revision int64) (infrastructure.Release, error) {
	var out infrastructure.Release
	e := s.Change(ctx, a, "release.publish", "release", id, &pack, func(tx *gorm.DB) (any, any, error) {
		r, e := draft(tx, pack, id, revision)
		if e != nil {
			return nil, nil, e
		}
		before := r
		var p infrastructure.Pack
		if e = tx.First(&p, "id = ?", pack).Error; e != nil {
			return nil, nil, e
		}
		var old infrastructure.Release
		if p.CurrentReleaseID != nil {
			if e = tx.First(&old, "id = ?", *p.CurrentReleaseID).Error; e != nil {
				return nil, nil, e
			}
		}
		r.Items, e = formatReleaseItems(tx, r.Items)
		if e != nil {
			return nil, nil, e
		}
		plan, e := makePlan(tx, p, old, r)
		if e != nil {
			return nil, nil, e
		}
		if e = validateUpgradeHistory(tx, p, r); e != nil {
			return nil, nil, e
		}
		for _, item := range r.Items {
			if e = protocol.ValidateTarget(item.Target); e != nil {
				return nil, nil, e
			}
			if _, e = enabledSources(tx, item.Target.Sources); e != nil {
				return nil, nil, e
			}
		}
		now := time.Now().UTC().Format(time.RFC3339Nano)
		r.Status = "published"
		r.PublishedAt = &now
		r.BaseReleaseID = p.CurrentReleaseID
		r.Revision++
		if e = tx.Save(&r).Error; e != nil {
			return nil, nil, e
		}
		if e = tx.Model(&infrastructure.Pack{}).Where("id = ?", p.ID).Update("current_release_id", id).Error; e != nil {
			return nil, nil, e
		}
		out = r
		return map[string]any{"release": before, "current_release_id": p.CurrentReleaseID}, map[string]any{"release": r, "current_release_id": id, "operations": plan.Operations}, nil
	})
	return out, e
}
func enabledSources(tx *gorm.DB, sources []protocol.Source) ([]protocol.Source, error) {
	var types []infrastructure.SourceType
	if e := tx.Find(&types).Error; e != nil {
		return nil, e
	}
	order := map[string]int{}
	for _, t := range types {
		if t.Enabled {
			order[t.Code] = t.SortOrder
		}
	}
	out := []protocol.Source{}
	for _, s := range sources {
		if _, ok := order[s.Type]; ok {
			out = append(out, s)
		}
	}
	sort.SliceStable(out, func(i, j int) bool {
		if out[i].Priority != out[j].Priority {
			return out[i].Priority < out[j].Priority
		}
		return order[out[i].Type] < order[out[j].Type]
	})
	if len(out) == 0 {
		return nil, Failure("SOURCE_UNAVAILABLE", "文件没有启用的下载来源")
	}
	return out, nil
}
func makePlan(tx *gorm.DB, p infrastructure.Pack, old, target infrastructure.Release) (protocol.Plan, error) {
	plan := protocol.Plan{PackID: p.ID, PackName: p.Name, TargetReleaseID: target.ID, TargetVersion: target.Version, Changelog: target.Changelog, Operations: domain.Diff(old.Items, target.Items)}
	for i := range plan.Operations {
		t := plan.Operations[i].Target
		if t != nil {
			sources, e := enabledSources(tx, t.Sources)
			if e != nil {
				return plan, e
			}
			t.Sources = sources
		}
	}
	return plan, nil
}
func (s *Service) Plan(ctx context.Context, pack, installed string) (protocol.Plan, error) {
	var out protocol.Plan
	e := s.DB.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		var p infrastructure.Pack
		if e := tx.First(&p, "id = ?", pack).Error; e != nil {
			return e
		}
		if p.CurrentReleaseID == nil {
			return Failure("NO_PUBLISHED_RELEASE", "整合包尚未发布")
		}
		var old, target infrastructure.Release
		if e := tx.First(&old, "id = ? AND pack_id = ? AND status = ?", installed, pack, "published").Error; e != nil {
			return e
		}
		if e := tx.First(&target, "id = ? AND pack_id = ? AND status = ?", *p.CurrentReleaseID, pack, "published").Error; e != nil {
			return e
		}
		var e error
		out, e = makePlan(tx, p, old, target)
		if e == nil {
			e = protocol.ValidatePlan(out)
		}
		if e == nil && len(out.Operations) == 0 {
			out.TargetReleaseID = old.ID
			out.TargetVersion = old.Version
		}
		return e
	})
	return out, DatabaseError(e)
}
func (s *Service) Preview(ctx context.Context, pack, id string, revision int64) (protocol.Plan, error) {
	var out protocol.Plan
	e := s.DB.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		r, e := draft(tx, pack, id, revision)
		if e != nil {
			return e
		}
		var p infrastructure.Pack
		if e = tx.First(&p, "id = ?", pack).Error; e != nil {
			return e
		}
		old := infrastructure.Release{}
		if p.CurrentReleaseID != nil {
			if e = tx.First(&old, "id = ?", *p.CurrentReleaseID).Error; e != nil {
				return e
			}
		}
		r.Items, e = formatReleaseItems(tx, r.Items)
		if e != nil {
			return e
		}
		if e = validateUpgradeHistory(tx, p, r); e != nil {
			return e
		}
		out, e = makePlan(tx, p, old, r)
		return e
	})
	return out, DatabaseError(e)
}

// 每个历史正式版本都必须能直接升级，来源开关仅影响实际需要下载的计划
func validateUpgradeHistory(tx *gorm.DB, pack infrastructure.Pack, target infrastructure.Release) error {
	var releases []infrastructure.Release
	if e := tx.Where("pack_id = ? AND status = ?", pack.ID, "published").Find(&releases).Error; e != nil {
		return e
	}
	releases = append(releases, infrastructure.Release{})
	for _, old := range releases {
		plan := protocol.Plan{PackID: pack.ID, TargetReleaseID: target.ID, Operations: domain.Diff(old.Items, target.Items)}
		if e := protocol.ValidatePlan(plan); e != nil {
			return protocol.Invalid("无法从历史版本 " + old.Version + " 升级: " + e.Error())
		}
	}
	return nil
}
