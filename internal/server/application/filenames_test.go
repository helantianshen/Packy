package application

import (
	"context"
	"strings"
	"testing"

	"github.com/stretchr/testify/require"
	"packy/internal/server/domain"
	"packy/pkg/protocol"
)

func TestInstallPath(t *testing.T) {
	for _, tc := range []struct{ name, target, want string }{
		{"钠", "mods/sodium.jar", "mods/[钠]sodium.jar"},
		{"钠", "mods/子目录/[钠]sodium.jar", "mods/子目录/[钠]sodium.jar"},
		{"中文:/\\<>\"|?*[]\n名", "mods/a.jar", "mods/[中文____________名]sodium.jar"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			got, err := installPath(tc.target, tc.name, "sodium.jar")
			require.NoError(t, err)
			require.Equal(t, tc.want, got)
		})
	}
	for _, target := range []string{"../mods/a.jar", "mods/../a.jar", `mods\a.jar`} {
		_, err := installPath(target, "钠", "sodium.jar")
		require.Error(t, err)
	}
	_, err := installPath("mods/a.jar", strings.Repeat("中", 100), "sodium.jar")
	require.Error(t, err)
}

func TestFormattedReleasePaths(t *testing.T) {
	s := fixture(t)
	ctx := context.Background()
	a := Actor{Name: "test"}
	m, err := s.SaveMod(ctx, a, "", ModInput{Modid: "sodium", Name: "钠"})
	require.NoError(t, err)
	f, err := s.CreateArtifact(ctx, a, ArtifactInput{ModID: m.ID, Version: "1", Filename: "sodium.jar", SHA512: strings.Repeat("a", 128), Sources: []protocol.Source{{Type: "github", URL: "https://example.com/sodium.jar"}}})
	require.NoError(t, err)
	p, err := s.SavePack(ctx, a, "", PackInput{"测试", "1.21.1", "fabric"})
	require.NoError(t, err)
	d, err := s.CreateDraft(ctx, a, p.ID, CreateDraft{Version: "1"})
	require.NoError(t, err)
	d, err = s.UpdateDraft(ctx, a, p.ID, d.ID, UpdateDraft{Revision: d.Revision, Version: "1", Items: []DraftItem{{m.ID, f.ID, "mods/sodium.jar"}}})
	require.NoError(t, err)
	require.Equal(t, "mods/[钠]sodium.jar", d.Items[0].Target.Path)
	d, err = s.UpdateDraft(ctx, a, p.ID, d.ID, UpdateDraft{Revision: d.Revision, Version: "1", Items: []DraftItem{{m.ID, f.ID, d.Items[0].Target.Path}}})
	require.NoError(t, err)
	require.Equal(t, "mods/[钠]sodium.jar", d.Items[0].Target.Path)
	// 模拟旧版本发布记录，后续发布必须替换其原路径
	legacy := append([]protocol.ReleaseItem(nil), d.Items...)
	legacy[0].Target.Path = "mods/sodium.jar"
	legacyDraft := d
	legacyDraft.Items = legacy
	require.NoError(t, s.DB.Save(&legacyDraft).Error)
	preview, err := s.Preview(ctx, p.ID, d.ID, d.Revision)
	require.NoError(t, err)
	require.Equal(t, "mods/[钠]sodium.jar", preview.Operations[0].Target.Path)
	d, err = s.Publish(ctx, a, p.ID, d.ID, d.Revision)
	require.NoError(t, err)
	require.Equal(t, "mods/[钠]sodium.jar", d.Items[0].Target.Path)
	ops := domain.Diff(legacy, d.Items)
	require.Len(t, ops, 1)
	require.Equal(t, "REPLACE", ops[0].Action)
	// 名称清洗后的路径冲突必须拒绝，不能覆盖其他 Mod
	other := d.Items[0]
	other.ModID = "other"
	f2 := f
	f2.ID = "other-file"
	f2.ModID = other.ModID
	require.NoError(t, s.DB.Create(&f2).Error)
	other.ArtifactID = f2.ID
	_, err = formatReleaseItems(s.DB, append(d.Items, other))
	require.Error(t, err)
}
