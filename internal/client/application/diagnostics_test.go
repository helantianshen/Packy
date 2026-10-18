package application

import (
	"context"
	"fmt"
	"github.com/stretchr/testify/require"
	"net/http"
	"net/http/httptest"
	"os"
	"packy/config"
	"packy/pkg/protocol"
	"strings"
	"testing"
)

func TestDownloadFailureKeepsDiagnostics(t *testing.T) {
	d := config.DefaultDownload()
	d.MaxAttempts = 1
	c := testClient(t, d)
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { http.Error(w, "fixture unavailable", 503) }))
	defer server.Close()
	capture, e := os.CreateTemp(t.TempDir(), "stderr")
	require.NoError(t, e)
	defer capture.Close()
	previous := os.Stderr
	os.Stderr = capture
	defer func() { os.Stderr = previous }()
	e = c.download(context.Background(), ".updater/review.jar", protocol.Target{Path: "mods/review.jar", SHA512: strings.Repeat("a", 128), Sources: []protocol.Source{{Type: "server", URL: server.URL}}})
	require.Error(t, e)
	b, readErr := os.ReadFile(capture.Name())
	require.NoError(t, readErr)
	t.Logf("HTTP 503 returned error=%q stderr=%q", e.Error(), string(b))
	require.Contains(t, e.Error(), "503")
	require.Contains(t, string(b), "503")
	require.Contains(t, string(b), `source="server"`)
	require.Contains(t, string(b), "attempt=1/1")
}

func TestNetworkErrorRedactsURLSecrets(t *testing.T) {
	message := safeNetworkError(fmt.Errorf("Get https://user:password@example.com/file.jar?token=private#secret: HTTP 503"))
	require.Contains(t, message, "HTTP 503")
	require.NotContains(t, message, "password")
	require.NotContains(t, message, "private")
	require.NotContains(t, message, "secret")
}
