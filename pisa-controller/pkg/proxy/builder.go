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

	"github.com/database-mesh/golang-sdk/client"
)

type PisaProxyConfig struct {
	Admin     AdminConfig     `json:"admin"`
	MySQL     MySQLConfig     `json:"mysql"`
	Proxy     ProxyConfig     `json:"proxy"`
	NodeGroup NodeGroupConfig `json:"node_group,omitempty"`
}

type NodeGroupConfig struct {
	Members []NodeGroupMember `json:"member"`
}

type NodeGroupMember struct {
	Name      string   `json:"name"`
	Reads     []string `json:"reads"`
	ReadWrite string   `json:"readwrite"`
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
	AdminConfigBuilder     *AdminConfigBuilder
	MySQLConfigBuilder     *MySQLConfigBuilder
	ProxyConfigBuilder     *ProxyConfigBuilder
	NodeGroupConfigBuilder *NodeGroupConfigBuilder
}

func NewPisaProxyConfigBuilder() *PisaProxyConfigBuilder {
	return &PisaProxyConfigBuilder{
		AdminConfigBuilder:     NewAdminConfigBuilder(),
		MySQLConfigBuilder:     NewMySQLConfigBuilder(),
		ProxyConfigBuilder:     NewProxyConfigBuilder(),
		NodeGroupConfigBuilder: NewNodeGroupConfigBuilder(),
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

func (b *PisaProxyConfigBuilder) SetNodeGroupConfigBuilder(builder *NodeGroupConfigBuilder) *PisaProxyConfigBuilder {
	b.NodeGroupConfigBuilder = builder
	return b
}

func (b *PisaProxyConfigBuilder) Build() *PisaProxyConfig {
	config := &PisaProxyConfig{}
	config.Admin = *b.AdminConfigBuilder.Build()
	config.MySQL = *b.MySQLConfigBuilder.Build()
	config.Proxy = *b.ProxyConfigBuilder.Build()
	config.NodeGroup = *b.NodeGroupConfigBuilder.Build()

	return config
}

type AdminConfigBuilder struct {
	host     string
	port     uint32
	logLevel string
}

func NewAdminConfigBuilder() *AdminConfigBuilder {
	return &AdminConfigBuilder{
		host:     "0.0.0.0",
		port:     5591,
		logLevel: "INFO",
	}
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
	VirtualDatabaseService client.VirtualDatabaseService
	TrafficStrategy        client.TrafficStrategy
	DatabaseEndpoints      []client.DatabaseEndpoint
	DataShard              client.DataShard
}

func NewProxyBuilder() *ProxyBuilder {
	return &ProxyBuilder{}
}

func (b *ProxyBuilder) SetVirtualDatabaseService(svc client.VirtualDatabaseService) *ProxyBuilder {
	b.VirtualDatabaseService = svc
	return b
}

func (b *ProxyBuilder) SetTrafficStrategy(ts client.TrafficStrategy) *ProxyBuilder {
	b.TrafficStrategy = ts
	return b
}

func (b *ProxyBuilder) SetDatabaseEndpoints(dbeps []client.DatabaseEndpoint) *ProxyBuilder {
	b.DatabaseEndpoints = dbeps
	return b
}

func (b *ProxyBuilder) SetDataShards(shard client.DataShard) *ProxyBuilder {
	b.DataShard = shard
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

		if b.TrafficStrategy.Spec.LoadBalance != nil {

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
		}
		if len(b.DataShard.Spec.Rules) != 0 {
			proxy.Sharding = []Sharding{}
			if len(b.DataShard.Spec.Rules) != 0 {
				for _, r := range b.DataShard.Spec.Rules {
					s := Sharding{
						TableName: r.TableName,
						TableStrategy: &TableStrategy{
							TableShardingAlgorithmName: r.TableStrategy.TableShardingAlgorithmName,
							TableShardingColumn:        r.TableStrategy.TableShardingColumn,
							ShardingCount:              r.TableStrategy.ShardingCount,
						},
						DatabaseStrategy: (*DatabaseStrategy)(r.DatabaseStrategy),
						// DatabaseTableStrategy: r.DatabaseTableStrategy,
					}
					actualNodes := []string{}
					if r.ActualDatanodes.ValueSource != nil {
						// if r.ActualDatanodes.ValueSource.ActualDatanodesExpressionValue != nil {					}
						if r.ActualDatanodes.ValueSource.ActualDatanodesNodeValue != nil {
							if len(r.ActualDatanodes.ValueSource.ActualDatanodesNodeValue.Nodes) != 0 {
								for _, n := range r.ActualDatanodes.ValueSource.ActualDatanodesNodeValue.Nodes {
									if n.Value != "" {
										actualNodes = append(actualNodes, n.Value)
									} else if n.ValueFromReadWriteSplitting != nil {
										actualNodes = append(actualNodes, n.ValueFromReadWriteSplitting.Name)
									} else {
									}
								}
							}
						}
					}
					s.ActualDatanodes = actualNodes
					proxy.Sharding = append(proxy.Sharding, s)
				}
			}
		}

		if b.TrafficStrategy.Spec.CircuitBreaks != nil || b.TrafficStrategy.Spec.ConcurrencyControls != nil {
			proxy.Plugin = &Plugin{}
		}

		if len(b.TrafficStrategy.Spec.CircuitBreaks) != 0 {
			for _, cb := range b.TrafficStrategy.Spec.CircuitBreaks {
				proxy.Plugin.CircuitBreaks = append(proxy.Plugin.CircuitBreaks, CircuitBreak{
					Regex: cb.Regex,
				})
			}

		}

		if len(b.TrafficStrategy.Spec.ConcurrencyControls) != 0 {
			for _, control := range b.TrafficStrategy.Spec.ConcurrencyControls {
				// TODO: Convert CRD to configuration file json format.Need a better implementation
				// Ref: https://stackoverflow.com/questions/24613271/golang-is-conversion-between-different-struct-types-possible
				proxy.Plugin.ConcurrencyControls = append(proxy.Plugin.ConcurrencyControls, *(*ConcurrencyControl)(&control))
			}
		}

	}

	return proxy
}

func BuildMySQLNodesFromDatabaseEndpoints(dbeps []client.DatabaseEndpoint) []MySQLNode {
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
				Weight:   DefaultLoadBalanceWeight,
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

	DefaultLoadBalanceWeight = 1
)

func getDbEpRole(annotations map[string]string) (role string) {
	role = annotations[DatabaseEndpointRoleKey]
	if role == "" {
		role = ReadWriteSplittingRoleReadWrite
	}

	return
}

type MySQLConfigBuilder struct {
	VirtualDatabaseService client.VirtualDatabaseService
	TrafficStrategy        client.TrafficStrategy
	DatabaseEndpoints      []client.DatabaseEndpoint
}

func NewMySQLConfigBuilder() *MySQLConfigBuilder {
	return &MySQLConfigBuilder{}
}

func (b *MySQLConfigBuilder) SetDatabaseEndpoints(dbeps []client.DatabaseEndpoint) *MySQLConfigBuilder {
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

type NodeGroupConfigBuilder struct {
	DataShards        client.DataShard
	DatabaseEndpoints []client.DatabaseEndpoint
}

func NewNodeGroupConfigBuilder() *NodeGroupConfigBuilder {
	return &NodeGroupConfigBuilder{
		DataShards: client.DataShard{},
	}
}

func (b *NodeGroupConfigBuilder) SetDataShards(ds client.DataShard) *NodeGroupConfigBuilder {
	b.DataShards = ds
	return b
}

func (b *NodeGroupConfigBuilder) SetDatabaseEndpoints(dbeps []client.DatabaseEndpoint) *NodeGroupConfigBuilder {
	b.DatabaseEndpoints = dbeps
	return b
}

func (b *NodeGroupConfigBuilder) Build() *NodeGroupConfig {
	dbepmap := map[string][]string{}
	for _, dbep := range b.DatabaseEndpoints {
		if dbep.Annotations[DatabaseEndpointRoleKey] == ReadWriteSplittingRoleRead {
			dbepmap[ReadWriteSplittingRoleRead] = append(dbepmap[ReadWriteSplittingRoleRead], dbep.Name)
		}
		if dbep.Annotations[DatabaseEndpointRoleKey] == ReadWriteSplittingRoleReadWrite {
			dbepmap[ReadWriteSplittingRoleReadWrite] = append(dbepmap[ReadWriteSplittingRoleReadWrite], dbep.Name)
		}
	}

	config := &NodeGroupConfig{
		Members: []NodeGroupMember{},
	}

	for _, r := range b.DataShards.Spec.Rules {
		if r.ReadWriteSplittingGroup != nil {
			for _, g := range r.ReadWriteSplittingGroup {
				m := NodeGroupMember{
					Name: g.Name,
				}
				for _, rule := range g.Rules {
					//FIXME: if multiple dbeps share same role but belong to different node groups
					if rule.Target == ReadWriteSplittingRoleRead {
						m.Reads = dbepmap[rule.Target]
					}
					if rule.Target == ReadWriteSplittingRoleReadWrite {
						m.ReadWrite = dbepmap[rule.Target][0]
					}
				}
				config.Members = append(config.Members, m)
				//FIXME: do not support multiple ReadWriteSplittingGroup now
				break
			}
		}
	}

	return config
}

type PisaDaemonConfig struct {
	// Global GlobalConfig `json:"global"`
	Apps []App `json:"app"`
}

type GlobalConfig struct {
	EgressDevice string `json:"egress_device"`
	BridgeDevice string `json:"bridge_device"`
}

type App struct {
	Name     string    `json:"name"`
	Services []Service `json:"service,omitempty"`
}

type Service struct {
	Name      string     `json:"name"`
	QoSGroups []QoSGroup `json:"qos_group"`
}

type QoSClassKind string

const (
	QoSClassKindGuaranteed = "guaranteed"
	QoSClassKindBurstable  = "burstable"
	QoSClassKindBestEffort = "besteffort"
)

type QoSGroup struct {
	Rate string `json:"rate,omitempty"`
	Ceil string `json:"ceil,omitempty"`
}
