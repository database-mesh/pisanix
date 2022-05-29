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

package config

import (
	"time"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// VirtualDatabaseSpec defines the desired state of VirtualDatabase
type VirtualDatabaseSpec struct {
	Services []Service `json:"services"`
}

type Service struct {
	DatabaseService `json:",inline"`

	Name            string `json:"name"`
	TrafficStrategy string `json:"trafficStrategy"`
}
type DatabaseService struct {
	DatabaseMySQL *DatabaseMySQL `json:"databaseMySQL"`
}
type DatabaseMySQL struct {
	// +optional
	Host string `json:"host,omitempty"`
	// +optional
	Port string `json:"port,omitempty"`
	// +optional
	Username string `json:"username,omitempty"`
	// +optional
	Password string `json:"password,omitempty"`
	// +optional
	DB string `json:"db,omitempty"`
	// +optional
	PoolSize int `json:"poolSize,omitempty"`
}

// VirtualDatabaseStatus defines the observed state of VirtualDatabase
type VirtualDatabaseStatus struct {
	Endpoints []string `json:"endpoints"`
}

// TrafficStrategySpec defines the desired state of TrafficStrategy
type TrafficStrategySpec struct {
	Selector *metav1.LabelSelector `json:"selector"`
	// +optional
	// 均衡器（默认值）文档标注
	LoadBalance *LoadBalance `json:"loadBalance,omitempty"`
	// +optional
	// 断路器
	CircuitBreaks []CircuitBreak `json:"circuitBreaks,omitempty"`
	// +optional
	// 限流器
	ConcurrencyControls []ConcurrencyControl `json:"ConcurrencyControls,omitempty"`
}

type LoadBalance struct {
	SimpleLoadBalancer *SimpleLoadBalancer `json:"simpleLoadBalancer"`
}

type SimpleLoadBalancer struct {
	Kind string `json:"kind"`
}

type CircuitBreak struct {
	Regex string `json:"regex"`
}

type ConcurrencyControl struct {
	Regex          string        `json:"regex"`
	Duration       time.Duration `json:"duration"`
	MaxConcurrency int           `json:"maxConcurrency"`
}

// DatabaseEndpointSpec defines the desired state of DatabaseEndpoint
type DatabaseEndpointSpec struct {
	Database Database `json:"database"`
}
type Database struct {
	MySQL *MySQL `json:"MySQL"`
}

type MySQL struct {
	Name     string `json:"name,omitempty"`
	Host     string `json:"host"`
	Port     string `json:"port"`
	Username string `json:"username"`
	Password string `json:"password"`
	DB       string `json:"db"`
	Weight   int    `json:"weight"`
}

type PisaProxyConfig struct {
	Admin struct {
		Port     string `json:"port"`
		LogLevel string `json:"log_level"`
	} `json:"admin"`
	Proxies    []ProxySelf `json:"proxies"`
	MysqlNodes []MySQL     `json:"mysql_nodes"`
}

type ProxySelf struct {
	BackendType        string          `json:"backend_type"`
	DB                 string          `json:"db"`
	ListenAddr         string          `json:"listen_addr"`
	Name               string          `json:"name"`
	Password           string          `json:"password"`
	PoolSize           int             `json:"pool_size"`
	Username           string          `json:"username"`
	SimpleLoadBalancer SimpleProxySelf `json:"simple_loadbalancer"`
	Plugins            PluginSelf      `json:"plugins"`
}

type PluginSelf struct {
	CircuitBreaks       []CircuitBreak           `json:"circuit_breaks,omitempty"`
	ConcurrencyControls []ConcurrencyControlSelf `json:"concurrency_controls,omitempty"`
}

type SimpleProxySelf struct {
	BalancerType string   `json:"balancer_type"`
	Nodes        []string `json:"nodes"`
}

type ConcurrencyControlSelf struct {
	Regex          string        `json:"regex"`
	Duration       time.Duration `json:"duration"`
	MaxConcurrency int           `json:"max_concurrency"`
}
