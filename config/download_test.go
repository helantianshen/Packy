package config

import (
	"github.com/stretchr/testify/require"
	"testing"
)

func TestDownloadDefaultsAndOverrides(t *testing.T) {
	c, e := Embedded()
	require.NoError(t, e)
	require.NoError(t, c.Download.Validate())
	legacy := []byte("pack_name: test\npack_id: 11111111-1111-4111-8111-111111111111\nserver_url: http://localhost\ngame_dir: .\nlauncher_path: launcher.sh\n")
	c, e = decodeClient(legacy)
	require.NoError(t, e)
	require.Equal(t, DefaultDownload(), *c.Download)
	c, e = decodeClient(append(legacy, []byte("download:\n  max_concurrent_downloads: 7\n  proxies:\n    github: http://localhost:7890\n")...))
	require.NoError(t, e)
	require.Equal(t, 7, c.Download.MaxConcurrent)
	require.Equal(t, 600, c.Download.DownloadTimeout)
	require.Equal(t, "http://localhost:7890", c.Download.Proxies["github"])
	require.Empty(t, c.Download.Proxies["server"])
	d := DefaultDownload()
	d.Proxies["github"] = "http://localhost:7890"
	x := d.Merge(DownloadOverrides{Proxies: map[string]string{"github": ""}})
	require.Empty(t, x.Proxies["github"])
	require.NotEmpty(t, d.Proxies["github"])
	require.Equal(t, d, d.Merge(DownloadOverrides{}))
	for _, proxy := range []string{"socks5://localhost:1080", "http://user:password@localhost:8080", "http://localhost:abc", "http://localhost:8080/path"} {
		x := d.Merge(DownloadOverrides{Proxies: map[string]string{"github": proxy}})
		require.Error(t, x.Validate())
	}
}
