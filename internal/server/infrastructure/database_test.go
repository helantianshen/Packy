package infrastructure

import (
	"github.com/stretchr/testify/require"
	"net/url"
	"path/filepath"
	"testing"
)

func TestDatabaseURIPaths(t *testing.T) {
	for _, path := range []string{"C:/整合包 空格/db?#%.sqlite", "D:/Packy/db.sqlite", "//server/share/db.sqlite", "/tmp/整合包/db.sqlite"} {
		u, e := url.Parse(databaseDSN(path, "rw"))
		require.NoError(t, e)
		require.Empty(t, u.Host)
		expected := path
		if path[1] == ':' {
			expected = "/" + path
		}
		require.Equal(t, expected, u.Path)
		require.Equal(t, "rw", u.Query().Get("mode"))
	}
}
func TestDatabaseSpecialCharacterPath(t *testing.T) {
	path := filepath.Join(t.TempDir(), "中文 #%.sqlite")
	db, e := OpenDatabase(path, true)
	require.NoError(t, e)
	pool, e := db.DB()
	require.NoError(t, e)
	require.NoError(t, pool.Close())
	db, e = OpenDatabase(path, false)
	require.NoError(t, e)
	pool, e = db.DB()
	require.NoError(t, e)
	require.NoError(t, pool.Close())
}
