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
	"os"

	"github.com/database-mesh/pisanix/pisa-daemon/pkg/server"
	"github.com/database-mesh/pisanix/pisa-daemon/pkg/server/config"
	"github.com/database-mesh/pisanix/pisa-daemon/pkg/version"

	"github.com/mlycore/log"
)

const (
	ProjectName = "Pisa-Daemon"
)

var (
	printVersion bool
	conf         = &config.Config{}
)

func init() {
	flag.BoolVar(&printVersion, "version", false, "print version information")
	flag.StringVar(&conf.CRI, "cri", "docker", "cluster runtime")
	flag.Parse()
}

func main() {
	version.PrintVersionInfo(ProjectName)
	if printVersion {
		os.Exit(0)
	}

	s, err := server.New(conf)
	if err != nil {
		log.Fatalf("new server error")
	}

	if err := s.Run(); err != nil {
		log.Fatalf("server run error: %s", err)
	}
}
