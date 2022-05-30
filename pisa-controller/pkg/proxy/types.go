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
	"time"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"
)

type PisaProxyConfig struct {
	Admin struct {
		Port     string `json:"port"`
		LogLevel string `json:"log_level"`
	} `json:"admin"`
	Proxies    []Proxy            `json:"proxies"`
	MysqlNodes []kubernetes.MySQL `json:"mysql_nodes"`
}

type Proxy struct {
	BackendType        string             `json:"backend_type"`
	DB                 string             `json:"db"`
	ListenAddr         string             `json:"listen_addr"`
	Name               string             `json:"name"`
	Password           string             `json:"password"`
	PoolSize           int                `json:"pool_size"`
	Username           string             `json:"username"`
	SimpleLoadBalancer SimpleLoadBalancer `json:"simple_loadbalancer"`
	Plugins            Plugin             `json:"plugins"`
}

type Plugin struct {
	CircuitBreaks       []kubernetes.CircuitBreak `json:"circuit_breaks,omitempty"`
	ConcurrencyControls []ConcurrencyControl      `json:"concurrency_controls,omitempty"`
}

type SimpleLoadBalancer struct {
	BalancerType string   `json:"balancer_type"`
	Nodes        []string `json:"nodes"`
}

// ConcurrencyControl The conversion used for json key is defined here
// for kubernetes/types.go  MaxConcurrency key is maxConcurrency.
// We need to convert it to max_concurrency to fit the pisa-proxy's configuration format
// FIXME: A better way to convert
type ConcurrencyControl struct {
	Regex          string        `json:"regex"`
	Duration       time.Duration `json:"duration"`
	MaxConcurrency int           `json:"max_concurrency"`
}
