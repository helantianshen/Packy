package application

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"github.com/google/uuid"
	"gorm.io/gorm"
	"net/url"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
	"strings"
	"time"
)

type Service struct {
	DB     *gorm.DB
	Config infrastructure.Config
}
type Actor struct {
	ID                  *int
	Name, RequestID, IP string
}

func Failure(code, message string) error { return &protocol.APIError{Code: code, Message: message} }
func DatabaseError(e error) error {
	if errors.Is(e, gorm.ErrRecordNotFound) {
		return Failure("NOT_FOUND", "记录不存在")
	}
	if errors.Is(e, gorm.ErrDuplicatedKey) || errors.Is(e, gorm.ErrForeignKeyViolated) {
		return Failure("CONFLICT", "记录冲突或引用无效")
	}
	return e
}

// Change 在同一事务保存业务与审计，失败不会留下成功记录
func (s *Service) Change(ctx context.Context, a Actor, action, kind, id string, pack *string, fn func(*gorm.DB) (any, any, error)) error {
	e := s.DB.WithContext(ctx).Transaction(func(tx *gorm.DB) error {
		before, after, e := fn(tx)
		if e != nil {
			return e
		}
		changes, e := difference(before, after)
		if e != nil {
			return e
		}
		return tx.Create(&infrastructure.Audit{ID: uuid.NewString(), OccurredAt: time.Now().UTC().Format(time.RFC3339Nano), ActorID: a.ID, ActorName: a.Name, Action: action, Result: "success", PackID: pack, ResourceType: kind, ResourceID: id, RequestID: a.RequestID, ClientIP: a.IP, Changes: changes, Summary: action}).Error
	})
	if e != nil {
		code := "INTERNAL_ERROR"
		var api *protocol.APIError
		if errors.As(e, &api) {
			code = api.Code
		}
		auditErr := s.DB.WithContext(context.WithoutCancel(ctx)).Create(&infrastructure.Audit{ID: uuid.NewString(), OccurredAt: time.Now().UTC().Format(time.RFC3339Nano), ActorID: a.ID, ActorName: a.Name, Action: action, Result: "failed", PackID: pack, ResourceType: kind, ResourceID: id, RequestID: a.RequestID, ClientIP: a.IP, Changes: map[string]any{}, ErrorCode: code, Summary: "操作未提交"}).Error
		if auditErr != nil {
			return fmt.Errorf("业务和失败审计均未提交: %w", auditErr)
		}
	}
	return DatabaseError(e)
}
func difference(before, after any) (map[string]any, error) {
	a, e := safeObject(before)
	if e != nil {
		return nil, e
	}
	b, e := safeObject(after)
	if e != nil {
		return nil, e
	}
	out := map[string]any{}
	keys := map[string]bool{}
	for k := range a {
		keys[k] = true
	}
	for k := range b {
		keys[k] = true
	}
	for k := range keys {
		x, _ := json.Marshal(a[k])
		y, _ := json.Marshal(b[k])
		if string(x) != string(y) {
			out[k] = map[string]any{"old": a[k], "new": b[k]}
		}
	}
	return out, nil
}
func safeObject(v any) (map[string]any, error) {
	if v == nil {
		return map[string]any{}, nil
	}
	data, e := json.Marshal(v)
	if e != nil {
		return nil, e
	}
	var out map[string]any
	if e = json.Unmarshal(data, &out); e != nil {
		return nil, e
	}
	if items, ok := out["items"].([]any); ok {
		delete(out, "items")
		for _, v := range items {
			if item, ok := v.(map[string]any); ok {
				if id, ok := item["mod_id"].(string); ok {
					out["items."+id] = item
				}
			}
		}
	}
	sanitize(out)
	return out, nil
}
func sanitize(v any) {
	switch x := v.(type) {
	case map[string]any:
		for k, value := range x {
			lower := strings.ToLower(k)
			if strings.Contains(lower, "password") || strings.Contains(lower, "token") || strings.Contains(lower, "secret") {
				delete(x, k)
				continue
			}
			if lower == "url" {
				if raw, ok := value.(string); ok {
					u, e := url.Parse(raw)
					if e == nil {
						u.RawQuery = ""
						u.User = nil
						u.Fragment = ""
						x[k] = u.String()
					}
				}
			}
			sanitize(x[k])
		}
	case []any:
		for _, value := range x {
			sanitize(value)
		}
	}
}

// RecordRejected 补记在业务事务之前被参数或认证校验拒绝的写请求
func (s *Service) RecordRejected(ctx context.Context, a Actor, action, code string) error {
	var count int64
	if e := s.DB.WithContext(context.WithoutCancel(ctx)).Model(&infrastructure.Audit{}).Where("request_id = ?", a.RequestID).Count(&count).Error; e != nil {
		return e
	}
	if count > 0 {
		return nil
	}
	return s.DB.WithContext(context.WithoutCancel(ctx)).Create(&infrastructure.Audit{ID: uuid.NewString(), OccurredAt: time.Now().UTC().Format(time.RFC3339Nano), ActorID: a.ID, ActorName: a.Name, Action: action, Result: "failed", ResourceType: "request", RequestID: a.RequestID, ClientIP: a.IP, Changes: map[string]any{}, Summary: "请求未提交", ErrorCode: code}).Error
}
