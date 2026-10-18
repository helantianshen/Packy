package protocol

import (
	"github.com/stretchr/testify/require"
	"testing"
)

func TestPortablePathsAndStrictJSON(t *testing.T) {
	for _, p := range []string{"../mods/a.jar", "C:/a.jar", `mods\a.jar`, "/mods/a.jar", "mods/CON.jar", "mods/a.jar:stream", "mods/a./x.jar", "mods/../a.jar", "mods/a /x.jar", "mods/LPT¹.jar"} {
		require.Error(t, ModPath(p), p)
	}
	require.NoError(t, ModPath("mods/中文 空格.jar"))
	var out struct {
		A int `json:"a"`
	}
	for _, s := range []string{`{"a":1,"a":2}`, `{"unknown":1}`, `{"a":1} {"a":2}`} {
		require.Error(t, Decode([]byte(s), &out))
	}
}
