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

package http

import (
	"flag"
	"fmt"
	"time"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/core"
)

var Config core.Config

func init() {
	flag.StringVar(&Config.Port, "corePort", "80", "CoreServer port.")
}

const (
	DefaultReadTimeout  = 5 * time.Second
	DefaultWriteTimeout = 10 * time.Second
)

type HttpServer interface {
	// BuildWithConfig(conf HttpConfig) HttpServer
	Build() HttpServer
	Run() error
}

type HttpConfig struct {
	Addr string
	// Handler      http.Handler
	ReadTimeout  time.Duration
	WriteTimeout time.Duration
}

func NewHttpConfig() *HttpConfig {
	return &HttpConfig{}
}

func (c *HttpConfig) SetAddr(port string) *HttpConfig {
	c.Addr = fmt.Sprintf(":%s", port)
	return c
}

// type HttpBuilder interface {
// 	Build() HttpServer
// 	NewHttpServer(port string, handler http.Handler) HttpBuilder
// }

// type Builder struct{}

// func (b *Builder) NewHttpServer(port string, handler http.Handler) *http.Server {
// 	return &http.Server{
// 		Addr:         fmt.Sprintf(":%s", port),
// 		Handler:      handler,
// 		ReadTimeout:  DefaultReadTimeout,
// 		WriteTimeout: DefaultWriteTimeout,
// 	}
// }
