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
	"fmt"
	"time"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"
)

type PisaProxyConfig struct {
	Admin struct {
		Host     string `json:"host,omitempty"`
		Port     uint32 `json:"port,omitempty"`
		LogLevel string `json:"log_level"`
	} `json:"admin"`
	Mysql struct {
		Nodes []Node `json:"node"`
	} `json:"mysql"`
	Proxy struct {
		Configs []Proxy `json:"config"`
	} `json:"proxy"`
}

// How about the name
type ProxyBuilder struct {
	// VirtualDatabase   kubernetes.VirtualDatabase
	VirtualDatabaseService kubernetes.VirtualDatabaseService
	TrafficStrategy        kubernetes.TrafficStrategy
	DatabaseEndpoints      []kubernetes.DatabaseEndpoint
}

func NewProxyBuilder() *ProxyBuilder {
	return &ProxyBuilder{}
}

func (b *ProxyBuilder) SetVirtualDatabaseService(svc kubernetes.VirtualDatabaseService) *ProxyBuilder {
	b.VirtualDatabaseService = svc
	return b
}

func (b *ProxyBuilder) SetTrafficStrategy(ts kubernetes.TrafficStrategy) *ProxyBuilder {
	b.TrafficStrategy = ts
	return b
}

func (b *ProxyBuilder) SetDatabaseEndpoints(dbeps []kubernetes.DatabaseEndpoint) *ProxyBuilder {
	b.DatabaseEndpoints = dbeps
	return b
}

func (b *ProxyBuilder) Build() *Proxy {
	proxy := &Proxy{}
	proxy.BackendType = "mysql"
	proxy.DB = b.VirtualDatabaseService.DatabaseService.DatabaseMySQL.DB
	proxy.Name = b.VirtualDatabaseService.Name
	proxy.User = b.VirtualDatabaseService.DatabaseService.DatabaseMySQL.User
	proxy.Password = b.VirtualDatabaseService.DatabaseService.DatabaseMySQL.Password
	proxy.PoolSize = b.VirtualDatabaseService.DatabaseService.DatabaseMySQL.PoolSize
	if b.VirtualDatabaseService.DatabaseMySQL.Host == "" {
		b.VirtualDatabaseService.DatabaseMySQL.Host = "0.0.0.0"
	}
	if b.VirtualDatabaseService.DatabaseMySQL.Port == 0 {
		b.VirtualDatabaseService.DatabaseMySQL.Port = 3306
	}
	proxy.ListenAddr = fmt.Sprintf("%s:%d", b.VirtualDatabaseService.DatabaseService.DatabaseMySQL.Host, b.VirtualDatabaseService.DatabaseService.DatabaseMySQL.Port)
	proxy.ServerVersion = b.VirtualDatabaseService.DatabaseService.DatabaseMySQL.ServerVersion

	switch {
	case b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting != nil:
		{
			proxy.ReadWriteSplitting = &ReadWriteSplitting{
				Static: &ReadWriteSplittingStatic{},
			}
			if b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static != nil {
				proxy.ReadWriteSplitting.Static.DefaultTarget = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.DefaultTarget
				proxy.ReadWriteSplitting.Static.Rules = make([]ReadWriteSplittingStaticRule, len(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules))
				for i := range b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules {
					proxy.ReadWriteSplitting.Static.Rules[i].Name = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].Name
					proxy.ReadWriteSplitting.Static.Rules[i].Type = string(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].Type)
					proxy.ReadWriteSplitting.Static.Rules[i].Target = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].Target
					proxy.ReadWriteSplitting.Static.Rules[i].Regex = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].Regex
					proxy.ReadWriteSplitting.Static.Rules[i].AlgorithmName = string(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].AlgorithmName)
				}
			}
		}
	case b.TrafficStrategy.Spec.LoadBalance.SimpleLoadBalance != nil:
		{
			proxy.SimpleLoadBalance = &SimpleLoadBalance{
				BalancerType: string(b.TrafficStrategy.Spec.LoadBalance.SimpleLoadBalance.Kind),
			}
		}
	}

	if len(b.TrafficStrategy.Spec.CircuitBreaks) != 0 {
		proxy.Plugin.CircuitBreaks = b.TrafficStrategy.Spec.CircuitBreaks
	}
	if len(b.TrafficStrategy.Spec.ConcurrencyControls) != 0 {
		for _, control := range b.TrafficStrategy.Spec.ConcurrencyControls {
			// TODO: Convert CRD to configuration file json format.Need a better implementation
			// Ref: https://stackoverflow.com/questions/24613271/golang-is-conversion-between-different-struct-types-possible
			proxy.Plugin.ConcurrencyControls = append(proxy.Plugin.ConcurrencyControls, *(*ConcurrencyControl)(&control))
		}
	}

	return proxy
}

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
type Node struct {
	Name     string `json:"name"`
	Db       string `json:"db"`
	User     string `json:"user"`
	Password string `json:"password"`
	Host     string `json:"host"`
	Port     uint32 `json:"port"`
	Weight   int    `json:"weight"`
	Role     string `json:"role"`
}
