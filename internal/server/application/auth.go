package application

import (
	"context"
	"github.com/golang-jwt/jwt/v5"
	"golang.org/x/crypto/bcrypt"
	"gorm.io/gorm"
	"packy/internal/server/infrastructure"
	"strconv"
	"time"
)

type Claims struct {
	TokenVersion int64 `json:"token_version"`
	jwt.RegisteredClaims
}

func (s *Service) Login(ctx context.Context, a Actor, username, password string) (string, error) {
	var admin infrastructure.Administrator
	lookup := s.DB.WithContext(ctx).First(&admin, 1).Error
	authenticated := lookup == nil && len([]byte(password)) <= 72 && username == admin.Username && bcrypt.CompareHashAndPassword([]byte(admin.PasswordHash), []byte(password)) == nil
	if authenticated {
		a.ID = &admin.ID
		a.Name = admin.Username
	}
	var token string
	e := s.Change(ctx, a, "auth.login", "administrator", "1", nil, func(tx *gorm.DB) (any, any, error) {
		if !authenticated {
			return nil, nil, Failure("UNAUTHENTICATED", "账号或密码错误")
		}
		var current infrastructure.Administrator
		if e := tx.First(&current, 1).Error; e != nil {
			return nil, nil, e
		}
		if current.PasswordHash != admin.PasswordHash || current.TokenVersion != admin.TokenVersion {
			return nil, nil, Failure("UNAUTHENTICATED", "账号状态已变化，请重新登录")
		}

		now := time.Now()
		claims := Claims{TokenVersion: admin.TokenVersion, RegisteredClaims: jwt.RegisteredClaims{Subject: strconv.Itoa(admin.ID), Issuer: "packy", Audience: jwt.ClaimStrings{"packy-admin"}, IssuedAt: jwt.NewNumericDate(now), ExpiresAt: jwt.NewNumericDate(now.Add(30 * time.Minute))}}
		var e error
		token, e = jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString([]byte(s.Config.JWTSecret))
		return nil, map[string]any{"authenticated": true}, e
	})
	return token, e
}
func (s *Service) Authenticate(ctx context.Context, token string) (infrastructure.Administrator, error) {
	var a infrastructure.Administrator
	c := &Claims{}
	_, e := jwt.ParseWithClaims(token, c, func(*jwt.Token) (any, error) { return []byte(s.Config.JWTSecret), nil }, jwt.WithValidMethods([]string{"HS256"}), jwt.WithIssuer("packy"), jwt.WithAudience("packy-admin"), jwt.WithExpirationRequired(), jwt.WithIssuedAt())
	if e != nil || c.Subject != "1" || c.IssuedAt == nil {
		return a, Failure("UNAUTHENTICATED", "登录已失效")
	}
	if e = s.DB.WithContext(ctx).First(&a, 1).Error; e != nil || a.TokenVersion != c.TokenVersion {
		return a, Failure("UNAUTHENTICATED", "登录已失效")
	}
	return a, nil
}
func (s *Service) Logout(ctx context.Context, a Actor) error {
	return s.Change(ctx, a, "auth.logout", "administrator", "1", nil, func(tx *gorm.DB) (any, any, error) {
		return nil, map[string]any{"logged_out": true}, tx.Model(&infrastructure.Administrator{}).Where("id = ?", 1).UpdateColumn("token_version", gorm.Expr("token_version + 1")).Error
	})
}
