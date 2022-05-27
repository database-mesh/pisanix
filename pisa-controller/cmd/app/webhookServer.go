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

package app

import (
	"flag"
	"net/http"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/webhook"

	"github.com/gin-gonic/gin"
)

var Webhook WebhookConfig

type WebhookConfig struct {
	TLSCertFile string
	TLSKeyFile  string
	Port        string
}

func init() {
	flag.StringVar(&Webhook.Port, "webhookPort", "6443", "WebhookServer port.")
	flag.StringVar(&Webhook.TLSCertFile, "webhookTLSCertFile", "/etc/webhook/certs/tls.crt", "File containing the x509 Certificate to --webhookTLSCertFile.")
	flag.StringVar(&Webhook.TLSKeyFile, "webhookTLSKeyFile", "/etc/webhook/certs/tls.key", "File containing the x509 private key to --webhookTLSKeyFile.")
}

func WebhookHandlers() http.Handler {

	r := gin.New()
	r.Use(gin.Recovery(), gin.Logger())
	g := r.Group("/apis/admission.database-mesh.io/v1alpha1")
	g.GET("", webhook.ApiCheck)
	g.POST("/mutate", webhook.InjectSidecar)

	return r
}
