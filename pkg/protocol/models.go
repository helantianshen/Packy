// Package protocol 定义双端共享的业务数据，不依赖数据库或桌面框架
package protocol

type Source struct {
	Type     string `json:"type"`
	URL      string `json:"url"`
	Priority int    `json:"priority"`
}
type Target struct {
	Path    string   `json:"path"`
	Size    *int64   `json:"size"`
	SHA512  string   `json:"sha512"`
	Sources []Source `json:"sources"`
}
type Operation struct {
	Action  string  `json:"action"`
	ModID   string  `json:"mod_id"`
	OldPath string  `json:"old_path,omitempty"`
	Target  *Target `json:"target,omitempty"`
}
type Plan struct {
	PackID          string      `json:"pack_id"`
	PackName        string      `json:"pack_name"`
	TargetReleaseID string      `json:"target_release_id"`
	TargetVersion   string      `json:"target_version"`
	Changelog       string      `json:"changelog"`
	Operations      []Operation `json:"operations"`
}
type ReleaseItem struct {
	ModID         string `json:"mod_id"`
	ArtifactID    string `json:"artifact_id"`
	ModidSnapshot string `json:"modid_snapshot"`
	NameSnapshot  string `json:"name_snapshot"`
	Target        Target `json:"target"`
}
type APIError struct {
	Code      string `json:"code"`
	Message   string `json:"message"`
	RequestID string `json:"request_id"`
}

func (e *APIError) Error() string  { return e.Message }
func Invalid(message string) error { return &APIError{Code: "INVALID_REQUEST", Message: message} }
