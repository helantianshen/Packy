package application

import (
	"context"
	"crypto/sha512"
	"encoding/hex"
	"encoding/json"
	"github.com/google/uuid"
	"github.com/stretchr/testify/require"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"packy/config"
	"packy/pkg/protocol"
	"path/filepath"
	"testing"
	"time"
)

func TestUpdateRecoveryAndUnlistedFiles(t *testing.T) {
	old, target, pack, mod := uuid.NewString(), uuid.NewString(), uuid.NewString(), uuid.NewString()
	content := []byte("new jar")
	sum := sha512.Sum512(content)
	size := int64(len(content))
	bad := true
	requests := 0
	var plan protocol.Plan
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/file" {
			if bad {
				w.Write([]byte("corrupt"))
			} else {
				w.Write(content)
			}
			return
		}
		requests++
		json.NewEncoder(w).Encode(plan)
	}))
	defer server.Close()
	plan = protocol.Plan{PackID: pack, TargetReleaseID: target, Operations: []protocol.Operation{{Action: "REPLACE", ModID: mod, OldPath: "mods/old.jar", Target: &protocol.Target{Path: "mods/中文 空格.jar", Size: &size, SHA512: hex.EncodeToString(sum[:]), Sources: []protocol.Source{{Type: "server", URL: server.URL + "/file"}}}}}}
	root := t.TempDir()
	require.NoError(t, os.MkdirAll(filepath.Join(root, ".updater"), 0700))
	require.NoError(t, os.MkdirAll(filepath.Join(root, "mods"), 0755))
	state, _ := json.Marshal(State{old})
	require.NoError(t, os.WriteFile(filepath.Join(root, ".updater/state.json"), state, 0600))
	require.NoError(t, os.WriteFile(filepath.Join(root, "mods/old.jar"), []byte("old"), 0644))
	require.NoError(t, os.WriteFile(filepath.Join(root, "mods/player.jar"), []byte("sentinel"), 0644))
	cfg := config.Client{PackID: pack, PackName: "test", ServerURL: server.URL, GameDir: ".", LauncherPath: "launcher.sh"}
	c, e := Open(root, cfg)
	require.NoError(t, e)
	_, e = Open(root, cfg)
	require.Error(t, e, "整合包锁必须排他")
	require.NoError(t, c.Check(context.Background()))
	id, e := c.Start("")
	require.NoError(t, e)
	wait(t, c)
	require.Equal(t, "failed", c.Snapshot().Status.Phase)
	data, e := os.ReadFile(filepath.Join(root, "mods/old.jar"))
	require.NoError(t, e)
	require.Equal(t, "old", string(data))
	require.Equal(t, old, c.Snapshot().State.InstalledReleaseID)
	require.Error(t, c.Launch())
	require.NoError(t, c.Close())
	bad = false
	plan.TargetReleaseID = uuid.NewString()
	c, e = Open(root, cfg)
	require.NoError(t, e)
	defer c.Close()
	require.Equal(t, "failed", c.Snapshot().Status.Phase)
	require.Error(t, c.Check(context.Background()))
	_, e = c.Start(id)
	require.NoError(t, e)
	wait(t, c)
	require.Equal(t, "completed", c.Snapshot().Status.Phase)
	require.Equal(t, target, c.Snapshot().State.InstalledReleaseID)
	require.Equal(t, 1, requests, "恢复不请求最新发布")
	data, e = os.ReadFile(filepath.Join(root, "mods/中文 空格.jar"))
	require.NoError(t, e)
	require.Equal(t, content, data)
	_, e = os.Stat(filepath.Join(root, "mods/old.jar"))
	require.True(t, os.IsNotExist(e))
	data, e = os.ReadFile(filepath.Join(root, "mods/player.jar"))
	require.NoError(t, e)
	require.Equal(t, "sentinel", string(data))
}
func wait(t *testing.T, c *Client) {
	t.Helper()
	require.Eventually(t, func() bool { c.mu.Lock(); defer c.mu.Unlock(); return c.done == nil }, 10*time.Second, 10*time.Millisecond)
}
func TestUnsafeFilesystem(t *testing.T) {
	root := t.TempDir()
	r, e := os.OpenRoot(root)
	require.NoError(t, e)
	defer r.Close()
	require.NoError(t, os.Mkdir(filepath.Join(root, "directory.jar"), 0755))
	require.Error(t, regularOrMissing(r, "directory.jar"))
	outside := t.TempDir()
	if e = os.Symlink(outside, filepath.Join(root, "mods")); e == nil {
		require.Error(t, regularOrMissing(r, "mods/a.jar"))
	}
}

func TestCheckCancellation(t *testing.T) {
	started := make(chan struct{})
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		io.Copy(io.Discard, r.Body)
		close(started)
		<-r.Context().Done()
	}))
	defer server.Close()
	root := t.TempDir()
	require.NoError(t, os.Mkdir(filepath.Join(root, ".updater"), 0700))
	data, _ := json.Marshal(State{uuid.NewString()})
	require.NoError(t, os.WriteFile(filepath.Join(root, ".updater/state.json"), data, 0600))
	c, e := Open(root, config.Client{PackID: uuid.NewString(), PackName: "test", ServerURL: server.URL, GameDir: ".", LauncherPath: "launcher.sh"})
	require.NoError(t, e)
	defer c.Close()
	checked := make(chan error, 1)
	go func() { checked <- c.Check(context.Background()) }()
	<-started
	require.Error(t, c.Check(context.Background()))
	require.Error(t, c.Launch())
	require.NoError(t, c.Cancel(""))
	require.Error(t, <-checked)
	require.Equal(t, "idle", c.Snapshot().Status.Phase)
	require.False(t, c.Snapshot().CanLaunch)
}
