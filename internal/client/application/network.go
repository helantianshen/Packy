package application

import (
	"context"
	"net/http"
	"net/url"
	"packy/config"
	"time"

	"github.com/go-resty/resty/v2"
	"golang.org/x/time/rate"
)

type networkKey struct{}
type settingsKey struct{}

func (c *Client) frozenSettings(ctx context.Context) config.Download {
	if d, ok := ctx.Value(settingsKey{}).(config.Download); ok {
		return d
	}
	return c.downloadSettings()
}

type downloadNetwork struct {
	settings config.Download
	clients  map[string]*resty.Client
	limiter  *rate.Limiter
}

func newDownloadNetwork(d config.Download) *downloadNetwork {
	n := &downloadNetwork{settings: d, clients: map[string]*resty.Client{}}
	if d.LimitKiB > 0 {
		n.limiter = rate.NewLimiter(rate.Limit(d.LimitKiB)*1024, 64*1024)
	}
	for _, source := range []string{"modrinth", "curseforge", "github", "server"} {
		c := httpClient().SetTimeout(time.Duration(d.DownloadTimeout) * time.Second)
		// 显式设置代理函数，空值不继承宿主环境的代理配置
		transport := http.DefaultTransport.(*http.Transport).Clone()
		transport.Proxy = nil
		if address := d.Proxies[source]; address != "" {
			u, _ := url.Parse(address)
			transport.Proxy = http.ProxyURL(u)
		}
		c.SetTransport(transport)
		n.clients[source] = c
	}
	return n
}
func (n *downloadNetwork) close() {
	for _, c := range n.clients {
		c.GetClient().CloseIdleConnections()
	}
}
func (c *Client) network(ctx context.Context) (*downloadNetwork, func()) {
	if n, ok := ctx.Value(networkKey{}).(*downloadNetwork); ok {
		return n, func() {}
	}
	n := newDownloadNetwork(c.frozenSettings(ctx))
	return n, n.close
}
