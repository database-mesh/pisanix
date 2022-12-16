// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
package proxy

import (
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/database-mesh/golang-sdk/kubernetes/client"
	"github.com/stretchr/testify/assert"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

var vdb = client.VirtualDatabase{
	ObjectMeta: metav1.ObjectMeta{
		Name: "catalogue",
	},
	Spec: client.VirtualDatabaseSpec{
		Services: []client.VirtualDatabaseService{
			{
				DatabaseService: client.DatabaseService{
					DatabaseMySQL: &client.DatabaseMySQL{
						Host:          "127.0.0.1",
						Port:          3306,
						DB:            "socksdb",
						User:          "root",
						Password:      "fake_password",
						ServerVersion: "5.7.37",
						PoolSize:      3,
					},
				},
				Name:            "catalogue",
				TrafficStrategy: "catalogue",
				DataShard:       "catalogue",
			},
		},
	},
}

var dbep = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "catalogue",
		Namespace: "demotest",
		Annotations: map[string]string{
			DatabaseEndpointRoleKey: ReadWriteSplittingRoleReadWrite,
		},
		Labels: map[string]string{
			"source": "catalogue",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				DB:       "socksdb",
				Host:     "catalogue-db.demotest",
				Password: "fake_password",
				Port:     3306,
				User:     "root",
			},
		},
	},
}

var cbs = []client.CircuitBreak{
	{
		Regex: []string{
			"^select",
		},
	},
}

var ccs = []client.ConcurrencyControl{
	{
		Regex: []string{
			"^insert",
		},
		Duration:       10 * time.Second,
		MaxConcurrency: 10,
	},
}

var tsSimpleLoadBalance = client.TrafficStrategy{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "catalogue",
		Namespace: "demotest",
	},
	Spec: client.TrafficStrategySpec{
		Selector: &metav1.LabelSelector{
			MatchLabels: map[string]string{
				"source": "catalogue",
			},
		},
		CircuitBreaks:       cbs,
		ConcurrencyControls: ccs,
		LoadBalance: &client.LoadBalance{
			SimpleLoadBalance: &client.SimpleLoadBalance{
				Kind: client.LoadBalanceAlgorithmRoundRobin,
			},
		},
	},
}
var tsReadWriteSplittingStatic = client.TrafficStrategy{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "catalogue",
		Namespace: "demotest",
	},
	Spec: client.TrafficStrategySpec{
		Selector: &metav1.LabelSelector{
			MatchLabels: map[string]string{
				"source": "catalogue",
			},
		},
		CircuitBreaks:       cbs,
		ConcurrencyControls: ccs,
		LoadBalance: &client.LoadBalance{
			ReadWriteSplitting: &client.ReadWriteSplitting{
				Static: &client.ReadWriteSplittingStatic{
					DefaultTarget: "readwrite",
					Rules: []client.ReadWriteSplittingRule{
						{
							Name:          "write-rule",
							Regex:         []string{"^insert"},
							Target:        "readwrite",
							Type:          "regex",
							AlgorithmName: "roundrobin",
						},
						{
							Name:          "read-rule",
							Regex:         []string{"^select"},
							Target:        "read",
							Type:          "regex",
							AlgorithmName: "roundrobin",
						},
					},
				},
			},
		},
	},
}
var tsReadWriteSplttingDynamic = client.TrafficStrategy{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "catalogue",
		Namespace: "demotest",
	},
	Spec: client.TrafficStrategySpec{
		Selector: &metav1.LabelSelector{
			MatchLabels: map[string]string{
				"source": "catalogue",
			},
		},
		CircuitBreaks:       cbs,
		ConcurrencyControls: ccs,
		LoadBalance: &client.LoadBalance{
			ReadWriteSplitting: &client.ReadWriteSplitting{
				Dynamic: &client.ReadWriteSplittingDynamic{
					DefaultTarget: "readwrite",
					Rules: []client.ReadWriteSplittingRule{
						{
							Name:          "write-rule",
							Regex:         []string{"^insert"},
							Target:        "readwrite",
							Type:          "regex",
							AlgorithmName: "roundrobin",
						},
						{
							Name:          "read-rule",
							Regex:         []string{"^select"},
							Target:        "read",
							Type:          "regex",
							AlgorithmName: "roundrobin",
						},
					},
					Discovery: client.ReadWriteDiscovery{
						MasterHighAvailability: &client.MasterHighAvailability{
							User:            "monitor",
							Password:        "monitor",
							MonitorInterval: 1000,
							ConnectionProbe: &client.ConnectionProbe{
								Probe: &client.Probe{
									PeriodMilliseconds:  2000,
									FailureThreshold:    3,
									TimeoutMilliseconds: 200,
								},
							},
							PingProbe: &client.PingProbe{
								Probe: &client.Probe{
									PeriodMilliseconds:  1000,
									TimeoutMilliseconds: 100,
									FailureThreshold:    3,
								},
							},
							ReplicationLagProbe: &client.ReplicationLagProbe{
								Probe: &client.Probe{
									PeriodMilliseconds:  1000,
									TimeoutMilliseconds: 3,
									FailureThreshold:    3,
								},
								MaxReplicationLag: 3,
							},
							ReadOnlyProbe: &client.ReadOnlyProbe{
								Probe: &client.Probe{
									PeriodMilliseconds:  1000,
									TimeoutMilliseconds: 3,
									FailureThreshold:    3,
								},
							},
						},
					},
				},
			},
		},
	},
}

var expectedProxy = &Proxy{
	Name:          "catalogue",
	BackendType:   "mysql",
	DB:            "socksdb",
	User:          "root",
	Password:      "fake_password",
	ServerVersion: "5.7.37",
	PoolSize:      3,
	ListenAddr:    "127.0.0.1:3306",
	SimpleLoadBalance: &SimpleLoadBalance{
		BalancerType: "roundrobin",
		Nodes:        []string{"catalogue"},
	},
	ReadWriteSplitting: &ReadWriteSplitting{
		Static: &ReadWriteSplittingStatic{
			DefaultTarget: "readwrite",
			Rules: []ReadWriteSplittingRule{
				{
					Name:          "write-rule",
					Regex:         []string{"^insert"},
					Target:        "readwrite",
					Type:          "regex",
					AlgorithmName: "roundrobin",
				},
				{
					Name:          "read-rule",
					Regex:         []string{"^select"},
					Target:        "read",
					Type:          "regex",
					AlgorithmName: "roundrobin",
				},
			},
		},
		Dynamic: &ReadWriteSplittingDynamic{
			DefaultTarget: "readwrite",
			Rules: []ReadWriteSplittingRule{
				{
					Name:          "write-rule",
					Regex:         []string{"^insert"},
					Target:        "readwrite",
					Type:          "regex",
					AlgorithmName: "roundrobin",
				},
				{
					Name:          "read-rule",
					Regex:         []string{"^select"},
					Target:        "read",
					Type:          "regex",
					AlgorithmName: "roundrobin",
				},
			},
			Discovery: ReadWriteDiscovery{
				MasterHighAvailablity: &MasterHighAvailablity{
					Type:                      "mha",
					User:                      "monitor",
					Password:                  "monitor",
					MonitorInterval:           1000,
					ConnectInterval:           2000,
					ConnectMaxFailures:        3,
					ConnectTimeout:            200,
					PingInterval:              1000,
					PingTimeout:               100,
					PingMaxFailures:           3,
					ReplicationLagInterval:    1000,
					ReplicationLagTimeout:     3,
					ReplicationLagMaxFailures: 3,
					MaxReplicationLag:         3,
					ReadOnlyInterval:          1000,
					ReadOnlyTimeout:           3,
					ReadOnlyMaxFailures:       3,
				},
			},
		},
	},
	Sharding: []Sharding{
		{
			TableName: "testshard",
			ActualDatanodes: []string{
				"ms001",
			},
			TableStrategy: &TableStrategy{
				TableShardingAlgorithmName: "crc32mod",
				TableShardingColumn:        "order_id",
				ShardingCount:              4,
			},
		},
	},
	Plugin: &Plugin{
		CircuitBreaks: []CircuitBreak{
			{
				Regex: []string{
					"^select",
				},
			},
		},
		ConcurrencyControls: []ConcurrencyControl{
			{
				Regex: []string{
					"^insert",
				},
				Duration:       10 * time.Second,
				MaxConcurrency: 10,
			},
		},
	},
}

var expectedrwProxy = &Proxy{
	Name:          "catalogue",
	BackendType:   "mysql",
	DB:            "socksdb",
	User:          "root",
	Password:      "fake_password",
	ServerVersion: "5.7.37",
	PoolSize:      3,
	ListenAddr:    "127.0.0.1:3306",
	SimpleLoadBalance: &SimpleLoadBalance{
		BalancerType: "roundrobin",
		Nodes:        []string{"catalogue"},
	},
	Sharding: []Sharding{
		{
			TableName: "testshard",
			ActualDatanodes: []string{
				"ms001",
			},
			TableStrategy: &TableStrategy{
				TableShardingAlgorithmName: "crc32mod",
				TableShardingColumn:        "order_id",
				ShardingCount:              4,
			},
		},
	},
}

var expectedgeneralProxy = &Proxy{
	Name:          "catalogue",
	BackendType:   "mysql",
	DB:            "socksdb",
	User:          "root",
	Password:      "fake_password",
	ServerVersion: "5.7.37",
	PoolSize:      3,
	ListenAddr:    "127.0.0.1:3306",
	SimpleLoadBalance: &SimpleLoadBalance{
		BalancerType: "roundrobin",
		Nodes:        []string{"catalogue"},
	},
	Sharding: []Sharding{
		{
			TableName: "testshard",
			ActualDatanodes: []string{
				"ds001",
				"ds002",
			},
			TableStrategy: &TableStrategy{
				TableShardingAlgorithmName: "crc32mod",
				TableShardingColumn:        "order_id",
				ShardingCount:              4,
			},
		},
	},
}

func Test_ProxyBuilder(t *testing.T) {
	builders := []*ProxyBuilder{
		{
			VirtualDatabaseService: vdb.Spec.Services[0],
			TrafficStrategy:        tsSimpleLoadBalance,
			DatabaseEndpoints:      []client.DatabaseEndpoint{dbep},
		},
		{
			VirtualDatabaseService: vdb.Spec.Services[0],
			TrafficStrategy:        tsReadWriteSplittingStatic,
			DatabaseEndpoints:      []client.DatabaseEndpoint{dbep},
		},
		{
			VirtualDatabaseService: vdb.Spec.Services[0],
			TrafficStrategy:        tsReadWriteSplttingDynamic,
			DatabaseEndpoints:      []client.DatabaseEndpoint{dbep},
		},
	}

	for _, b := range builders {
		actual := b.Build()
		assertProxy(t, expectedProxy, actual, "proxy should be correct")
	}
}

var expectedShardedProxy = &Proxy{
	Name:          "catalogue",
	BackendType:   "mysql",
	DB:            "socksdb",
	User:          "root",
	Password:      "fake_password",
	ServerVersion: "5.7.37",
	PoolSize:      3,
	ListenAddr:    "127.0.0.1:3306",
	Sharding: []Sharding{
		{
			TableName: "testshard",
			ActualDatanodes: []string{
				"ms001",
			},
			TableStrategy: &TableStrategy{
				TableShardingAlgorithmName: "crc32mod",
				TableShardingColumn:        "order_id",
				ShardingCount:              4,
			},
		},
	},
}

func Test_ShardedProxyBuilder(t *testing.T) {
	builders := []*ProxyBuilder{
		{
			VirtualDatabaseService: vdb.Spec.Services[0],
			// TODO: temp
			DataShard:         rwshard,
			DatabaseEndpoints: []client.DatabaseEndpoint{dbepreadwrite, dbepread1, dbepread2},
		},
		{
			VirtualDatabaseService: vdb.Spec.Services[0],
			// TODO: temp
			DataShard:         generalshard,
			DatabaseEndpoints: []client.DatabaseEndpoint{dbepreadwrite, dbepread1, dbepread2},
		},
	}

	actualSharded := builders[0].Build()
	assertProxy(t, expectedShardedProxy, actualSharded, "proxy should be correct")
	actualGeneral := builders[1].Build()
	assertProxy(t, expectedgeneralProxy, actualGeneral, "proxy should be correct")
}

func assertProxy(t *testing.T, exp, act *Proxy, msg ...interface{}) bool {
	return assert.Equal(t, exp.BackendType, act.BackendType, "backendType should be equal") &&
		assert.Equal(t, exp.DB, act.DB, "db should be equal") &&
		assert.Equal(t, exp.ListenAddr, act.ListenAddr, "listenAddr should be equal") &&
		assert.Equal(t, exp.Name, act.Name, "name should be equal") &&
		assert.Equal(t, exp.Password, act.Password, "password should be equal") &&
		assert.Equal(t, exp.PoolSize, act.PoolSize, "poolSize should be equal") &&
		assert.Equal(t, exp.User, act.User, "user should be equal") &&
		assert.Equal(t, exp.ServerVersion, act.ServerVersion, "serverVersion should be equal") &&
		assertSimpleLoadBalance(t, exp.SimpleLoadBalance, act.SimpleLoadBalance, "simpleLoadBalance should be equal") &&
		assertReadWriteSplitting(t, exp.ReadWriteSplitting, act.ReadWriteSplitting, "readWriteSplitting should be equal") &&
		assertDataSharding(t, exp.Sharding, act.Sharding, "sharding should be equal") &&
		assertPlugin(t, exp.Plugin, act.Plugin, "plugin should be equal")
}

func assertDataSharding(t *testing.T, exp, act []Sharding, msg ...interface{}) bool {
	if act != nil && exp != nil {
		return assert.ElementsMatch(t, exp, act, "rules should be equal")
	}
	return true
}

func assertSimpleLoadBalance(t *testing.T, exp, act *SimpleLoadBalance, msg ...interface{}) bool {
	if act != nil && exp != nil {
		return assert.Equal(t, exp.BalancerType, act.BalancerType, "balancerType should be equal") &&
			assert.ElementsMatch(t, exp.Nodes, act.Nodes, "nodes should be equal")

	}
	return true
}

func assertReadWriteSplitting(t *testing.T, exp, act *ReadWriteSplitting, msg ...interface{}) bool {
	if act != nil && exp != nil {
		return assertReadWriteSplittingStatic(t, exp.Static, act.Static, "readWriteSplittingStatic should be equal") &&
			assertReadWriteSplittingDynamic(t, exp.Dynamic, act.Dynamic, "readWriteSplittingDynamic should be equal")
	}
	return true
}

func assertReadWriteSplittingStatic(t *testing.T, exp, act *ReadWriteSplittingStatic, msg ...interface{}) bool {
	if act != nil && exp != nil {
		return assert.Equal(t, exp.DefaultTarget, act.DefaultTarget, "defaultTarget should be equal") &&
			//TODO: check if this is valid
			assert.Equal(t, exp.Rules, act.Rules, "rules should be equal")
	}
	return true
}

func assertReadWriteSplittingDynamic(t *testing.T, exp, act *ReadWriteSplittingDynamic, msg ...interface{}) bool {
	if act != nil && exp != nil {
		return assert.Equal(t, exp.DefaultTarget, act.DefaultTarget, "defaultType should be equal") &&
			assert.Equal(t, exp.Rules, act.Rules, "rules should be equal") &&
			assertReadWriteDiscovery(t, exp.Discovery, act.Discovery, "discovery should be equal")

	}
	return true
}

func assertReadWriteDiscovery(t *testing.T, exp, act ReadWriteDiscovery, msg ...interface{}) bool {
	return assert.Equal(t, exp.Type, act.Type, "type should be equal") &&
		assert.Equal(t, exp.User, act.User, "user should be equal") &&
		assert.Equal(t, exp.Password, act.Password, "password should be equal") &&
		assert.Equal(t, exp.MonitorInterval, act.MonitorInterval, "monitorInterval should be equal") &&
		assert.Equal(t, exp.ConnectInterval, act.ConnectInterval, "connectInterval should be equal") &&
		assert.Equal(t, exp.ConnectTimeout, act.ConnectTimeout, "connectTimeout should be equal") &&
		assert.Equal(t, exp.ConnectMaxFailures, act.ConnectMaxFailures, "connectMaxFailures should be equal") &&
		assert.Equal(t, exp.PingInterval, act.PingInterval, "pingInterval should be equal") &&
		assert.Equal(t, exp.PingTimeout, act.PingTimeout, "pingTimeout should be equal") &&
		assert.Equal(t, exp.PingMaxFailures, act.PingMaxFailures, "pingMaxFailures should be equal") &&
		assert.Equal(t, exp.ReplicationLagInterval, act.ReplicationLagInterval, "replicationLagInterval should be equal") &&
		assert.Equal(t, exp.ReplicationLagTimeout, act.ReplicationLagTimeout, "replicationLagTimeout should be equal") &&
		assert.Equal(t, exp.ReplicationLagMaxFailures, act.ReplicationLagMaxFailures, "replicationLagMaxFailures should be equal") &&
		assert.Equal(t, exp.MaxReplicationLag, act.MaxReplicationLag, "maxReplicationLagLag should be equal") &&
		assert.Equal(t, exp.ReadOnlyInterval, act.ReadOnlyInterval, "readOnlyInterval should be equal") &&
		assert.Equal(t, exp.ReadOnlyTimeout, act.ReadOnlyTimeout, "readOnlyTimeout should be equal") &&
		assert.Equal(t, exp.ReadOnlyMaxFailures, act.ReadOnlyMaxFailures, "readOnlyMaxFailures should be equal")
}

func assertPlugin(t *testing.T, exp, act *Plugin, msg ...interface{}) bool {
	if act != nil && exp != nil {
		return assertCircuitBreaks(t, exp.CircuitBreaks, act.CircuitBreaks, "circuitBreaks should be equal") && assertConcurrencyControls(t, exp.ConcurrencyControls, act.ConcurrencyControls, "concurrencyControls should be equal")
	}
	return true
}

func assertCircuitBreaks(t *testing.T, exp, act []CircuitBreak, msg ...interface{}) bool {
	return assert.ElementsMatch(t, exp, act, "circuitBreaks should be equal")
}

func assertConcurrencyControls(t *testing.T, exp, act []ConcurrencyControl, msg ...interface{}) bool {
	return assert.ElementsMatch(t, exp, act, "concurrencyControls should be equal")
}
func Test_ReadWriteSplittingDynamicConversion(t *testing.T) {
	builder := &ProxyBuilder{
		VirtualDatabaseService: client.VirtualDatabaseService{
			DatabaseService: client.DatabaseService{
				DatabaseMySQL: &client.DatabaseMySQL{
					Host:          "127.0.0.1",
					Port:          3306,
					DB:            "socksdb",
					User:          "root",
					Password:      "fake_password",
					ServerVersion: "5.7.37",
					PoolSize:      3,
				},
			},
			Name:            "catalogue",
			TrafficStrategy: "catalogue",
		},
		TrafficStrategy: client.TrafficStrategy{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "catalogue",
				Namespace: "demotest",
			},
			Spec: client.TrafficStrategySpec{
				LoadBalance: &client.LoadBalance{
					ReadWriteSplitting: &client.ReadWriteSplitting{
						Dynamic: &client.ReadWriteSplittingDynamic{
							DefaultTarget: "",
							Rules: []client.ReadWriteSplittingRule{
								{
									Name:          "write-rule",
									Regex:         []string{"^insert"},
									Target:        "readwrite",
									Type:          "regex",
									AlgorithmName: "roundrobin",
								},
								{
									Name:          "read-rule",
									Regex:         []string{"^select"},
									Target:        "read",
									Type:          "regex",
									AlgorithmName: "roundrobin",
								},
							},
							Discovery: client.ReadWriteDiscovery{
								MasterHighAvailability: &client.MasterHighAvailability{
									User:            "monitor",
									Password:        "monitor",
									MonitorInterval: 1000,
									ConnectionProbe: &client.ConnectionProbe{
										Probe: &client.Probe{
											PeriodMilliseconds:  2000,
											FailureThreshold:    3,
											TimeoutMilliseconds: 200,
										},
									},
									PingProbe: &client.PingProbe{
										Probe: &client.Probe{
											PeriodMilliseconds:  1000,
											TimeoutMilliseconds: 100,
											FailureThreshold:    3,
										},
									},
									ReplicationLagProbe: &client.ReplicationLagProbe{
										Probe: &client.Probe{
											PeriodMilliseconds:  1000,
											TimeoutMilliseconds: 3,
											FailureThreshold:    3,
										},
										MaxReplicationLag: 3,
									},
									ReadOnlyProbe: &client.ReadOnlyProbe{
										Probe: &client.Probe{
											PeriodMilliseconds:  1000,
											TimeoutMilliseconds: 3,
											FailureThreshold:    3,
										},
									},
								},
							},
						},
					},
				},
			},
		},
		DatabaseEndpoints: []client.DatabaseEndpoint{
			{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "catalogue",
					Namespace: "demotest",
					Labels: map[string]string{
						"source": "catalogue",
					},
				},
				Spec: client.DatabaseEndpointSpec{
					Database: client.Database{
						MySQL: &client.MySQL{
							DB:       "socksdb",
							Host:     "catalogue-db.demotest",
							Password: "fake_password",
							Port:     3306,
							User:     "root",
						},
					},
				},
			},
		},
	}

	proxy := builder.Build()
	data, err := json.Marshal(proxy)
	if err != nil {
		t.Fatal(err)
	}

	fmt.Printf("%s\n", string(data))
}

func Test_ShardingConfig(t *testing.T) {
	/*
		config := PisaProxyConfig{
			Admin: AdminConfig{
				Host:     "0.0.0.0",
				Port:     8082,
				LogLevel: "INFO",
			},
			MySQL: MySQLConfig{
				Nodes: []MySQLNode{
					{
						Name:     "ds001",
						Db:       "socksdb",
						User:     "root",
						Password: "12345678",
						Host:     "127.0.0.1",
						Port:     3306,
						// Weight:   1,
						Role: "read",
					},
				},
			},
			Proxy: ProxyConfig{
				Config: []Proxy{
					{
						ListenAddr:    "0.0.0.0:9088",
						User:          "root",
						Password:      "12345678",
						DB:            "testrw",
						BackendType:   "mysql",
						PoolSize:      3,
						ServerVersion: "",
						Sharding: []Sharding{
							{
								TableName: "test_shard_hash",
								ActualDatanodes: []string{
									"ds001",
								},
								TableStrategy: &TableStrategy{
									TableShardingAlgorithmName: "crc32mod",
									TableShardingColumn:        "order_id",
									ShardingCount:              4,
								},
								DatabaseStrategy: &DatabaseStrategy{
									DatabaseShardingAlgorithmName: "mod",
									DatabaseShardingColumn:        "id",
								},
								DatabaseTableStrategy: &DatabaseTableStrategy{
									TableStrategy: TableStrategy{
										TableShardingAlgorithmName: "crc32_mod",
										TableShardingColumn:        "order_id",
										ShardingCount:              4,
									},
									DatabaseStrategy: DatabaseStrategy{
										DatabaseShardingAlgorithmName: "mod",
										DatabaseShardingColumn:        "order_id",
									},
								},
							},
						},
					},
				},
			},
			NodeGroup: NodeGroupConfig{
				Members: []NodeGroupMember{
					{
						Name:      "ms001",
						ReadWrite: "ds001",
						Reads: []string{
							"ds001",
							"ds002",
						},
					},
					{
						Name:      "ms002",
						ReadWrite: "ds002",
						Reads: []string{
							"ds002",
							"ds003",
						},
					},
				},
			},
		}
	*/

	// data, _ := json.Marshal(config)
}

var dbepreadwrite = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "ds000",
		Namespace: "demotest",
		Annotations: map[string]string{
			DatabaseEndpointRoleKey: ReadWriteSplittingRoleReadWrite,
		},
		Labels: map[string]string{
			"source": "catalogue",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				DB:       "socksdb",
				Host:     "catalogue-db.demotest",
				Password: "fake_password",
				Port:     3306,
				User:     "root",
			},
		},
	},
}

var dbepread1 = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "ds001",
		Namespace: "demotest",
		Annotations: map[string]string{
			DatabaseEndpointRoleKey: ReadWriteSplittingRoleRead,
		},
		Labels: map[string]string{
			"source": "catalogue",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				DB:       "socksdb",
				Host:     "catalogue-db.demotest",
				Password: "fake_password",
				Port:     3306,
				User:     "root",
			},
		},
	},
}

var dbepread2 = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "ds002",
		Namespace: "demotest",
		Annotations: map[string]string{
			DatabaseEndpointRoleKey: ReadWriteSplittingRoleRead,
		},
		Labels: map[string]string{
			"source": "catalogue",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				DB:       "socksdb",
				Host:     "catalogue-db.demotest",
				Password: "fake_password",
				Port:     3306,
				User:     "root",
			},
		},
	},
}

var generalshard = client.DataShard{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "catalogue",
		Namespace: "demotest",
		Labels: map[string]string{
			"source": "catalogue",
		},
	},
	Spec: client.DataShardSpec{
		Rules: []client.ShardingRule{
			{
				TableName: "testshard",
				TableStrategy: &client.TableStrategy{
					TableShardingAlgorithmName: "crc32mod",
					TableShardingColumn:        "order_id",
					ShardingCount:              4,
				},
				ActualDatanodes: client.ActualDatanodesValue{
					ValueSource: &client.ValueSourceType{
						ActualDatanodesNodeValue: &client.ActualDatanodesNodeValue{
							Nodes: []client.ValueFrom{
								{
									Value: "ds001",
								},
								{
									Value: "ds002",
								},
							},
						},
					},
				},
				ReadWriteSplittingGroup: []client.ReadWriteSplittingGroup{
					{
						Name: "ms001",
						Rules: []client.ReadWriteSplittingRule{
							{
								Name:   "read",
								Target: "read",
							},
							{
								Name:   "readwrite",
								Target: "readwrite",
							},
						},
					},
				},
			},
		},
	},
}

var rwshard = client.DataShard{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "catalogue",
		Namespace: "demotest",
		Labels: map[string]string{
			"source": "catalogue",
		},
	},
	Spec: client.DataShardSpec{
		Rules: []client.ShardingRule{
			{
				TableName: "testshard",
				TableStrategy: &client.TableStrategy{
					TableShardingAlgorithmName: "crc32mod",
					TableShardingColumn:        "order_id",
					ShardingCount:              4,
				},
				ActualDatanodes: client.ActualDatanodesValue{
					ValueSource: &client.ValueSourceType{
						ActualDatanodesNodeValue: &client.ActualDatanodesNodeValue{
							Nodes: []client.ValueFrom{
								{
									ValueFromReadWriteSplitting: &client.ValueFromReadWriteSplitting{
										Name: "ms001",
									},
								},
							},
						},
					},
				},
				ReadWriteSplittingGroup: []client.ReadWriteSplittingGroup{
					{
						Name: "ms001",
						Rules: []client.ReadWriteSplittingRule{
							{
								Name:   "read",
								Target: "read",
							},
							{
								Name:   "readwrite",
								Target: "readwrite",
							},
						},
					},
				},
			},
		},
	},
}

var expectedNodeGroup = NodeGroupConfig{
	Members: []NodeGroupMember{
		{
			Name: "ms001",
			Reads: []string{
				"ds001",
				"ds002",
			},
			ReadWrite: "ds000",
		},
	},
}

func Test_NodeGroupConfigBuilder(t *testing.T) {
	builder := NewNodeGroupConfigBuilder().SetDataShards(rwshard).SetDatabaseEndpoints([]client.DatabaseEndpoint{dbepreadwrite, dbepread1, dbepread2})
	cfg := builder.Build()
	assert.Equal(t, len(expectedNodeGroup.Members), len(cfg.Members), "members in total should be equal")
	for _, cfgm := range cfg.Members {
		for _, expm := range expectedNodeGroup.Members {
			if cfgm.Name == expm.Name {
				assert.EqualValues(t, expm.ReadWrite, cfgm.ReadWrite, "readwrite should be equal")
				assert.EqualValues(t, expm.Reads, cfgm.Reads, "read should be equal")
			}
		}
	}
}

var vdb1 = client.VirtualDatabase{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "vdb1",
		Namespace: "test",
	},
	Spec: client.VirtualDatabaseSpec{
		Services: []client.VirtualDatabaseService{
			{
				Name:            "svc1",
				TrafficStrategy: "ts1",
				QoSClaim:        "qc1",
				DatabaseService: client.DatabaseService{
					DatabaseMySQL: &client.DatabaseMySQL{
						Host:     "127.0.0.1",
						Port:     3306,
						User:     "root",
						Password: "root",
					},
				},
			},
			{
				Name:            "svc2",
				TrafficStrategy: "ts2",
				QoSClaim:        "qc2",
				DatabaseService: client.DatabaseService{
					DatabaseMySQL: &client.DatabaseMySQL{
						Host:     "127.0.0.1",
						Port:     3307,
						User:     "root",
						Password: "root",
					},
				},
			},
		},
	},
}

var qc1 = client.QoSClaim{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "qc1",
		Namespace: "test",
	},
	Spec: client.QoSClaimSpec{
		TrafficQoS: client.TrafficQoS{
			Name: "svc1",
			QoSGroup: client.QoSGroup{
				Rate: "1MB",
				Ceil: "1MB",
			},
		},
	},
}

var ts1 = client.TrafficStrategy{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "ts1",
		Namespace: "test",
	},
	Spec: client.TrafficStrategySpec{
		Selector: &metav1.LabelSelector{
			MatchLabels: map[string]string{
				"source": "ts1",
			},
		},
	},
}

var dbep1a = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "dbep-1a",
		Namespace: "test",
		Labels: map[string]string{
			"source": "ts1",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				Host:     "1.1.1.1",
				Port:     3306,
				User:     "root",
				Password: "root",
			},
		},
	},
}

var dbep1b = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "dbep-1b",
		Namespace: "test",
		Labels: map[string]string{
			"source": "ts1",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				Host:     "1.1.1.1",
				Port:     3307,
				User:     "root",
				Password: "root",
			},
		},
	},
}

var qc2 = client.QoSClaim{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "qc2",
		Namespace: "test",
	},
	Spec: client.QoSClaimSpec{
		TrafficQoS: client.TrafficQoS{
			Name: "svc2",
			QoSGroup: client.QoSGroup{
				Rate: "2MB",
				Ceil: "2MB",
			},
		},
	},
}

var ts2 = client.TrafficStrategy{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "ts2",
		Namespace: "test",
	},
	Spec: client.TrafficStrategySpec{
		Selector: &metav1.LabelSelector{
			MatchLabels: map[string]string{
				"source": "ts2",
			},
		},
	},
}

var dbep2a = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "dbep-2a",
		Namespace: "test",
		Labels: map[string]string{
			"source": "ts2",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				Host:     "1.1.1.2",
				Port:     3306,
				User:     "root",
				Password: "root",
			},
		},
	},
}

var dbep2b = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "dbep-2b",
		Namespace: "test",
		Labels: map[string]string{
			"source": "ts2",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				Host:     "1.1.1.2",
				Port:     3307,
				User:     "root",
				Password: "root",
			},
		},
	},
}

var vdb2 = client.VirtualDatabase{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "vdb3",
		Namespace: "test",
	},
	Spec: client.VirtualDatabaseSpec{
		Services: []client.VirtualDatabaseService{
			{
				Name:            "svc3",
				TrafficStrategy: "ts3",
				DatabaseService: client.DatabaseService{
					DatabaseMySQL: &client.DatabaseMySQL{
						Host:     "127.0.0.3",
						Port:     3306,
						User:     "root",
						Password: "root",
					},
				},
			},
		},
	},
}

var vdb3 = client.VirtualDatabase{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "vdb3",
		Namespace: "test",
	},
	Spec: client.VirtualDatabaseSpec{
		Services: []client.VirtualDatabaseService{
			{
				Name:            "svc3",
				TrafficStrategy: "ts3",
				QoSClaim:        "qc3",
				DatabaseService: client.DatabaseService{
					DatabaseMySQL: &client.DatabaseMySQL{
						Host:     "127.0.0.3",
						Port:     3306,
						User:     "root",
						Password: "root",
					},
				},
			},
		},
	},
}

var qc3 = client.QoSClaim{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "qc3",
		Namespace: "test",
	},
	Spec: client.QoSClaimSpec{
		TrafficQoS: client.TrafficQoS{
			Name: "svc3",
			QoSGroup: client.QoSGroup{
				Rate: "3MB",
				Ceil: "3MB",
			},
		},
	},
}

var ts3 = client.TrafficStrategy{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "ts3",
		Namespace: "test",
	},
	Spec: client.TrafficStrategySpec{
		Selector: &metav1.LabelSelector{
			MatchLabels: map[string]string{
				"source": "ts3",
			},
		},
	},
}

var dbep3 = client.DatabaseEndpoint{
	ObjectMeta: metav1.ObjectMeta{
		Name:      "dbep-3",
		Namespace: "test",
		Labels: map[string]string{
			"source": "ts3",
		},
	},
	Spec: client.DatabaseEndpointSpec{
		Database: client.Database{
			MySQL: &client.MySQL{
				Host:     "1.1.1.3",
				Port:     3306,
				User:     "root",
				Password: "root",
			},
		},
	},
}

var expectedDaemonConfig = PisaDaemonConfig{
	Apps: []App{
		{
			Name: "vdb1",
			Services: []Service{
				{
					Name: "svc1",
					QoSGroup: QoSGroup{
						Rate: "1MB",
						Ceil: "1MB",
					},
					Endpoints: []Endpoint{
						{
							IP:   "1.1.1.1",
							Port: 3306,
						},
						{
							IP:   "1.1.1.1",
							Port: 3307,
						},
					},
				},
				{
					Name: "svc2",
					QoSGroup: QoSGroup{
						Rate: "2MB",
						Ceil: "2MB",
					},
					Endpoints: []Endpoint{
						{
							IP:   "1.1.1.2",
							Port: 3306,
						},
						{
							IP:   "1.1.1.2",
							Port: 3307,
						},
					},
				},
			},
		},
		{
			Name: "vdb3",
			Services: []Service{
				{
					Name: "svc3",
					QoSGroup: QoSGroup{
						Rate: "3MB",
						Ceil: "3MB",
					},
					Endpoints: []Endpoint{
						{
							IP:   "1.1.1.3",
							Port: 3306,
						},
					},
				},
			},
		},
	},
}

func Test_PisaDaemonConfigBuilder(t *testing.T) {
	appbuilder1 := NewAppBuilder().SetVirtualDatabase(vdb1).SetTrafficStrategies([]client.TrafficStrategy{ts1, ts2}).SetDatabaseEndpoints([]client.DatabaseEndpoint{dbep1a, dbep1b, dbep2a, dbep2b}).SetQoSClaims([]client.QoSClaim{qc1, qc2})
	appbuilder3 := NewAppBuilder().SetVirtualDatabase(vdb3).SetTrafficStrategies([]client.TrafficStrategy{ts3}).SetDatabaseEndpoints([]client.DatabaseEndpoint{dbep3}).SetQoSClaims([]client.QoSClaim{qc3})
	cases := []struct {
		builder PisaDaemonConfigBuilder
		exp     PisaDaemonConfig
		message string
	}{
		{
			builder: *NewPisaDaemonConfigBuilder().SetAppBuilders([]*AppBuilder{appbuilder1, appbuilder3}),
			exp:     expectedDaemonConfig,
			message: "DaemonConfig should be equal",
		},
	}

	for _, c := range cases {
		cfg := c.builder.Build()
		assert.Equal(t, len(c.exp.Apps), len(cfg.Apps), c.message)
		assert.ElementsMatch(t, c.exp.Apps, cfg.Apps, c.message)
	}
}

func Test_daemonConfigBuild(t *testing.T) {
	vdblist := client.VirtualDatabaseList{Items: []client.VirtualDatabase{vdb1, vdb2, vdb3}}
	tslist := client.TrafficStrategyList{Items: []client.TrafficStrategy{ts1, ts2, ts3}}
	dbeplist := client.DatabaseEndpointList{Items: []client.DatabaseEndpoint{dbep1a, dbep1b, dbep2a, dbep2b, dbep3}}
	qclist := client.QoSClaimList{Items: []client.QoSClaim{qc1, qc2, qc3}}
	cases := []struct {
		vdblist  client.VirtualDatabaseList
		tslist   client.TrafficStrategyList
		dbeplist client.DatabaseEndpointList
		qclist   client.QoSClaimList
		exp      PisaDaemonConfig
		message  string
	}{
		{
			vdblist:  vdblist,
			tslist:   tslist,
			dbeplist: dbeplist,
			qclist:   qclist,
			exp:      expectedDaemonConfig,
			message:  "DaemonConfig should be equal",
		},
	}

	for _, c := range cases {
		cfg, _ := daemonConfigBuild(&c.vdblist, &c.tslist, &c.qclist, &c.dbeplist)
		assert.Equal(t, len(c.exp.Apps), len(cfg.(*PisaDaemonConfig).Apps), c.message)
		assert.ElementsMatch(t, c.exp.Apps, cfg.(*PisaDaemonConfig).Apps, c.message)
	}
}
