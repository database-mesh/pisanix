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
	"net/http"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/config"
	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"

	"github.com/gin-gonic/gin"
)

var Config ProxyConfig

type ProxyConfig struct {
	Port string
}

func init() {
	flag.StringVar(&Config.Port, "proxyConfigsPort", "8080", "ProxyConfigsServer port.")
}

func Handler() http.Handler {
	client, err := kubernetes.NewInClusterClient()
	if err != nil {
		// TODO: add error handling
	}

	r := gin.New()
	r.Use(gin.Recovery(), gin.Logger())
	g := r.Group("/apis/configs.database-mesh.io/v1alpha1")

	g.GET("/namespaces/:namespace/proxyconfigs/:appname", config.GetConfig(client))

	return r
}
