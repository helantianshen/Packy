package infrastructure

import "packy/pkg/protocol"

type Administrator struct {
	ID           int    `gorm:"primaryKey;check:single_admin,id = 1" json:"id"`
	Username     string `gorm:"not null;uniqueIndex" json:"username"`
	PasswordHash string `gorm:"not null" json:"-"`
	TokenVersion int64  `json:"-"`
}
type Pack struct {
	ID               string  `gorm:"primaryKey" json:"id"`
	Name             string  `json:"name"`
	MinecraftVersion string  `json:"minecraft_version"`
	Loader           string  `json:"loader"`
	CurrentReleaseID *string `json:"current_release_id"`
}
type Mod struct {
	ID                  string  `gorm:"primaryKey" json:"id"`
	Modid               string  `gorm:"not null;uniqueIndex" json:"modid"`
	Name                string  `json:"name"`
	ModrinthProjectID   *string `json:"modrinth_project_id"`
	CurseforgeProjectID *string `json:"curseforge_project_id"`
}
type Artifact struct {
	ID           string            `gorm:"primaryKey" json:"id"`
	ModID        string            `gorm:"index;not null" json:"mod_id"`
	Version      string            `json:"version"`
	Filename     string            `json:"filename"`
	Size         *int64            `json:"size"`
	SHA512       string            `json:"sha512"`
	HostedFileID *string           `json:"hosted_file_id"`
	Sources      []protocol.Source `gorm:"serializer:json" json:"sources"`
}
type Release struct {
	ID               string                 `gorm:"primaryKey" json:"id"`
	PackID           string                 `gorm:"not null;uniqueIndex:pack_version" json:"pack_id"`
	Version          string                 `gorm:"not null;uniqueIndex:pack_version" json:"version"`
	Changelog        string                 `json:"changelog"`
	MinecraftVersion string                 `json:"minecraft_version"`
	Loader           string                 `json:"loader"`
	Status           string                 `json:"status"`
	Revision         int64                  `json:"revision"`
	BaseReleaseID    *string                `json:"base_release_id"`
	PublishedAt      *string                `json:"published_at"`
	Items            []protocol.ReleaseItem `gorm:"serializer:json" json:"items"`
}
type SourceType struct {
	Code      string `gorm:"primaryKey" json:"code"`
	Name      string `json:"name"`
	Enabled   bool   `json:"enabled"`
	SortOrder int    `json:"sort_order"`
}
type HostedFile struct {
	ID       string `gorm:"primaryKey" json:"id"`
	Filename string `json:"filename"`
	Size     int64  `json:"size"`
	SHA512   string `json:"sha512"`
	URL      string `json:"url"`
}
type Audit struct {
	ID            string         `gorm:"primaryKey" json:"id"`
	OccurredAt    string         `gorm:"index" json:"occurred_at"`
	ActorID       *int           `json:"actor_id"`
	ActorName     string         `json:"actor_name"`
	Action        string         `gorm:"index" json:"action"`
	Result        string         `json:"result"`
	PackID        *string        `gorm:"index" json:"pack_id"`
	ResourceType  string         `json:"resource_type"`
	ResourceID    string         `json:"resource_id"`
	ResourceLabel string         `json:"resource_label"`
	RequestID     string         `gorm:"index" json:"request_id"`
	ClientIP      string         `json:"client_ip"`
	Changes       map[string]any `gorm:"serializer:json" json:"changes"`
	Summary       string         `json:"summary"`
	ErrorCode     string         `json:"error_code"`
}

func (Audit) TableName() string { return "audit_logs" }
