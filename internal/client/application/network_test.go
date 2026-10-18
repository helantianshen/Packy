package application

import (
	"context"
	"crypto/sha512"
	"crypto/tls"
	"crypto/x509"
	"encoding/hex"
	"encoding/json"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/google/uuid"
	"github.com/stretchr/testify/require"
	"packy/config"
	"packy/pkg/protocol"
)

func testClient(t *testing.T, d config.Download) *Client {
	t.Helper()
	root := t.TempDir()
	require.NoError(t, os.Mkdir(filepath.Join(root, ".updater"), 0700))
	state, _ := json.Marshal(State{uuid.NewString()})
	require.NoError(t, os.WriteFile(filepath.Join(root, ".updater/state.json"), state, 0600))
	c, e := Open(root, config.Client{PackName: "test", PackID: uuid.NewString(), ServerURL: "http://127.0.0.1", GameDir: ".", LauncherPath: "launcher.sh", Download: &d})
	require.NoError(t, e)
	t.Cleanup(func() { c.Close() })
	return c
}
func TestSettingsPersistenceAndFrozenTask(t *testing.T) {
	d := config.DefaultDownload()
	d.Proxies["github"] = "http://localhost:7890"
	c := testClient(t, d)
	v := 2
	require.NoError(t, c.SetDownload(config.DownloadOverrides{MaxConcurrent: &v, Proxies: map[string]string{"github": ""}}))
	frozen := c.downloadSettings()
	ctx := context.WithValue(context.Background(), settingsKey{}, frozen)
	require.NoError(t, c.SetBackground(false))
	require.Equal(t, 2, c.Snapshot().Download.MaxConcurrent)
	require.Empty(t, c.Snapshot().Download.Proxies["github"])
	v = 0
	require.Error(t, c.SetDownload(config.DownloadOverrides{MaxConcurrent: &v}))
	require.Equal(t, 2, c.Snapshot().Download.MaxConcurrent)
	require.NoError(t, c.SetDownload(config.DownloadOverrides{}))
	require.Equal(t, 4, c.Snapshot().Download.MaxConcurrent)
	require.Equal(t, 2, c.frozenSettings(ctx).MaxConcurrent)
	data, e := os.ReadFile(filepath.Join(c.Root.Name(), ".updater/preferences.json"))
	require.NoError(t, e)
	var p Preferences
	require.NoError(t, json.Unmarshal(data, &p))
	require.False(t, p.BackgroundEnabled)
	require.Nil(t, p.Download.MaxConcurrent)
}
func TestSourceProxyDirectAndCONNECT(t *testing.T) {
	t.Setenv("HTTP_PROXY", "http://127.0.0.1:1")
	t.Setenv("HTTPS_PROXY", "http://127.0.0.1:1")
	origin := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { io.WriteString(w, "direct") }))
	defer origin.Close()
	tlsOrigin := httptest.NewTLSServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { io.WriteString(w, "secure") }))
	defer tlsOrigin.Close()
	var connects atomic.Int32
	proxy := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodConnect {
			if r.URL.Path == "/redirect" {
				http.Redirect(w, r, "http://cdn.invalid/file", 302)
			} else {
				io.WriteString(w, "proxy")
			}
			return
		}
		connects.Add(1)
		up, e := net.Dial("tcp", r.Host)
		if e != nil {
			http.Error(w, "dial", 502)
			return
		}
		conn, _, e := w.(http.Hijacker).Hijack()
		if e != nil {
			up.Close()
			return
		}
		io.WriteString(conn, "HTTP/1.1 200 Connection Established\r\n\r\n")
		go func() { defer conn.Close(); defer up.Close(); go io.Copy(up, conn); io.Copy(conn, up) }()
	}))
	defer proxy.Close()
	d := config.DefaultDownload()
	d.Proxies["github"] = proxy.URL
	n := newDownloadNetwork(d)
	defer n.close()
	transport, e := n.clients["server"].Transport()
	require.NoError(t, e)
	require.Nil(t, transport.Proxy)
	res, e := n.clients["server"].R().Get(origin.URL)
	require.NoError(t, e)
	require.Equal(t, "direct", res.String())
	res, e = n.clients["github"].R().Get("http://github.invalid/redirect")
	require.NoError(t, e)
	require.Equal(t, "proxy", res.String())
	pool := x509.NewCertPool()
	pool.AddCert(tlsOrigin.Certificate())
	n.clients["github"].SetTLSClientConfig(&tls.Config{RootCAs: pool, MinVersion: tls.VersionTLS12})
	res, e = n.clients["github"].R().Get(tlsOrigin.URL)
	require.NoError(t, e)
	require.Equal(t, "secure", res.String())
	require.EqualValues(t, 1, connects.Load())
}
func TestAttemptBudgetAndSharedLimiter(t *testing.T) {
	var attempts atomic.Int32
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { attempts.Add(1); http.Error(w, "fail", 503) }))
	defer server.Close()
	d := config.DefaultDownload()
	d.MaxAttempts = 2
	c := testClient(t, d)
	sum := sha512.Sum512([]byte("jar"))
	target := protocol.Target{Path: "mods/test.jar", SHA512: hex.EncodeToString(sum[:]), Sources: []protocol.Source{{Type: "server", URL: server.URL}}}
	require.Error(t, c.download(context.Background(), ".updater/test.jar", target))
	require.EqualValues(t, 2, attempts.Load())
	d.LimitKiB = 64
	n := newDownloadNetwork(d)
	defer n.close()
	require.NoError(t, n.limiter.WaitN(context.Background(), 65536))
	start := time.Now()
	var wg sync.WaitGroup
	for i := 0; i < 2; i++ {
		wg.Add(1)
		go func() { defer wg.Done(); n.limiter.WaitN(context.Background(), 32768) }()
	}
	wg.Wait()
	require.GreaterOrEqual(t, time.Since(start), 900*time.Millisecond)
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	require.Error(t, n.limiter.WaitN(ctx, 65536))
}

func TestExecutionConcurrencyUsesStartSnapshot(t *testing.T) {
	var active, peak atomic.Int32
	content := []byte("jar")
	sum := sha512.Sum512(content)
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		now := active.Add(1)
		defer active.Add(-1)
		for old := peak.Load(); now > old; old = peak.Load() {
			if peak.CompareAndSwap(old, now) {
				break
			}
		}
		time.Sleep(60 * time.Millisecond)
		w.Write(content)
	}))
	defer server.Close()
	d := config.DefaultDownload()
	d.MaxConcurrent = 2
	c := testClient(t, d)
	c.plan = &protocol.Plan{PackID: c.config.PackID, TargetReleaseID: uuid.NewString()}
	for i := 0; i < 6; i++ {
		c.plan.Operations = append(c.plan.Operations, protocol.Operation{Action: "ADD", ModID: uuid.NewString(), Target: &protocol.Target{Path: "mods/" + uuid.NewString() + ".jar", SHA512: hex.EncodeToString(sum[:]), Sources: []protocol.Source{{Type: "server", URL: server.URL}}}})
	}
	c.status.Phase = "available"
	_, e := c.Start("")
	require.NoError(t, e)
	v := 8
	require.NoError(t, c.SetDownload(config.DownloadOverrides{MaxConcurrent: &v}))
	wait(t, c)
	require.Equal(t, "completed", c.Snapshot().Status.Phase)
	require.EqualValues(t, 2, peak.Load())
	require.Equal(t, 8, c.Snapshot().Download.MaxConcurrent)
}
func TestDownloadTimeoutCancelsRequest(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { <-r.Context().Done() }))
	defer server.Close()
	c := testClient(t, config.DefaultDownload())
	d := config.DefaultDownload()
	d.DownloadTimeout = 1
	n := newDownloadNetwork(d)
	defer n.close()
	ctx := context.WithValue(context.Background(), networkKey{}, n)
	start := time.Now()
	e := c.downloadFrom(ctx, ".updater/test.jar", protocol.Target{}, server.URL, "server")
	require.Error(t, e)
	require.Less(t, time.Since(start), 3*time.Second)
}
