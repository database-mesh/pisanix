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

	"github.com/database-mesh/pisanix/pisa-controller/cmd/app"

	"golang.org/x/sync/errgroup"
)

var (
	eg errgroup.Group
)

func main() {
	wsc := &app.WebhookServerConfig{}

	flag.StringVar(&wsc.Port, "webhookPort", "6443", "Webhook wsc port.")
	flag.StringVar(&wsc.TLSCertFile, "webhookTLSCertFile", "/etc/webhook/certs/cert.pem", "File containing the x509 Certificate for HTTPS.")
	flag.StringVar(&wsc.TLSKeyFile, "webhookTLSKeyFile", "/etc/webhook/certs/key.pem", "File containing the x509 private key to --tlsCertFile.")
	flag.Parse()

	webhook := app.Webhook()
	config := app.Config()
	healthz := app.Healthz()

	eg.Go(
		func() error {
			return webhook.RunTLS(fmt.Sprintf(":%s", wsc.Port), wsc.TLSCertFile, wsc.TLSKeyFile)
		})

	eg.Go(
		func() error {
			return config.Run(":8080")
		})

	eg.Go(
		func() error {
			return healthz.Run(":80")
		})

	if err := eg.Wait(); err != nil {
		panic(err)
	}
}
