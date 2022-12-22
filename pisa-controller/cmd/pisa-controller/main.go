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
	controllermanager "github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/controller-manager"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/proxy"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/task"
	"github.com/database-mesh/pisanix/pisa-controller/cmd/pisa-controller/webhook"

	_ "k8s.io/client-go/plugin/pkg/client/auth"

	"github.com/mlycore/log"
)

var (
	version   string
	gitcommit string
	branch    string
)

func init() {
	setVersion()
	log.Infof("version: %s, gitcommit: %s, branch: %s", version, gitcommit, branch)
}

func main() {
	taskmgr := task.TaskManager{}

	p := proxy.NewTask()
	w := webhook.NewTask()
	ctrlm := controllermanager.NewTask()

	taskmgr.Register(p).Register(w).Register(ctrlm).Run()
}

func setVersion() {
	if version == "" {
		version = branch + "-" + gitcommit
	}
	if branch == "" {
		branch = version
	}
}
