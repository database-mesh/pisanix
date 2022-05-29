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

	"github.com/database-mesh/pisanix/pisa-controller/cmd/app"

	log "github.com/sirupsen/logrus"
	"golang.org/x/sync/errgroup"
)

var (
	g errgroup.Group
)

func main() {
	flag.Parse()

	serverWebhook := &http.Server{
		Addr:         fmt.Sprintf(":%s", app.Webhook.Port),
		Handler:      app.WebhookHandlers(),
		ReadTimeout:  5 * time.Second,
		WriteTimeout: 10 * time.Second,
	}

	serverConfig := &http.Server{
		Addr:         fmt.Sprintf(":%s", app.ProxyConfigs.Port),
		Handler:      app.ProxyConfigsHandlers(),
		ReadTimeout:  5 * time.Second,
		WriteTimeout: 10 * time.Second,
	}
	serverBasic := &http.Server{
		Addr:         fmt.Sprintf(":%s", app.Basic.Port),
		Handler:      app.BasicHandlers(),
		ReadTimeout:  5 * time.Second,
		WriteTimeout: 10 * time.Second,
	}

	g.Go(func() error {
		return serverWebhook.ListenAndServeTLS(app.Webhook.TLSCertFile, app.Webhook.TLSKeyFile)
	})
	g.Go(func() error {
		return serverConfig.ListenAndServe()
	})
	g.Go(func() error {
		return serverBasic.ListenAndServe()
	})

	if err := g.Wait(); err != nil {
		log.Fatal(err)
	}

}
