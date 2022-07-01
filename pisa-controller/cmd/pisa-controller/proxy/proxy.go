// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package proxy

import (
	"flag"
	"fmt"
	"net/http"
	"time"

	pisahttp "github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/http"
	"github.com/database-mesh/pisanix/pisa-controller/pkg/proxy"
)

var Config proxy.Config

func init() {
	flag.StringVar(&Config.Port, "proxyConfigsPort", "8080", "ProxyConfigsServer port.")
}

const (
	DefaultReadTimeout  = 5 * time.Second
	DefaultWriteTimeout = 10 * time.Second
)

func newHttpServer(port string, handler http.Handler) *http.Server {
	return &http.Server{
		Addr:         fmt.Sprintf(":%s", port),
		Handler:      handler,
		ReadTimeout:  DefaultReadTimeout,
		WriteTimeout: DefaultWriteTimeout,
	}
}

type ProxyServerBuilder struct {
	Addr         string
	Handler      http.Handler
	ReadTimeout  time.Duration
	WriteTimeout time.Duration
	Buider       pisahttp.Builder
}

func NewProxyServerBuilder() *ProxyServerBuilder {
	return &ProxyServerBuilder{
		ReadTimeout:  DefaultReadTimeout,
		WriteTimeout: DefaultWriteTimeout,
	}
}

// func (b *ProxyServerBuilder) SetPort(port string) *ProxyServerBuilder {
// 	b.Addr = fmt.Sprintf(":%s", port)
// 	return b
// }

// func (b *ProxyServerBuilder) SetHandler(handler http.Handler) *ProxyServerBuilder {
// 	b.Handler = handler
// 	return b
// }

func (b *ProxyServerBuilder) Build() pisahttp.HttpServer {
	hs := newHttpServer(b.Addr, b.Handler)
	return &ProxyServer{
		core: hs,
	}
}

func (b *ProxyServerBuilder) NewHttpServer(port string, handler http.Handler) *http.Server {
	return b.Buider.NewHttpServer(port, handler)
}

type ProxyServer struct {
	core *http.Server
}

func NewProxyServer(conf pisahttp.Config) *ProxyServer {
	hs := newHttpServer(conf.Addr, conf.Handler)
	return &ProxyServer{
		core: hs,
	}
}

func (s *ProxyServer) Run() error {
	return s.core.ListenAndServe()
}

func (s *ProxyServer) WithHandler(handler http.Handler) *ProxyServer {
	s.core.Handler = handler
	return s
}

func (s *ProxyServer) Build() pisahttp.HttpServer {
	return s
}
