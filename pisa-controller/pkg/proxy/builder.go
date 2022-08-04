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
	LogLevel string `json:"log_level,omitempty"`
}

type MySQLConfig struct {
	Nodes []MySQLNode `json:"node"`
}

type ProxyConfig struct {
	Config []Proxy `json:"config"`
}

type PisaProxyConfigBuilder struct {
	AdminConfigBuilder *AdminConfigBuilder
	MySQLConfigBuilder *MySQLConfigBuilder
	ProxyConfigBuilder *ProxyConfigBuilder
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

func (b *ProxyConfigBuilder) Build() *ProxyConfig {
	config := &ProxyConfig{
		Config: []Proxy{},
	}

	for _, builder := range b.ProxyBuilders {
		conf := builder.Build()
		config.Config = append(config.Config, *conf)
	}

	return config
}

type ProxyBuilder struct {
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
	proxy.Name = b.VirtualDatabaseService.Name
	if b.VirtualDatabaseService.DatabaseService.DatabaseMySQL != nil {
		proxy.BackendType = "mysql"
		proxy.DB = b.VirtualDatabaseService.DatabaseService.DatabaseMySQL.DB
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

				if b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static != nil {
					proxy.ReadWriteSplitting = &ReadWriteSplitting{
						Static: &ReadWriteSplittingStatic{},
					}
					proxy.ReadWriteSplitting.Static.DefaultTarget = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.DefaultTarget
					proxy.ReadWriteSplitting.Static.Rules = make([]ReadWriteSplittingRule, len(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules))
					for i := range b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules {
						proxy.ReadWriteSplitting.Static.Rules[i].Name = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].Name
						proxy.ReadWriteSplitting.Static.Rules[i].Type = string(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].Type)
						proxy.ReadWriteSplitting.Static.Rules[i].Target = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].Target
						proxy.ReadWriteSplitting.Static.Rules[i].Regex = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].Regex
						proxy.ReadWriteSplitting.Static.Rules[i].AlgorithmName = string(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Static.Rules[i].AlgorithmName)
					}
				}
				if b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic != nil {
					proxy.ReadWriteSplitting = &ReadWriteSplitting{
						Dynamic: &ReadWriteSplittingDynamic{},
					}

					proxy.ReadWriteSplitting.Dynamic.DefaultTarget = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.DefaultTarget
					proxy.ReadWriteSplitting.Dynamic.Rules = make([]ReadWriteSplittingRule, len(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Rules))

					for i := range b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Rules {
						proxy.ReadWriteSplitting.Dynamic.Rules[i].Name = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Rules[i].Name
						proxy.ReadWriteSplitting.Dynamic.Rules[i].Type = string(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Rules[i].Type)
						proxy.ReadWriteSplitting.Dynamic.Rules[i].Target = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Rules[i].Target
						proxy.ReadWriteSplitting.Dynamic.Rules[i].Regex = b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Rules[i].Regex
						proxy.ReadWriteSplitting.Dynamic.Rules[i].AlgorithmName = string(b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Rules[i].AlgorithmName)
					}

					if b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability != nil {
						proxy.ReadWriteSplitting.Dynamic.Discovery = ReadWriteDiscovery{
							MasterHighAvailablity: &MasterHighAvailablity{
								Type:                      "mha",
								User:                      b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.User,
								Password:                  b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.Password,
								MonitorInterval:           b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.MonitorInterval,
								ConnectInterval:           b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ConnectionProbe.PeriodMilliseconds,
								ConnectTimeout:            b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ConnectionProbe.TimeoutMilliseconds,
								ConnectMaxFailures:        b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ConnectionProbe.FailureThreshold,
								PingInterval:              b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.PingProbe.PeriodMilliseconds,
								PingTimeout:               b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.PingProbe.TimeoutMilliseconds,
								PingMaxFailures:           b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.PingProbe.FailureThreshold,
								ReplicationLagInterval:    b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ReplicationLagProbe.PeriodMilliseconds,
								ReplicationLagTimeout:     b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ReplicationLagProbe.TimeoutMilliseconds,
								ReplicationLagMaxFailures: b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ReplicationLagProbe.FailureThreshold,
								MaxReplicationLag:         b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ReplicationLagProbe.MaxReplicationLag,
								ReadOnlyInterval:          b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ReadOnlyProbe.PeriodMilliseconds,
								ReadOnlyTimeout:           b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ReadOnlyProbe.TimeoutMilliseconds,
								ReadOnlyMaxFailures:       b.TrafficStrategy.Spec.LoadBalance.ReadWriteSplitting.Dynamic.Discovery.MasterHighAvailability.ReadOnlyProbe.FailureThreshold,
							},
						}
					}
				}
			}
		case b.TrafficStrategy.Spec.LoadBalance.SimpleLoadBalance != nil:
			{
				nodes := BuildMySQLNodesFromDatabaseEndpoints(b.DatabaseEndpoints)
				proxy.SimpleLoadBalance = &SimpleLoadBalance{
					BalancerType: string(b.TrafficStrategy.Spec.LoadBalance.SimpleLoadBalance.Kind),
					Nodes:        []string{},
				}
				for _, node := range nodes {
					proxy.SimpleLoadBalance.Nodes = append(proxy.SimpleLoadBalance.Nodes, node.Name)
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

		// if b.TrafficStrategy.Spec.LoadBalance.SimpleLoadBalance != nil {
		// 	for _, node := range nodes {
		// 		proxy.SimpleLoadBalance.Nodes = append(proxy.SimpleLoadBalance.Nodes, node.Name)
		// 	}
		// }

	}

	return proxy
}

func BuildMySQLNodesFromDatabaseEndpoints(dbeps []kubernetes.DatabaseEndpoint) []MySQLNode {
	nodes := []MySQLNode{}
	for _, dbep := range dbeps {
		if dbep.Spec.Database.MySQL != nil {
			nodes = append(nodes, MySQLNode{
				Name:     dbep.GetName(),
				Db:       dbep.Spec.Database.MySQL.DB,
				User:     dbep.Spec.Database.MySQL.User,
				Password: dbep.Spec.Database.MySQL.Password,
				Host:     dbep.Spec.Database.MySQL.Host,
				Port:     dbep.Spec.Database.MySQL.Port,
				Weight:   1,
				Role:     getDbEpRole(dbep.GetAnnotations()),
			})
		}
	}
	return nodes
}

const (
	ReadWriteSplittingRoleReadWrite = "readwrite"
	ReadWriteSplittingRoleRead      = "read"

	DatabaseEndpointRoleKey = "database-mesh.io/role"
)

func getDbEpRole(annotations map[string]string) (role string) {
	role = annotations[DatabaseEndpointRoleKey]
	if role == "" {
		role = ReadWriteSplittingRoleReadWrite
	}

	return
}

type MySQLConfigBuilder struct {
	VirtualDatabaseService kubernetes.VirtualDatabaseService
	TrafficStrategy        kubernetes.TrafficStrategy
	DatabaseEndpoints      []kubernetes.DatabaseEndpoint
}

func NewMySQLConfigBuilder() *MySQLConfigBuilder {
	return &MySQLConfigBuilder{}
}

func (b *MySQLConfigBuilder) SetDatabaseEndpoints(dbeps []kubernetes.DatabaseEndpoint) *MySQLConfigBuilder {
	b.DatabaseEndpoints = dbeps
	return b
}

func (b *MySQLConfigBuilder) Build() *MySQLConfig {
	config := &MySQLConfig{
		Nodes: []MySQLNode{},
	}

	config.Nodes = BuildMySQLNodesFromDatabaseEndpoints(b.DatabaseEndpoints)

	return config
}
