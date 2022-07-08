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
	Admin AdminConfig `json:"admin"`
	MySQL MySQLConfig `json:"mysql"`
	Proxy ProxyConfig `json:"proxy"`
}

type AdminConfig struct {
	Host     string `json:"host,omitempty"`
	Port     uint32 `json:"port,omitempty"`
	LogLevel string `json:"log_level"`
}

type MySQLConfig struct {
	Nodes []MySQLNode `json:"node"`
}

type ProxyConfig struct {
	Config []Proxy `json:"config"`
}

type PisaProxyConfigBuilder struct {
	// Admin AdminConfig
	// AdminHost     string
	// AdminPort     string
	// AdminLoglevel string
	// VirtualDatabases  []kubernetes.VirtualDatabase
	// TrafficStrategies []kubernetes.TrafficStrategy
	// DatabaseEndpoints []kubernetes.DatabaseEndpoint
	AdminConfigBuilder *AdminConfigBuilder
	MySQLConfigBuilder *MySQLConfigBuilder
	ProxyConfigBuilder *ProxyConfigBuilder
}

type AdminConfigBuilder struct {
	host     string
	port     uint32
	logLevel string
}

func NewAdminConfigBuilder() *AdminConfigBuilder {
	return &AdminConfigBuilder{}
}

func (b *AdminConfigBuilder) SetHost(host string) *AdminConfigBuilder {
	b.host = host
	return b
}

func (b *AdminConfigBuilder) SetPort(port uint32) *AdminConfigBuilder {
	b.port = port
	return b
}

// Need to add loglevel to env ?
func (b *AdminConfigBuilder) SetLoglevel(loglevel string) *AdminConfigBuilder {
	b.logLevel = loglevel
	return b
}

func (b *AdminConfigBuilder) Build() *AdminConfig {
	return &AdminConfig{
		Host:     b.host,
		Port:     b.port,
		LogLevel: b.logLevel,
	}
}

func NewPisaProxyConfigBuilder() *PisaProxyConfigBuilder {
	return &PisaProxyConfigBuilder{
		AdminConfigBuilder: NewAdminConfigBuilder(),
		MySQLConfigBuilder: NewMySQLConfigBuilder(),
		ProxyConfigBuilder: NewProxyConfigBuilder(),
	}
}

func (b *PisaProxyConfigBuilder) SetAdminConfigBuilder(builder *AdminConfigBuilder) *PisaProxyConfigBuilder {
	b.AdminConfigBuilder = builder
	return b
}

func (b *PisaProxyConfigBuilder) SetMySQLConfigBuilder(builder *MySQLConfigBuilder) *PisaProxyConfigBuilder {
	b.MySQLConfigBuilder = builder
	return b
}

func (b *PisaProxyConfigBuilder) SetProxyConfigBuilder(builder *ProxyConfigBuilder) *PisaProxyConfigBuilder {
	b.ProxyConfigBuilder = builder
	return b
}

func (b *PisaProxyConfigBuilder) Build() *PisaProxyConfig {
	config := &PisaProxyConfig{}
	config.Admin = *b.AdminConfigBuilder.Build()
	config.MySQL = *b.MySQLConfigBuilder.Build()
	config.Proxy = *b.ProxyConfigBuilder.Build()

	return config
}

// func (b *PisaProxyConfigBuilder) SetVirtualDatabases(vdbs []kubernetes.VirtualDatabase) *PisaProxyConfigBuilder {
// 	b.ProxyConfigBuilder.SetVirtualDatabases(vdbs)
// 	return b
// }

// func (b *PisaProxyConfigBuilder) SetTrafficStrategies(tses []kubernetes.TrafficStrategy) *PisaProxyConfigBuilder {
// 	return b
// }

// func (b *PisaProxyConfigBuilder) SetDatabaseEndpoints(dbeps []kubernetes.DatabaseEndpoint) *PisaProxyConfigBuilder {
// 	// b.DatabaseEndpoints = dbeps
// 	return b
// }

type ProxyConfigBuilder struct {
	ProxyBuilders []*ProxyBuilder
}

func NewProxyConfigBuilder() *ProxyConfigBuilder {
	return &ProxyConfigBuilder{
		ProxyBuilders: []*ProxyBuilder{},
	}
}

func (b *ProxyConfigBuilder) SetProxyBuilders(builders []*ProxyBuilder) *ProxyConfigBuilder {
	b.ProxyBuilders = builders
	return b
}

// func (b *ProxyConfigBuilder) SetVirtualDatabaseService(services []kubernetes.VirtualDatabaseService) *ProxyConfigBuilder {
// 	for _, svc := range services {
// 		b.ProxyBuilder = append(b.ProxyBuilder, NewProxyBuilder().SetVirtualDatabaseService(svc))
// 	}
// 	return b
// }

// func (b *ProxyConfigBuilder) SetTrafficStrategies(tses []kubernetes.TrafficStrategy) *ProxyConfigBuilder {
// 	// for _, ts := range tses {

// 	// }
// }

func (b *ProxyConfigBuilder) Build() *ProxyConfig {
	config := &ProxyConfig{
		Config: []Proxy{},
	}

	for _, builder := range b.ProxyBuilders {
		conf := builder.Build()
		fmt.Printf("-- %+v", conf)
		config.Config = append(config.Config, *conf)
	}

	return config
}

// func (b *PisaProxyConfigBuilder) Build() *PisaProxyConfig {
// 	config := &PisaProxyConfig{}

// 	return config
// }

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

	nodes := BuildMySQLNodesFromDatabaseEndpoints(b.DatabaseEndpoints)

	if b.TrafficStrategy.Spec.LoadBalance.SimpleLoadBalance != nil {
		for _, node := range nodes {
			proxy.SimpleLoadBalance.Nodes = append(proxy.SimpleLoadBalance.Nodes, node.Name)
		}
	}

	return proxy
}

type MySQLConfigBuilder struct {
	// DatabaseEndpoints []kubernetes.DatabaseEndpoint
	// Selectors         *metav1.LabelSelector
	VirtualDatabaseService kubernetes.VirtualDatabaseService
	TrafficStrategy        kubernetes.TrafficStrategy
	DatabaseEndpoints      []kubernetes.DatabaseEndpoint
}

func NewMySQLConfigBuilder() *MySQLConfigBuilder {
	return &MySQLConfigBuilder{}
}

// func (b *MySQLConfigBuilder) SetVirtualDatabaseService(svc kubernetes.VirtualDatabaseService) *ProxyBuilder {
// 	b.VirtualDatabaseService = svc
// 	return b
// }

// func (b *MySQLConfigBuilder) SetTrafficStrategy(ts kubernetes.TrafficStrategy) *ProxyBuilder {
// 	b.TrafficStrategy = ts
// 	return b
// }

func (b *MySQLConfigBuilder) SetDatabaseEndpoints(dbeps []kubernetes.DatabaseEndpoint) *MySQLConfigBuilder {
	b.DatabaseEndpoints = dbeps
	return b
}

// func (b *MySQLConfigBuilder) Build() *MySQLConfig {
// 	config := &MySQLConfig{}

// 	return config
// }

// func (b *MySQLConfigBuilder) SetDatabaseEndpoints(dbeps []kubernetes.DatabaseEndpoint) *MySQLConfigBuilder {
// 	b.DatabaseEndpoints = dbeps
// 	return b
// }

// func (b *MySQLConfigBuilder) SetSelectors(selectors metav1.LabelSelector) *MySQLConfigBuilder {
// 	b.Selectors = &selectors
// 	return b
// }

func (b *MySQLConfigBuilder) Build() *MySQLConfig {
	config := &MySQLConfig{
		Nodes: []MySQLNode{},
	}

	for _, dbep := range b.DatabaseEndpoints {
		// if reflect.DeepEqual(dbep.Labels, b.Selectors.MatchLabels) {
		config.Nodes = append(config.Nodes, MySQLNode{
			Name:     dbep.GetName(),
			Db:       dbep.Spec.Database.MySQL.DB,
			User:     dbep.Spec.Database.MySQL.User,
			Password: dbep.Spec.Database.MySQL.Password,
			Host:     dbep.Spec.Database.MySQL.Host,
			Port:     dbep.Spec.Database.MySQL.Port,
			Weight:   1,
			Role:     getDbEpRole(dbep.GetAnnotations()),
		})
		// }
	}

	return config
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
