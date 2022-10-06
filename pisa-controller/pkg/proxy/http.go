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
	"context"
	"net/http"

	"github.com/database-mesh/golang-sdk/client"
	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"

	"github.com/gin-gonic/gin"
	"k8s.io/client-go/dynamic"
)

func GetProxyConfig(ctx *gin.Context) {
	namespace := ctx.Param("namespace")
	appname := ctx.Param("appname")
	c := client.GetClient()
	proxyConfig, err := getProxyConfig(ctx, c.Client, namespace, appname)
	if err != nil {
		ctx.JSON(http.StatusBadRequest, err)
		return
	}

	ctx.JSON(http.StatusOK, proxyConfig)
}

func getProxyConfig(ctx context.Context, c dynamic.Interface, namespace, appname string) (interface{}, error) {
	vdb, err := kubernetes.GetVirtualDatabaseWithContext(ctx, c, namespace, appname)
	if err != nil {
		return nil, err
	}

	tslist, err := kubernetes.GetTrafficStrategyListWithContext(ctx, c, namespace)
	if err != nil {
		return nil, err
	}

	dslist, err := kubernetes.GetDataShardListWithContext(ctx, c, namespace)
	if err != nil {
		return nil, err
	}

	dbeplist, err := kubernetes.GetDatabaseEndpointListWithContext(ctx, c, namespace)
	if err != nil {
		return nil, err
	}

	return proxyConfigBuild(vdb, tslist, dslist, dbeplist)

}

func proxyConfigBuild(vdb *client.VirtualDatabase, tslist *client.TrafficStrategyList, dslist *client.DataShardList, dbeplist *client.DatabaseEndpointList) (*PisaProxyConfig, error) {
	builder := NewPisaProxyConfigBuilder()
	builders := []*ProxyBuilder{}
	nodeGroupConfigBuilder := NewNodeGroupConfigBuilder()
	for _, service := range vdb.Spec.Services {
		builder := NewProxyBuilder().SetVirtualDatabaseService(service)

		var tsobj client.TrafficStrategy
		for _, ts := range tslist.Items {
			if ts.Name == service.TrafficStrategy {
				tsobj = ts
				builder.SetTrafficStrategy(ts)
			}
		}

		for _, ds := range dslist.Items {
			if ds.Name == service.DataShard {
				builder.SetDataShards(ds)
				nodeGroupConfigBuilder.SetDataShards(ds)
			}
		}

		//FIXME
		dbeps := &client.DatabaseEndpointList{Items: []client.DatabaseEndpoint{}}
		for _, dbep := range dbeplist.Items {
			for k, v := range tsobj.Spec.Selector.MatchLabels {
				if dbep.Labels[k] == v {
					dbeps.Items = append(dbeps.Items, dbep)
				}
			}
		}

		builder.SetDatabaseEndpoints(dbeplist.Items)
		builders = append(builders, builder)
	}

	adminConfigBuilder := NewAdminConfigBuilder()
	builder.SetAdminConfigBuilder(adminConfigBuilder)

	proxyConfigBuilder := NewProxyConfigBuilder()
	proxyConfigBuilder.SetProxyBuilders(builders)
	builder.SetProxyConfigBuilder(proxyConfigBuilder)

	mysqlConfigBuilder := NewMySQLConfigBuilder()
	mysqlConfigBuilder.SetDatabaseEndpoints(dbeplist.Items)
	builder.SetMySQLConfigBuilder(mysqlConfigBuilder)

	nodeGroupConfigBuilder.SetDatabaseEndpoints(dbeplist.Items)
	builder.SetNodeGroupConfigBuilder(nodeGroupConfigBuilder)

	proxyconfig := builder.Build()

	return proxyconfig, nil
}

func GetDaemonConfig(ctx *gin.Context) {
	daemonConfig, err := getDaemonConfig(ctx)
	if err != nil {
		ctx.JSON(http.StatusBadRequest, err)
		return
	}

	ctx.JSON(http.StatusOK, daemonConfig)
}

func getDaemonConfig(ctx context.Context) (interface{}, error) {
	cfg := PisaDaemonConfig{}
	return cfg, nil
}
