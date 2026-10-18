package application

import (
	"context"
	"encoding/json"
	"github.com/stretchr/testify/require"
	"golang.org/x/crypto/bcrypt"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
	"path/filepath"
	"strings"
	"testing"
)

func fixture(t *testing.T) *Service {
	t.Helper()
	db, e := infrastructure.OpenDatabase(filepath.Join(t.TempDir(), "test.db"), true)
	require.NoError(t, e)
	p, e := db.DB()
	require.NoError(t, e)
	t.Cleanup(func() { p.Close() })
	require.NoError(t, infrastructure.Migrate(db))
	require.Error(t, infrastructure.Ready(db))
	hash, e := bcrypt.GenerateFromPassword([]byte("test-password"), bcrypt.MinCost)
	require.NoError(t, e)
	require.NoError(t, db.Create(&infrastructure.Administrator{ID: 1, Username: "admin", PasswordHash: string(hash)}).Error)
	for _, code := range []string{"modrinth", "curseforge", "github", "server"} {
		require.NoError(t, db.Create(&infrastructure.SourceType{Code: code, Name: code, Enabled: true}).Error)
	}
	require.NoError(t, infrastructure.Ready(db))
	return &Service{DB: db, Config: infrastructure.Config{JWTSecret: strings.Repeat("x", 32)}}
}
func TestPublicationIsolationAuditAndAuth(t *testing.T) {
	s := fixture(t)
	ctx := context.Background()
	id := 1
	a := Actor{ID: &id, Name: "admin", RequestID: "test"}
	token, e := s.Login(ctx, a, "admin", "test-password")
	require.NoError(t, e)
	_, e = s.Authenticate(ctx, token)
	require.NoError(t, e)
	require.NoError(t, s.Logout(ctx, a))
	_, e = s.Authenticate(ctx, token)
	require.Error(t, e)
	p, e := s.SavePack(ctx, a, "", PackInput{"pack", "1.21.1", "neoforge"})
	require.NoError(t, e)
	other, e := s.SavePack(ctx, a, "", PackInput{"other", "1.21.1", "neoforge"})
	require.NoError(t, e)
	base, e := s.CreateDraft(ctx, a, p.ID, CreateDraft{Version: "1"})
	require.NoError(t, e)
	base, e = s.Publish(ctx, a, p.ID, base.ID, base.Revision)
	require.NoError(t, e)
	m, e := s.SaveMod(ctx, a, "", ModInput{Modid: "test", Name: "test"})
	require.NoError(t, e)
	f, e := s.CreateArtifact(ctx, a, ArtifactInput{ModID: m.ID, Version: "1", Filename: "a.jar", SHA512: strings.Repeat("a", 128), Sources: []protocol.Source{{Type: "github", URL: "https://example.com/a.jar?token=private"}}})
	require.NoError(t, e)
	d, e := s.CreateDraft(ctx, a, p.ID, CreateDraft{Version: "2"})
	require.NoError(t, e)
	d, e = s.UpdateDraft(ctx, a, p.ID, d.ID, UpdateDraft{Revision: d.Revision, Version: "2", Items: []DraftItem{{m.ID, f.ID, "mods/a.jar"}}})
	require.NoError(t, e)
	_, e = s.Publish(ctx, a, p.ID, d.ID, d.Revision-1)
	require.Error(t, e)
	d, e = s.Publish(ctx, a, p.ID, d.ID, d.Revision)
	require.NoError(t, e)
	_, e = s.UpdateDraft(ctx, a, p.ID, d.ID, UpdateDraft{Revision: d.Revision, Version: "3"})
	require.Error(t, e)
	plan, e := s.Plan(ctx, p.ID, base.ID)
	require.NoError(t, e)
	require.Len(t, plan.Operations, 1)
	require.Equal(t, "ADD", plan.Operations[0].Action)
	_, e = s.Plan(ctx, other.ID, base.ID)
	require.Error(t, e)
	var audits []infrastructure.Audit
	require.NoError(t, s.DB.Find(&audits).Error)
	data, e := json.Marshal(audits)
	require.NoError(t, e)
	require.NotContains(t, string(data), "private")
	require.NotContains(t, string(data), "test-password")
	require.NotContains(t, string(data), token)
	require.NoError(t, s.DB.Exec(`CREATE TRIGGER reject_success BEFORE INSERT ON audit_logs WHEN NEW.result = 'success' BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END`).Error)
	_, e = s.SavePack(ctx, a, p.ID, PackInput{"must rollback", "1.21.1", "neoforge"})
	require.Error(t, e)
	var actual infrastructure.Pack
	require.NoError(t, s.DB.First(&actual, "id = ?", p.ID).Error)
	require.Equal(t, "pack", actual.Name)
}
