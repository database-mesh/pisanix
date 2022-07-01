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

package factory

import (
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/http"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/proxy"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/webhook"
)

type ServerKind string

const (
	ServerKindProxy   ServerKind = "proxy"
	ServerKindWebhook ServerKind = "webhook"
)

type Factory interface {
	NewHttpServer(kind ServerKind) http.HttpServer
}

type PisaFactory struct{}

func (p PisaFactory) NewHttpServer(kind ServerKind, conf http.HttpConfig, handler http.Handler) http.HttpServer {
	switch kind {
	case ServerKindProxy:
		{
			return proxy.NewProxyServer(conf).WithHandler(handler).Build()
		}
	case ServerKindWebhook:
		{
			return webhook.NewProxyServer(conf).WithHandler(handler).Build()
		}
	}
	return nil
}
