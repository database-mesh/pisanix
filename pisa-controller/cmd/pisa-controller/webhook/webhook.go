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
	"time"

	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/http"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/task"
	"github.com/database-mesh/pisanix/pisa-controller/pkg/webhook"
)

const (
	DefaultReadTimeout  = 5 * time.Second
	DefaultWriteTimeout = 10 * time.Second
)

type WebhookServer struct {
	core *http.HttpServer
}

func (s WebhookServer) Run() error {
	return s.core.ListenAndServe()
}

func NewTask() task.Task {
	conf := http.NewHttpConfig().SetAddr(webhook.Conf.Port)
	handler := webhook.Handler()
	hs := http.NewHttpServer(conf).WithHandler(handler).Build()
	w := &WebhookServer{
		core: hs,
	}

	return w
}
