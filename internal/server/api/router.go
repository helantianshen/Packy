package api

import (
	"encoding/json"
	"errors"
	"github.com/gin-gonic/gin"
	"github.com/go-resty/resty/v2"
	"github.com/google/uuid"
	"go.uber.org/zap"
	"golang.org/x/time/rate"
	"io"
	"net/http"
	"os"
	"packy/internal/server/application"
	"packy/internal/server/infrastructure"
	"packy/pkg/protocol"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"time"
)

type Router struct {
	Service *application.Service
	Log     *zap.Logger
}

func (r *Router) actor(c *gin.Context) application.Actor {
	a := application.Actor{RequestID: c.GetString("request_id"), IP: c.ClientIP()}
	if admin, ok := c.Get("admin"); ok {
		x := admin.(infrastructure.Administrator)
		a.ID = &x.ID
		a.Name = x.Username
	}
	return a
}
func (r *Router) fail(c *gin.Context, e error) {
	e = application.DatabaseError(e)
	api := &protocol.APIError{Code: "INTERNAL_ERROR", Message: "服务处理失败"}
	if !errors.As(e, &api) {
		r.Log.Error("请求处理失败", zap.String("request_id", c.GetString("request_id")), zap.Error(e))
		api = &protocol.APIError{Code: "INTERNAL_ERROR", Message: "服务处理失败"}
	}
	copy := *api
	copy.RequestID = c.GetString("request_id")
	status := map[string]int{"INVALID_REQUEST": 400, "UNAUTHENTICATED": 401, "FORBIDDEN": 403, "NOT_FOUND": 404, "CONFLICT": 409, "NO_PUBLISHED_RELEASE": 409, "SOURCE_UNAVAILABLE": 503, "UPSTREAM_UNAVAILABLE": 502, "RATE_LIMITED": 429}[copy.Code]
	if status == 0 {
		status = 500
	}
	route := c.FullPath()
	mutation := c.Request.Method == "POST" || c.Request.Method == "PUT" || c.Request.Method == "PATCH"
	managed := strings.HasPrefix(route, "/api/v1/admin/") || route == "/api/v1/auth/login" || route == "/api/v1/auth/logout"
	if mutation && managed && !strings.Contains(route, "/modrinth/") && !strings.HasSuffix(route, "/preview") {
		if auditErr := r.Service.RecordRejected(c.Request.Context(), r.actor(c), c.Request.Method+" "+route, copy.Code); auditErr != nil {
			r.Log.Error("拒绝请求审计失败", zap.String("request_id", copy.RequestID), zap.Error(auditErr))
		}
	}
	c.AbortWithStatusJSON(status, copy)
}
func (r *Router) bind(c *gin.Context, out any) bool {
	data, e := io.ReadAll(io.LimitReader(c.Request.Body, 1024*1024+1))
	if e != nil || len(data) > 1024*1024 {
		r.fail(c, protocol.Invalid("请求过大或无法读取"))
		return false
	}
	if e = protocol.Decode(data, out); e != nil {
		r.fail(c, e)
		return false
	}
	return true
}
func (r *Router) respond(c *gin.Context, status int, value any, e error) {
	if e != nil {
		r.fail(c, e)
		return
	}
	c.JSON(status, value)
}
func (r *Router) Handler() *gin.Engine {
	gin.SetMode(gin.ReleaseMode)
	g := gin.New()
	g.RedirectTrailingSlash = false
	g.Use(gin.Recovery())
	g.Use(func(c *gin.Context) {
		c.Set("request_id", uuid.NewString())
		c.Header("X-Request-ID", c.GetString("request_id"))
		c.Next()
	})
	_ = g.SetTrustedProxies(nil)
	s := r.Service
	g.GET("/healthz", func(c *gin.Context) { c.JSON(200, gin.H{"status": "ok", "version": "0.2.0", "phase": "go"}) })
	v := g.Group("/api/v1")
	v.POST("/packs/:pack/update-plan", func(c *gin.Context) {
		var in struct {
			Installed string `json:"installed_release_id"`
			Version   string `json:"client_version"`
		}
		if !r.bind(c, &in) {
			return
		}
		value, e := s.Plan(c.Request.Context(), c.Param("pack"), in.Installed)
		r.respond(c, 200, value, e)
	})
	v.GET("/files/:id/download", r.download)
	v.HEAD("/files/:id/download", r.download)
	limiter := rate.NewLimiter(rate.Every(time.Second), 5)
	v.POST("/auth/login", func(c *gin.Context) {
		if !limiter.Allow() {
			r.fail(c, application.Failure("RATE_LIMITED", "请求过于频繁"))
			return
		}
		var in struct {
			Username string `json:"username"`
			Password string `json:"password"`
		}
		if !r.bind(c, &in) {
			return
		}
		token, e := s.Login(c.Request.Context(), r.actor(c), in.Username, in.Password)
		r.respond(c, 200, gin.H{"access_token": token, "token_type": "Bearer", "expires_in": 1800}, e)
	})
	authorized := v.Group("")
	authorized.Use(func(c *gin.Context) {
		header := c.GetHeader("Authorization")
		if !strings.HasPrefix(header, "Bearer ") {
			r.fail(c, application.Failure("UNAUTHENTICATED", "请先登录"))
			return
		}
		admin, e := s.Authenticate(c.Request.Context(), strings.TrimPrefix(header, "Bearer "))
		if e != nil {
			r.fail(c, e)
			return
		}
		c.Set("admin", admin)
		c.Next()
	})
	authorized.GET("/auth/me", func(c *gin.Context) { a, _ := c.Get("admin"); c.JSON(200, a) })
	authorized.POST("/auth/logout", func(c *gin.Context) {
		if e := s.Logout(c.Request.Context(), r.actor(c)); e != nil {
			r.fail(c, e)
			return
		}
		c.Status(204)
	})
	admin := authorized.Group("/admin")
	admin.GET("/packs", r.list("packs"))
	admin.GET("/mods", r.list("mods"))
	admin.GET("/artifacts", r.list("artifacts"))
	admin.GET("/audit-logs", r.list("audit"))
	admin.GET("/packs/:pack/releases", r.list("releases"))
	admin.GET("/audit-logs/:id", func(c *gin.Context) {
		a, e := s.Audit(c.Request.Context(), c.Param("id"))
		r.respond(c, 200, a, e)
	})
	admin.GET("/packs/:pack", func(c *gin.Context) {
		p, e := s.Pack(c.Request.Context(), c.Param("pack"))
		r.respond(c, 200, p, e)
	})
	admin.GET("/packs/:pack/releases/:id", func(c *gin.Context) {
		release, e := s.Release(c.Request.Context(), c.Param("pack"), c.Param("id"))
		r.respond(c, 200, release, e)
	})
	admin.GET("/packs/:pack/releases/:id/diff", func(c *gin.Context) {
		v, e := s.ReleaseDiff(c.Request.Context(), c.Param("pack"), c.Param("id"))
		r.respond(c, 200, v, e)
	})
	savePack := func(c *gin.Context) {
		var in application.PackInput
		if !r.bind(c, &in) {
			return
		}
		v, e := s.SavePack(c.Request.Context(), r.actor(c), c.Param("pack"), in)
		status := 200
		if c.Param("pack") == "" {
			status = 201
		}
		r.respond(c, status, v, e)
	}
	admin.POST("/packs", savePack)
	admin.PUT("/packs/:pack", savePack)
	saveMod := func(c *gin.Context) {
		var in application.ModInput
		if !r.bind(c, &in) {
			return
		}
		v, e := s.SaveMod(c.Request.Context(), r.actor(c), c.Param("id"), in)
		status := 200
		if c.Param("id") == "" {
			status = 201
		}
		r.respond(c, status, v, e)
	}
	admin.POST("/mods", saveMod)
	admin.PATCH("/mods/:id", saveMod)
	admin.POST("/artifacts", func(c *gin.Context) {
		var in application.ArtifactInput
		if !r.bind(c, &in) {
			return
		}
		v, e := s.CreateArtifact(c.Request.Context(), r.actor(c), in)
		r.respond(c, 201, v, e)
	})
	admin.GET("/system/download-source-types", func(c *gin.Context) {
		v, e := s.SourceTypes(c.Request.Context())
		r.respond(c, 200, v, e)
	})
	admin.PUT("/system/download-source-types/:code", func(c *gin.Context) {
		var in struct {
			Name      string `json:"name"`
			Enabled   bool   `json:"enabled"`
			SortOrder int    `json:"sort_order"`
		}
		if !r.bind(c, &in) {
			return
		}
		v, e := s.SaveSource(c.Request.Context(), r.actor(c), c.Param("code"), infrastructure.SourceType{Name: in.Name, Enabled: in.Enabled, SortOrder: in.SortOrder})
		r.respond(c, 200, v, e)
	})
	admin.POST("/packs/:pack/drafts", func(c *gin.Context) {
		var in application.CreateDraft
		if !r.bind(c, &in) {
			return
		}
		v, e := s.CreateDraft(c.Request.Context(), r.actor(c), c.Param("pack"), in)
		r.respond(c, 201, v, e)
	})
	admin.PUT("/packs/:pack/drafts/:id", func(c *gin.Context) {
		var in application.UpdateDraft
		if !r.bind(c, &in) {
			return
		}
		v, e := s.UpdateDraft(c.Request.Context(), r.actor(c), c.Param("pack"), c.Param("id"), in)
		r.respond(c, 200, v, e)
	})
	for _, action := range []string{"preview", "publish"} {
		action := action
		admin.POST("/packs/:pack/drafts/:id/"+action, func(c *gin.Context) {
			var in struct {
				Revision int64 `json:"revision"`
			}
			if !r.bind(c, &in) {
				return
			}
			if action == "preview" {
				v, e := s.Preview(c.Request.Context(), c.Param("pack"), c.Param("id"), in.Revision)
				r.respond(c, 200, v, e)
			} else {
				v, e := s.Publish(c.Request.Context(), r.actor(c), c.Param("pack"), c.Param("id"), in.Revision)
				r.respond(c, 200, v, e)
			}
		})
	}
	admin.POST("/files", func(c *gin.Context) {
		c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, application.UploadLimit+1024*1024)
		reader, e := c.Request.MultipartReader()
		if e != nil {
			r.fail(c, protocol.Invalid("上传格式错误"))
			return
		}
		part, e := reader.NextPart()
		if e != nil || part.FormName() != "file" {
			r.fail(c, protocol.Invalid("缺少 file 字段"))
			return
		}
		defer part.Close()
		v, e := s.Upload(c.Request.Context(), r.actor(c), part.FileName(), part)
		r.respond(c, 201, v, e)
	})
	r.modrinth(admin)
	g.NoRoute(func(c *gin.Context) {
		r.fail(c, application.Failure("NOT_FOUND", "接口不存在"))
	})
	return g
}
func (r *Router) list(kind string) gin.HandlerFunc {
	return func(c *gin.Context) {
		page, _ := strconv.Atoi(c.DefaultQuery("page", "1"))
		size, _ := strconv.Atoi(c.DefaultQuery("page_size", "20"))
		value, e := r.Service.List(c.Request.Context(), kind, c.Param("pack"), page, size, c.Request.URL.Query())
		r.respond(c, 200, value, e)
	}
}
func (r *Router) download(c *gin.Context) {
	if !protocol.ID(c.Param("id")) {
		r.fail(c, protocol.Invalid("文件 ID 无效"))
		return
	}
	f, e := r.Service.HostedFile(c.Request.Context(), c.Param("id"))
	if e != nil {
		r.fail(c, e)
		return
	}
	file, e := os.Open(filepath.Join(r.Service.Config.FilesDir, f.ID+".jar"))
	if e != nil {
		r.fail(c, e)
		return
	}
	defer file.Close()
	info, e := file.Stat()
	if e != nil {
		r.fail(c, e)
		return
	}
	c.Header("Content-Type", "application/java-archive")
	http.ServeContent(c.Writer, c.Request, f.Filename, info.ModTime(), file)
}
func (r *Router) modrinth(admin *gin.RouterGroup) {
	client := resty.New().SetResponseBodyLimit(8*1024*1024).SetTimeout(30*time.Second).SetHeader("User-Agent", r.Service.Config.ModrinthUserAgent)
	for route, upstream := range map[string]string{"/search": "/search", "/projects/:id": "/project/", "/projects/:id/versions": "/project/", "/versions/:id": "/version/"} {
		route, upstream := route, upstream
		admin.GET("/modrinth"+route, func(c *gin.Context) {
			target := upstream
			if id := c.Param("id"); id != "" {
				if !regexp.MustCompile(`^[A-Za-z0-9_-]+$`).MatchString(id) {
					r.fail(c, protocol.Invalid("项目 ID 无效"))
					return
				}
				target += id
			}
			if strings.HasSuffix(route, "/versions") {
				target += "/version"
			}
			request := client.R().SetContext(c.Request.Context())
			for _, key := range []string{"query", "offset", "limit", "facets", "game_versions", "loaders"} {
				if value := c.Query(key); value != "" {
					request.SetQueryParam(key, value)
				}
			}
			if route == "/search" {
				limit, e := strconv.Atoi(c.DefaultQuery("limit", "20"))
				offset, x := strconv.Atoi(c.DefaultQuery("offset", "0"))
				if e != nil || x != nil || limit < 1 || limit > 100 || offset < 0 {
					r.fail(c, protocol.Invalid("搜索分页参数无效"))
					return
				}
				request.SetQueryParam("limit", strconv.Itoa(limit))
				request.SetQueryParam("offset", strconv.Itoa(offset))
				if c.Query("facets") == "" {
					facets := [][]string{{"project_type:mod"}}
					if v := c.Query("minecraft_version"); v != "" {
						facets = append(facets, []string{"versions:" + v})
					}
					if v := c.Query("loader"); v != "" {
						facets = append(facets, []string{"categories:" + v})
					}
					data, _ := json.Marshal(facets)
					request.SetQueryParam("facets", string(data))
				}
			}
			for key, param := range map[string]string{"minecraft_version": "game_versions", "loader": "loaders"} {
				if value := c.Query(key); value != "" && strings.HasSuffix(route, "/versions") {
					data, _ := json.Marshal([]string{value})
					request.SetQueryParam(param, string(data))
				}
			}
			response, e := request.Get("https://api.modrinth.com/v2" + target)
			if e != nil || response.StatusCode() != 200 {
				r.fail(c, application.Failure("UPSTREAM_UNAVAILABLE", "Modrinth 暂不可用"))
				return
			}
			c.Data(200, "application/json", response.Body())
		})
	}
	admin.POST("/modrinth/version-files", func(c *gin.Context) {
		var in struct {
			Hashes    []string `json:"hashes"`
			Algorithm string   `json:"algorithm"`
		}
		if !r.bind(c, &in) {
			return
		}
		if in.Algorithm != "sha512" || len(in.Hashes) == 0 || len(in.Hashes) > 100 {
			r.fail(c, protocol.Invalid("哈希参数无效"))
			return
		}
		for _, hash := range in.Hashes {
			if !protocol.Hash(hash) {
				r.fail(c, protocol.Invalid("哈希格式错误"))
				return
			}
		}
		response, e := client.R().SetContext(c.Request.Context()).SetBody(in).Post("https://api.modrinth.com/v2/version_files")
		if e == nil && response.StatusCode() == 429 {
			r.fail(c, application.Failure("RATE_LIMITED", "Modrinth 请求过于频繁"))
			return
		}
		if e != nil || response.StatusCode() != 200 {
			r.fail(c, application.Failure("UPSTREAM_UNAVAILABLE", "Modrinth 暂不可用"))
			return
		}
		c.Data(200, "application/json", response.Body())
	})
}
