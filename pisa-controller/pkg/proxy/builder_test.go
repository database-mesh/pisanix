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
	"encoding/json"
	"fmt"
	"testing"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"
	v1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

func Test_ReadWriteSplittingDynamicConversion(t *testing.T) {
	builder := &ProxyBuilder{
		VirtualDatabaseService: kubernetes.VirtualDatabaseService{
			DatabaseService: kubernetes.DatabaseService{
				DatabaseMySQL: &kubernetes.DatabaseMySQL{
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
		TrafficStrategy: kubernetes.TrafficStrategy{
			ObjectMeta: v1.ObjectMeta{
				Name:      "catalogue",
				Namespace: "demotest",
			},
			Spec: kubernetes.TrafficStrategySpec{
				LoadBalance: &kubernetes.LoadBalance{
					ReadWriteSplitting: &kubernetes.ReadWriteSplitting{
						Dynamic: &kubernetes.ReadWriteSplittingDynamic{
							DefaultTarget: "",
							Rules: []kubernetes.ReadWriteSplittingRule{
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
							Discovery: kubernetes.ReadWriteDiscovery{
								MasterHighAvailability: &kubernetes.MasterHighAvailability{
									User:            "monitor",
									Password:        "monitor",
									MonitorInterval: 1000,
									ConnectionProbe: &kubernetes.ConnectionProbe{
										Probe: &kubernetes.Probe{
											PeriodMilliseconds:  2000,
											FailureThreshold:    3,
											TimeoutMilliseconds: 200,
										},
									},
									PingProbe: &kubernetes.PingProbe{
										Probe: &kubernetes.Probe{
											PeriodMilliseconds:  1000,
											TimeoutMilliseconds: 100,
											FailureThreshold:    3,
										},
									},
									ReplicationLagProbe: &kubernetes.ReplicationLagProbe{
										Probe: &kubernetes.Probe{
											PeriodMilliseconds:  1000,
											TimeoutMilliseconds: 3,
											FailureThreshold:    3,
										},
										MaxReplicationLag: 3,
									},
									ReadOnlyProbe: &kubernetes.ReadOnlyProbe{
										Probe: &kubernetes.Probe{
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
		DatabaseEndpoints: []kubernetes.DatabaseEndpoint{
			{
				ObjectMeta: v1.ObjectMeta{
					Name:      "catalogue",
					Namespace: "demotest",
					Labels: map[string]string{
						"source": "catalogue",
					},
				},
				Spec: kubernetes.DatabaseEndpointSpec{
					Database: kubernetes.Database{
						MySQL: &kubernetes.MySQL{
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
