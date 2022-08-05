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
	"net/http"

	"github.com/gin-gonic/gin"
)

func init() {
	flag.StringVar(&Conf.Port, "webhookPort", "6443", "WebhookServer port.")
	flag.StringVar(&Conf.TLSCertFile, "webhookTLSCertFile", "/etc/webhook/certs/tls.crt", "File containing the x509 certificate to --webhookTLSCertFile.")
	flag.StringVar(&Conf.TLSKeyFile, "webhookTLSKeyFile", "/etc/webhook/certs/tls.key", "File containing the x509 private key to --webhookTLSKeyFile.")
}

type Config struct {
	TLSCertFile string
	TLSKeyFile  string
	Port        string
}

var Conf Config

func Handler() http.Handler {
	r := gin.New()
	r.Use(gin.Recovery(), gin.Logger())
	g := r.Group("/apis/admission.database-mesh.io/v1alpha1")

	// NOTE: there is not API path in this request
	// TODO: test trailing slash
	g.GET("", ApiCheck)
	g.POST("/mutate", Injection)

	return r
}
