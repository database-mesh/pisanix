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
	"github.com/database-mesh/pisanix/pisa-controller/pkg/config"

	"github.com/gin-gonic/gin"
)

// FIXME(#xxx): need to synchronise the config version with CustomResourceDefinitions
func Config() *gin.Engine {
	r := gin.Default()
	g := r.Group("/apis/proxy.pisanix.io/v1alpha1")
	g.GET("/namespaces/:namespace/proxyconfigs/:appname", config.GetConfig)

	return r
}
