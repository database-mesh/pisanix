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

package webhook

import (
	"flag"
	"fmt"
	"net/http"
	"time"

	pisahttp "github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/http"
	"github.com/database-mesh/pisanix/pisa-controller/pkg/webhook"
)

var Config webhook.Config

func init() {
	flag.StringVar(&Config.Port, "webhookPort", "6443", "WebhookServer port.")
	flag.StringVar(&Config.TLSCertFile, "webhookTLSCertFile", "/etc/webhook/certs/tls.crt", "File containing the x509 certificate to --webhookTLSCertFile.")
	flag.StringVar(&Config.TLSKeyFile, "webhookTLSKeyFile", "/etc/webhook/certs/tls.key", "File containing the x509 private key to --webhookTLSKeyFile.")
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

type WebhookServerBuilder struct {
	Addr         string
	Handler      http.Handler
	ReadTimeout  time.Duration
	WriteTimeout time.Duration
}

func NewWebhookServerBuilder() *WebhookServerBuilder {
	return &WebhookServerBuilder{
		ReadTimeout:  DefaultReadTimeout,
		WriteTimeout: DefaultWriteTimeout,
	}
}

// func (b *WebhookServerBuilder) SetPort(port string) *WebhookServerBuilder {
// 	b.Addr = fmt.Sprintf(":%s", port)
// 	return b
// }

// func (b *WebhookServerBuilder) SetHandler(handler http.Handler) *WebhookServerBuilder {
// 	b.Handler = handler
// 	return b
// }

func (b *WebhookServerBuilder) Build() pisahttp.HttpServer {
	hs := newHttpServer(b.Addr, b.Handler)
	return &WebhookServer{
		core: hs,
	}
}

type WebhookServer struct {
	core *http.Server
}

// func NewWebhookServer(port string, handler http.Handler) *WebhookServer {
// 	hs := newHttpServer(port, handler)
// 	return &WebhookServer{
// 		core: hs,
// 	}
// }

func (s WebhookServer) Run() error {
	return s.core.ListenAndServe()
}
