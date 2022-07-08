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
	"reflect"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"

	"github.com/gin-gonic/gin"
	"k8s.io/client-go/dynamic"
)

func GetConfig(ctx *gin.Context) {
	namespace := ctx.Param("namespace")
	appname := ctx.Param("appname")
	client := kubernetes.GetClient()
	proxyConfig, err := getConfig(client.Client, namespace, appname)
	if err != nil {
		ctx.JSON(http.StatusBadRequest, err)
		return
	}

	ctx.JSON(http.StatusOK, proxyConfig)
}

func getConfig(client dynamic.Interface, namespace, appname string) (interface{}, error) {
	ctx := context.Background()

	builder := NewPisaProxyConfigBuilder()

	vdb, err := kubernetes.GetVirtualDatabaseWithContext(ctx, client, namespace, appname)
	if err != nil {
		return nil, err
	}

	tslist, err := kubernetes.GetTrafficStrategyListWithContext(ctx, client, namespace)
	if err != nil {
		return nil, err
	}

	dbeplist, err := kubernetes.GetDatabaseEndpointListWithContext(ctx, client, namespace)
	if err != nil {
		return nil, err
	}

	builders := []*ProxyBuilder{}
	for _, service := range vdb.Spec.Services {
		builder := NewProxyBuilder().SetVirtualDatabaseService(service)

		var tsobj kubernetes.TrafficStrategy
		for _, ts := range tslist.Items {
			if ts.Name == service.TrafficStrategy {
				tsobj = ts
				builder.SetTrafficStrategy(ts)
			}
		}

		dbeps := &kubernetes.DatabaseEndpointList{Items: []kubernetes.DatabaseEndpoint{}}
		for _, dbep := range dbeplist.Items {
			if reflect.DeepEqual(dbep.Labels, tsobj.Spec.Selector.MatchLabels) {
				dbeps.Items = append(dbeps.Items, dbep)
			}
		}

		builder.SetDatabaseEndpoints(dbeplist.Items)
		builders = append(builders, builder)
	}

	adminConfigBuilder := NewAdminConfigBuilder()
	adminConfigBuilder.SetLoglevel("INFO")
	builder.SetAdminConfigBuilder(adminConfigBuilder)

	proxyConfigBuilder := NewProxyConfigBuilder()
	proxyConfigBuilder.SetProxyBuilders(builders)
	builder.SetProxyConfigBuilder(proxyConfigBuilder)

	mysqlConfigBuilder := NewMySQLConfigBuilder()
	mysqlConfigBuilder.SetDatabaseEndpoints(dbeplist.Items)
	builder.SetMySQLConfigBuilder(mysqlConfigBuilder)

	proxyconfig := builder.Build()

	return proxyconfig, nil
}
