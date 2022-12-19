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
	"testing"

	"github.com/database-mesh/golang-sdk/kubernetes/client"
	"github.com/stretchr/testify/assert"
)

func Test_build(t *testing.T) {
	tslist := &client.TrafficStrategyList{
		Items: []client.TrafficStrategy{
			tsReadWriteSplttingDynamic,
		},
	}

	dslist := &client.DataShardList{
		Items: []client.DataShard{
			generalshard,
		},
	}

	dbeplist := &client.DatabaseEndpointList{
		Items: []client.DatabaseEndpoint{
			dbep,
		},
	}
	actual, _ := proxyConfigBuild(&vdb, tslist, dslist, dbeplist)

	expected := &PisaProxyConfig{
		Admin: AdminConfig{
			Host:     "0.0.0.0",
			Port:     5591,
			LogLevel: "INFO",
		},
		MySQL: MySQLConfig{
			Nodes: []MySQLNode{
				{
					Name:     "catalogue",
					Db:       "socksdb",
					User:     "root",
					Password: "fake_password",
					Host:     "catalogue-db.demotest",
					Port:     3306,
					Weight:   DefaultLoadBalanceWeight,
					Role:     ReadWriteSplittingRoleReadWrite,
				},
			},
		},
		Proxy: ProxyConfig{
			Config: []Proxy{
				*expectedProxy,
			},
		},
	}
	assertPisaProxyConfig(t, expected, actual, "pisaProxyConfig should be equal")
}

func assertPisaProxyConfig(t *testing.T, exp, act *PisaProxyConfig, msg ...interface{}) bool {
	return assertAdminConfig(t, exp.Admin, act.Admin, "admin should be equal") &&
		assertMySQLConfig(t, exp.MySQL, act.MySQL, "mysql should be equal") &&
		assertProxyConfig(t, exp.Proxy, act.Proxy, "proxy should be equal")
}

func assertAdminConfig(t *testing.T, exp, act AdminConfig, msg ...interface{}) bool {
	return assert.Equal(t, exp.Host, act.Host, "host should be equal") &&
		assert.Equal(t, exp.Port, act.Port, "port should be equal") &&
		assert.Equal(t, exp.LogLevel, act.LogLevel, "loglevel should be equal")
}

func assertMySQLConfig(t *testing.T, exp, act MySQLConfig, msg ...interface{}) bool {
	return assert.ElementsMatch(t, exp.Nodes, act.Nodes, "nodes should be equal")
}

func assertProxyConfig(t *testing.T, exp, act ProxyConfig, msg ...interface{}) bool {
	return assert.Equal(t, exp.Config[0].BackendType, act.Config[0].BackendType, "backendType should be equal") &&
		assert.Equal(t, exp.Config[0].DB, act.Config[0].DB, "db should be equal") &&
		assert.Equal(t, exp.Config[0].ListenAddr, act.Config[0].ListenAddr, "listenAddr should be equal") &&
		assert.Equal(t, exp.Config[0].Name, act.Config[0].Name, "name should be equal") &&
		assert.Equal(t, exp.Config[0].Password, act.Config[0].Password, "password should be equal") &&
		assert.Equal(t, exp.Config[0].PoolSize, act.Config[0].PoolSize, "poolSize should be equal") &&
		assert.Equal(t, exp.Config[0].User, act.Config[0].User, "user should be equal") &&
		assert.Equal(t, exp.Config[0].ServerVersion, act.Config[0].ServerVersion, "serverVersion should be equal") &&
		assertReadWriteSplittingDynamic(t, exp.Config[0].ReadWriteSplitting.Dynamic, act.Config[0].ReadWriteSplitting.Dynamic, "readWriteSplittingDynamic should be equal") &&
		assertPlugin(t, exp.Config[0].Plugin, act.Config[0].Plugin, "plugin should be equal")
}
