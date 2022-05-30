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

package main

import (
	"flag"
	"fmt"
	"net/http"
	"time"

	"github.com/database-mesh/pisanix/pisa-controller/cmd/core"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/proxy"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/webhook"

	log "github.com/sirupsen/logrus"
	"golang.org/x/sync/errgroup"
)

var (
	eg errgroup.Group
)

const (
	DEFAULT_READ_TIMEOUT  = 5 * time.Second
	DEFAULT_WRITE_TIMEOUT = 10 * time.Second
)

func main() {
	flag.Parse()

	eg.Go(func() error {
		return newHttpServer(
			webhook.Config.Port,
			webhook.Handler(),
		).ListenAndServeTLS(webhook.Config.TLSCertFile, webhook.Config.TLSKeyFile)
	})
	eg.Go(func() error {
		return newHttpServer(
			proxy.Config.Port,
			proxy.Handler(),
		).ListenAndServe()
	})
	eg.Go(func() error {
		return newHttpServer(
			core.Config.Port,
			core.Handler(),
		).ListenAndServe()
	})

	if err := eg.Wait(); err != nil {
		log.Fatal(err)
	}
}

func newHttpServer(port string, handler http.Handler) *http.Server {
	return &http.Server{
		Addr:         fmt.Sprintf(":%s", port),
		Handler:      handler,
		ReadTimeout:  DEFAULT_READ_TIMEOUT,
		WriteTimeout: DEFAULT_WRITE_TIMEOUT,
	}
}
