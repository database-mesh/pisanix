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

type Proxy struct {
	BackendType        string              `json:"backend_type"`
	DB                 string              `json:"db"`
	ListenAddr         string              `json:"listen_addr"`
	Name               string              `json:"name"`
	Password           string              `json:"password"`
	PoolSize           uint32              `json:"pool_size,omitempty"`
	User               string              `json:"user"`
	SimpleLoadBalance  *SimpleLoadBalance  `json:"simple_loadbalance,omitempty"`
	ReadWriteSplitting *ReadWriteSplitting `json:"read_write_splitting,omitempty"`
	Plugin             Plugin              `json:"plugin,omitempty"`
	ServerVersion      string              `json:"server_version,omitempty"`
}

type Plugin struct {
	CircuitBreaks       []kubernetes.CircuitBreak `json:"circuit_break,omitempty"`
	ConcurrencyControls []ConcurrencyControl      `json:"concurrency_control,omitempty"`
}

type SimpleLoadBalance struct {
	BalancerType string   `json:"balance_type"`
	Nodes        []string `json:"nodes"`
}

type ReadWriteSplitting struct {
	Static *ReadWriteSplittingStatic `json:"static,omitempty"`
}

type ReadWriteSplittingStatic struct {
	DefaultTarget string                         `json:"default_target"`
	Rules         []ReadWriteSplittingStaticRule `json:"rule"`
}

type ReadWriteSplittingStaticRule struct {
	Name          string   `json:"name"`
	Type          string   `json:"type"`
	Regex         []string `json:"regex"`
	Target        string   `json:"target"`
	AlgorithmName string   `json:"algorithm_name"`
}

// ConcurrencyControl The conversion used for json key is defined here
// for kubernetes/types.go  MaxConcurrency key is maxConcurrency.
// We need to convert it to max_concurrency to fit the pisa-proxy's configuration format
// FIXME: A better way to convert
type ConcurrencyControl struct {
	Regex          []string      `json:"regex"`
	Duration       time.Duration `json:"duration"`
	MaxConcurrency int           `json:"max_concurrency"`
}

// Node describe mysql node
type MySQLNode struct {
	Name     string `json:"name"`
	Db       string `json:"db"`
	User     string `json:"user"`
	Password string `json:"password"`
	Host     string `json:"host"`
	Port     uint32 `json:"port"`
	Weight   int    `json:"weight"`
	Role     string `json:"role"`
}
