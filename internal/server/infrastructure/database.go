package infrastructure

import (
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"strings"

	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"
)

func OpenDatabase(path string, create bool) (*gorm.DB, error) {
	absolute, e := filepath.Abs(path)
	if e != nil {
		return nil, e
	}
	mode := "rw"
	if create {
		mode = "rwc"
		if e = os.MkdirAll(filepath.Dir(absolute), 0700); e != nil {
			return nil, e
		}
	}
	db, e := gorm.Open(sqlite.Open(databaseDSN(filepath.ToSlash(absolute), mode)), &gorm.Config{Logger: logger.Default.LogMode(logger.Silent), TranslateError: true})
	if e != nil {
		return nil, fmt.Errorf("数据库打开失败，请先手动准备结构和初始化数据: %w", e)
	}
	pool, e := db.DB()
	if e != nil {
		return nil, e
	}
	pool.SetMaxOpenConns(1)
	return db, nil
}
func Migrate(db *gorm.DB) error {
	if db.Migrator().HasTable("accounts") || db.Migrator().HasTable("release_items") {
		return fmt.Errorf("旧版数据库不能原地 AutoMigrate，请准备新库并手动导入副本")
	}
	return db.AutoMigrate(
		&Administrator{},
		&Pack{}, &Mod{},
		&Artifact{},
		&Release{},
		&SourceType{},
		&HostedFile{},
		&Audit{},
	)
}
func Ready(db *gorm.DB) error {
	for _, model := range []any{
		&Administrator{},
		&Pack{},
		&Mod{},
		&Artifact{},
		&Release{},
		&SourceType{},
		&HostedFile{},
		&Audit{},
	} {
		if !db.Migrator().HasTable(model) {
			return fmt.Errorf("缺少数据表，请手动执行 migrate 和 init.sql")
		}
	}
	var admin Administrator
	if e := db.First(&admin, 1).Error; e != nil || len(admin.PasswordHash) != 60 {
		return fmt.Errorf("请手动初始化唯一管理员")
	}
	var count int64
	if e := db.Model(&SourceType{}).Where("code IN ?", []string{"modrinth", "curseforge", "github", "server"}).Count(&count).Error; e != nil {
		return e
	}
	if count != 4 {
		return fmt.Errorf("请手动初始化四种下载来源")
	}
	return nil
}

// databaseDSN 接收已转换分隔符的绝对路径，盘符属于路径而不是 URI 主机
func databaseDSN(path, mode string) string {
	if len(path) >= 3 && path[1] == ':' && path[2] == '/' && !strings.HasPrefix(path, "/") {
		path = "/" + path
	}
	u := url.URL{Scheme: "file", Path: path}
	u.RawQuery = url.Values{"mode": {mode}, "_foreign_keys": {"on"}, "_busy_timeout": {"5000"}, "_journal_mode": {"WAL"}, "_txlock": {"immediate"}}.Encode()
	return u.String()
}
