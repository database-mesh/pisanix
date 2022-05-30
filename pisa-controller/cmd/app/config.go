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

	"github.com/database-mesh/pisanix/pisa-controller/pkg/config"

	"github.com/gin-gonic/gin"
	"k8s.io/client-go/dynamic"
	"k8s.io/client-go/rest"
)

var ProxyConfigs ProxyConfigsConfig

type ProxyConfigsConfig struct {
	Port string
}

func init() {
	flag.StringVar(&ProxyConfigs.Port, "proxyConfigsPort", "8080", "ProxyConfigsServer port.")
}
func ProxyConfigsHandler() http.Handler {
	client, _ := initClient()
	r := gin.New()
	r.Use(gin.Recovery(), gin.Logger())
	g := r.Group("/apis/configs.database-mesh.io/v1alpha1")
	g.GET("/namespaces/:namespace/proxyconfigs/:appname", config.GetConfig(client))
	return r
}

func initClient() (dynamic.Interface, error) {
	// creates the in-cluster config
	config, err := rest.InClusterConfig()
	if err != nil {
		return nil, err
	}
	// creates the clientset
	clientset, err := dynamic.NewForConfig(config)
	if err != nil {
		return nil, err
	}

	return clientset, nil
}
