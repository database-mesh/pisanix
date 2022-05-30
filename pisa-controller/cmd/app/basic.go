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

	"github.com/gin-gonic/gin"
)

var Basic BasicConfig

type BasicConfig struct {
	Port string
}

func init() {
	flag.StringVar(&Basic.Port, "basicPort", "80", "BasicServer port.")
}

func BasicHandler() http.Handler {
	r := gin.New()
	r.Use(gin.Recovery(), gin.Logger())

	r.GET("/healthz", func(ctx *gin.Context) {
		ctx.Status(http.StatusOK)
	})
	return r
}
