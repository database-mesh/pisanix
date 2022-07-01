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

	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/factory"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/http"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/proxy"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/webhook"
	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"

	log "github.com/sirupsen/logrus"
	"golang.org/x/sync/errgroup"
)

var (
	eg        errgroup.Group
	version   string
	gitcommit string
	branch    string
)

func init() {
	setVersion()
	log.Infof("version: %s,gitcommit: %s,branch: %s", version, gitcommit, branch)
}
func main() {
	flag.Parse()
	kubernetes.GetClient()

	proxyConf := http.NewHttpConfig().SetAddr(proxy.Config.Port)
	webhookConf := http.NewHttpConfig().SetAddr(webhook.Config.Port)

	f := factory.PisaFactory{}
	proxy := f.NewHttpServer(factory.ServerKindProxy, proxyConf, handler)
	eg.Go(
		proxy.Run,
	)
	webhook := f.NewHttpServer(factory.ServerKindWebhook, webhookConf, handler)
	eg.Go(
		webhook.Run,
	)

	if err := eg.Wait(); err != nil {
		log.Fatal(err)
	}
}

func setVersion() {
	if version == "" {
		version = branch + "-" + gitcommit
	}
	if branch == "" {
		branch = version
	}
}
