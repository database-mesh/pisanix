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

	"github.com/database-mesh/golang-sdk/client"
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
	actual, _ := build(&vdb, tslist, dslist, dbeplist)

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
				*expectedrwProxy,
			},
		},
	}
	assertPisaProxyConfig(t, expected, actual, "pisaProxyConfig should be equal")
}

func assertPisaProxyConfig(t *testing.T, exp, act *PisaProxyConfig, msg ...interface{}) bool {
	return assertAdminConfig(t, act.Admin, exp.Admin, "admin should be equal") &&
		assertMySQLConfig(t, act.MySQL, exp.MySQL, "mysql should be equal") &&
		assertProxyConfig(t, act.Proxy, exp.Proxy, "proxy should be equal")
}

func assertAdminConfig(t *testing.T, exp, act AdminConfig, msg ...interface{}) bool {
	return assert.Equal(t, act.Host, exp.Host, "host should be equal") &&
		assert.Equal(t, act.Port, exp.Port, "port should be equal") &&
		assert.Equal(t, act.LogLevel, exp.LogLevel, "loglevel should be equal")
}

func assertMySQLConfig(t *testing.T, exp, act MySQLConfig, msg ...interface{}) bool {
	return assert.ElementsMatch(t, act.Nodes, exp.Nodes, "nodes should be equal")
}

func assertProxyConfig(t *testing.T, exp, act ProxyConfig, msg ...interface{}) bool {
	return assert.Equal(t, exp.Config[0].BackendType, exp.Config[0].BackendType, "backendType should be equal") &&
		assert.Equal(t, exp.Config[0].DB, exp.Config[0].DB, "db should be equal") &&
		assert.Equal(t, exp.Config[0].ListenAddr, exp.Config[0].ListenAddr, "listenAddr should be equal") &&
		assert.Equal(t, exp.Config[0].Name, exp.Config[0].Name, "name should be equal") &&
		assert.Equal(t, exp.Config[0].Password, exp.Config[0].Password, "password should be equal") &&
		assert.Equal(t, exp.Config[0].PoolSize, exp.Config[0].PoolSize, "poolSize should be equal") &&
		assert.Equal(t, exp.Config[0].User, exp.Config[0].User, "user should be equal") &&
		assert.Equal(t, exp.Config[0].ServerVersion, exp.Config[0].ServerVersion, "serverVersion should be equal") &&
		assertReadWriteSplittingDynamic(t, exp.Config[0].ReadWriteSplitting.Dynamic, exp.Config[0].ReadWriteSplitting.Dynamic, "readWriteSplittingDynamic should be equal") &&
		assertPlugin(t, exp.Config[0].Plugin, exp.Config[0].Plugin, "plugin should be equal")
}
