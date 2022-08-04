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
)

type Proxy struct {
	BackendType string `json:"backend_type"`
	DB          string `json:"db"`
	ListenAddr  string `json:"listen_addr"`
	Name        string `json:"name"`
	Password    string `json:"password"`
	// TODO: should be refactored to a new pool configuration
	PoolSize           uint32              `json:"pool_size,omitempty"`
	User               string              `json:"user"`
	SimpleLoadBalance  *SimpleLoadBalance  `json:"simple_loadbalance,omitempty"`
	ReadWriteSplitting *ReadWriteSplitting `json:"read_write_splitting,omitempty"`
	Plugin             *Plugin             `json:"plugin,omitempty"`
	ServerVersion      string              `json:"server_version,omitempty"`
}

type Plugin struct {
	CircuitBreaks       []CircuitBreak       `json:"circuit_break,omitempty"`
	ConcurrencyControls []ConcurrencyControl `json:"concurrency_control,omitempty"`
}

type SimpleLoadBalance struct {
	BalancerType string   `json:"balance_type"`
	Nodes        []string `json:"nodes"`
}

type ReadWriteSplitting struct {
	Static  *ReadWriteSplittingStatic  `json:"static,omitempty"`
	Dynamic *ReadWriteSplittingDynamic `json:"dynamic,omitempty"`
}

type ReadWriteSplittingStatic struct {
	DefaultTarget string                   `json:"default_target"`
	Rules         []ReadWriteSplittingRule `json:"rule"`
}

type ReadWriteSplittingRule struct {
	Name          string   `json:"name"`
	Type          string   `json:"type"`
	Regex         []string `json:"regex"`
	Target        string   `json:"target"`
	AlgorithmName string   `json:"algorithm_name"`
}

type ReadWriteSplittingDynamic struct {
	DefaultTarget string                   `json:"default_target"`
	Rules         []ReadWriteSplittingRule `json:"rule"`
	Discovery     ReadWriteDiscovery       `json:"discovery"`
}

type ReadWriteDiscovery struct {
	*MasterHighAvailablity
}

type MasterHighAvailablity struct {
	Type                      string `json:"type"`
	User                      string `json:"user"`
	Password                  string `json:"password"`
	MonitorInterval           uint64 `json:"monitor_period"`
	ConnectInterval           uint64 `json:"connect_period"`
	ConnectTimeout            uint64 `json:"connect_timeout"`
	ConnectMaxFailures        uint64 `json:"connect_failure_threshold"`
	PingInterval              uint64 `json:"ping_period"`
	PingTimeout               uint64 `json:"ping_timeout"`
	PingMaxFailures           uint64 `json:"ping_failure_threshold"`
	ReplicationLagInterval    uint64 `json:"replication_lag_period"`
	ReplicationLagTimeout     uint64 `json:"replication_lag_timeout"`
	ReplicationLagMaxFailures uint64 `json:"replication_lag_failure_threshold"`
	MaxReplicationLag         uint64 `json:"max_replication_lag"`
	ReadOnlyInterval          uint64 `json:"read_only_period"`
	ReadOnlyTimeout           uint64 `json:"read_only_timeout"`
	ReadOnlyMaxFailures       uint64 `json:"read_only_failure_threshold"`
}

// CircuitBreak works with regular expressions.
// SQL statements that conform to regular expressions will be denied.
type CircuitBreak struct {
	Regex []string `json:"regex"`
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
