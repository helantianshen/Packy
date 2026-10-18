package application

import (
	"context"
	"encoding/json"
	"github.com/stretchr/testify/require"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
	"strings"
	"testing"
)

func TestRejectPublicationThatBreaksHistoricalUpgrade(t *testing.T) {
	s := fixture(t)
	ctx := context.Background()
	a := Actor{Name: "review"}
	p, e := s.SavePack(ctx, a, "", PackInput{Name: "test", MinecraftVersion: "1.21.1", Loader: "fabric"})
	require.NoError(t, e)
	m, e := s.SaveMod(ctx, a, "", ModInput{Modid: "test", Name: "测试"})
	require.NoError(t, e)
	ids := []string{}
	for i, name := range []string{"A.jar", "B.jar", "a.jar"} {
		version := []string{"1", "2", "3"}[i]
		f, e := s.CreateArtifact(ctx, a, ArtifactInput{ModID: m.ID, Version: version, Filename: name, SHA512: strings.Repeat("a", 128), Sources: []protocol.Source{{Type: "server", URL: "http://127.0.0.1/a.jar"}}})
		require.NoError(t, e)
		r, e := s.CreateDraft(ctx, a, p.ID, CreateDraft{Version: version})
		require.NoError(t, e)
		r, e = s.UpdateDraft(ctx, a, p.ID, r.ID, UpdateDraft{Revision: r.Revision, Version: version, Items: []DraftItem{{ModID: m.ID, ArtifactID: f.ID, TargetPath: "mods/" + name}}})
		require.NoError(t, e)
		if i == 2 {
			_, e = s.Preview(ctx, p.ID, r.ID, r.Revision)
			require.Error(t, e)
			_, e = s.Publish(ctx, a, p.ID, r.ID, r.Revision)
			require.Error(t, e)
			current, e := s.Pack(ctx, p.ID)
			require.NoError(t, e)
			require.Equal(t, ids[1], *current.CurrentReleaseID)
			saved, e := s.Release(ctx, p.ID, r.ID)
			require.NoError(t, e)
			require.Equal(t, "draft", saved.Status)
			return
		}
		r, e = s.Publish(ctx, a, p.ID, r.ID, r.Revision)
		require.NoError(t, e)
		ids = append(ids, r.ID)
	}

}
func TestPublishAuditPreservesPredecessor(t *testing.T) {
	s := fixture(t)
	ctx := context.Background()
	a := Actor{Name: "review"}
	p, e := s.SavePack(ctx, a, "", PackInput{Name: "test", MinecraftVersion: "1.21.1", Loader: "fabric"})
	require.NoError(t, e)
	var first, second string
	for _, v := range []string{"1", "2"} {
		r, e := s.CreateDraft(ctx, a, p.ID, CreateDraft{Version: v})
		require.NoError(t, e)
		r, e = s.Publish(ctx, a, p.ID, r.ID, r.Revision)
		require.NoError(t, e)
		if v == "1" {
			first = r.ID
		} else {
			second = r.ID
			require.NotNil(t, r.BaseReleaseID)
			require.Equal(t, first, *r.BaseReleaseID)
		}
	}
	var audit infrastructure.Audit
	require.NoError(t, s.DB.Where("action = ? AND resource_id = ?", "release.publish", second).First(&audit).Error)
	b, _ := json.Marshal(audit.Changes)
	t.Logf("changes=%s; old=%s; new=%s", b, first, second)
	pointer, ok := audit.Changes["current_release_id"].(map[string]interface{})
	require.True(t, ok, "audit must retain old and new current_release_id")
	require.Equal(t, first, pointer["old"])
	require.Equal(t, second, pointer["new"])
}
