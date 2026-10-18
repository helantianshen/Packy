package main

import (
	"bufio"
	"context"
	"fmt"
	"github.com/spf13/cobra"
	"go.uber.org/zap"
	"golang.org/x/crypto/bcrypt"
	"golang.org/x/term"
	"io"
	"net/http"
	"os"
	"os/signal"
	"packy/internal/server/api"
	"packy/internal/server/application"
	"packy/internal/server/infrastructure"
	"strings"
	"syscall"
	"time"
)

func main() {
	if e := command().Execute(); e != nil {
		fmt.Fprintln(os.Stderr, e)
		os.Exit(1)
	}
}
func command() *cobra.Command {
	root := &cobra.Command{Use: "packy-server", SilenceUsage: true, SilenceErrors: true, RunE: func(*cobra.Command, []string) error {
		c, e := infrastructure.LoadConfig(true)
		if e != nil {
			return e
		}
		db, e := infrastructure.OpenDatabase(c.Database, false)
		if e != nil {
			return e
		}
		pool, e := db.DB()
		if e != nil {
			return e
		}
		defer pool.Close()
		if e = infrastructure.Ready(db); e != nil {
			return e
		}
		logConfig := zap.NewProductionConfig()
		if e := logConfig.Level.UnmarshalText([]byte(c.LogFilter)); e != nil {
			return fmt.Errorf("log_filter 日志级别无效")
		}
		log, e := logConfig.Build()
		if e != nil {
			return e
		}
		defer log.Sync()
		handler := (&api.Router{Service: &application.Service{DB: db, Config: c}, Log: log}).Handler()
		server := &http.Server{Addr: c.Listen, Handler: handler, ReadHeaderTimeout: 10 * time.Second, IdleTimeout: 60 * time.Second}
		ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
		defer stop()
		shutdownDone := make(chan struct{})
		go func() {
			defer close(shutdownDone)
			<-ctx.Done()
			shutdown, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			defer cancel()
			if e := server.Shutdown(shutdown); e != nil {
				log.Error("服务退出失败", zap.Error(e))
			}
		}()
		e = server.ListenAndServe()
		if e == http.ErrServerClosed {
			<-shutdownDone
			return nil
		}
		return e
	}}
	root.AddCommand(&cobra.Command{Use: "migrate", RunE: func(*cobra.Command, []string) error {
		c, e := infrastructure.LoadConfig(false)
		if e != nil {
			return e
		}
		db, e := infrastructure.OpenDatabase(c.Database, true)
		if e != nil {
			return e
		}
		pool, e := db.DB()
		if e != nil {
			return e
		}
		defer pool.Close()
		return infrastructure.Migrate(db)
	}})
	root.AddCommand(&cobra.Command{Use: "password-hash", RunE: func(*cobra.Command, []string) error {
		var data []byte
		var e error
		if term.IsTerminal(int(os.Stdin.Fd())) {
			fmt.Fprint(os.Stderr, "密码: ")
			data, e = term.ReadPassword(int(os.Stdin.Fd()))
			fmt.Fprintln(os.Stderr)
		} else {
			var line string
			line, e = bufio.NewReader(os.Stdin).ReadString('\n')
			data = []byte(strings.TrimSuffix(strings.TrimSuffix(line, "\n"), "\r"))
			if e == io.EOF {
				e = nil
			}
		}
		if e != nil {
			return e
		}
		if len(data) < 8 || len(data) > 72 {
			return fmt.Errorf("密码需要 8–72 UTF-8 字节")
		}
		hash, e := bcrypt.GenerateFromPassword(data, 12)
		if e != nil {
			return e
		}
		fmt.Println(string(hash))
		return nil
	}})
	return root
}
