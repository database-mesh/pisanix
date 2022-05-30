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
	"fmt"
	"net/http"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"

	"github.com/gin-gonic/gin"
	log "github.com/sirupsen/logrus"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/apimachinery/pkg/util/json"
	"k8s.io/client-go/dynamic"
)

func GetConfig(client dynamic.Interface) gin.HandlerFunc {
	return func(ctx *gin.Context) {
		namespace := ctx.Param("namespace")
		appname := ctx.Param("appname")
		proxyConfig, err := getConfig(client, namespace, appname)
		if err != nil {
			ctx.JSON(http.StatusBadRequest, err)
			return
		}
		ctx.JSON(http.StatusOK, proxyConfig)
	}
}

func getConfig(client dynamic.Interface, namespace, appname string) (interface{}, error) {
	ctx := context.Background()

	proxyconfig := PisaProxyConfig{Admin: struct {
		Port     string `json:"port"`
		LogLevel string `json:"log_level"`
	}(struct {
		Port     string
		LogLevel string
	}{LogLevel: "INFO"})}

	virtualdatabases := schema.GroupVersionResource{
		Group:    "core.database-mesh.io",
		Version:  "v1alpha1",
		Resource: "virtualdatabases",
	}
	trafficstrategies := schema.GroupVersionResource{
		Group:    "core.database-mesh.io",
		Version:  "v1alpha1",
		Resource: "trafficstrategies",
	}
	databaseendpoints := schema.GroupVersionResource{
		Group:    "core.database-mesh.io",
		Version:  "v1alpha1",
		Resource: "databaseendpoints",
	}

	vdb, err := client.Resource(virtualdatabases).Namespace(namespace).Get(ctx, appname, metav1.GetOptions{})
	if err != nil {
		return nil, err
	}
	vdbSpec := &kubernetes.VirtualDatabaseSpec{}
	vdbs, _ := json.Marshal(vdb.Object["spec"])
	if err != nil {
		log.Errorf("%v", err)
		return nil, err
	}
	_ = json.Unmarshal(vdbs, vdbSpec)
	if err != nil {
		log.Errorf("%v", err)
		return nil, err
	}

	vdbmetedata := &metav1.ObjectMeta{}
	vdbm, _ := json.Marshal(vdb.Object["metadata"])

	_ = json.Unmarshal(vdbm, vdbmetedata)
	proxyconfig.Admin.Port = vdbmetedata.Annotations["database-mesh.io/metrics-port"]
	for _, service := range vdbSpec.Services {
		ts, err := client.Resource(trafficstrategies).Namespace(namespace).Get(ctx, service.TrafficStrategy, metav1.GetOptions{})
		if err != nil {
			return nil, err
		}
		tsSpec := &kubernetes.TrafficStrategySpec{}
		tsj, _ := json.Marshal(ts.Object["spec"])
		_ = json.Unmarshal(tsj, tsSpec)
		//TODO
		proxyself := Proxy{}
		if service.DatabaseService.DatabaseMySQL != nil {
			proxyself.BackendType = "mysql"
			proxyself.DB = service.DatabaseService.DatabaseMySQL.DB
			proxyself.Name = service.Name
			proxyself.Username = service.DatabaseService.DatabaseMySQL.Username
			proxyself.Password = service.DatabaseService.DatabaseMySQL.Password
			proxyself.PoolSize = service.DatabaseService.DatabaseMySQL.PoolSize
			proxyself.ListenAddr = fmt.Sprintf("%s:%s", service.DatabaseService.DatabaseMySQL.Host, service.DatabaseService.DatabaseMySQL.Port)
			if tsSpec.LoadBalance.SimpleLoadBalancer != nil {
				proxyself.SimpleLoadBalancer.BalancerType = tsSpec.LoadBalance.SimpleLoadBalancer.Kind
			}
			if len(tsSpec.CircuitBreaks) != 0 {
				proxyself.Plugins.CircuitBreaks = tsSpec.CircuitBreaks
			}
			if len(tsSpec.ConcurrencyControls) != 0 {
				for _, control := range tsSpec.ConcurrencyControls {
					//TODO: add comments
					proxyself.Plugins.ConcurrencyControls = append(proxyself.Plugins.ConcurrencyControls, *(*kubernetes.ConcurrencyControl)(&control))
				}
			}
		}
		dbes, err := client.Resource(databaseendpoints).Namespace(namespace).List(ctx, metav1.ListOptions{LabelSelector: labels.FormatLabels(tsSpec.Selector.MatchLabels)})
		if err != nil {
			log.Errorf("%v", err)
			return nil, err
		}
		for _, dbe := range dbes.Items {
			dbeSpec := &kubernetes.DatabaseEndpointSpec{}
			dbej, _ := json.Marshal(dbe.Object["spec"])
			_ = json.Unmarshal(dbej, dbeSpec)
			if dbeSpec.Database.MySQL != nil {
				dbeSpec.Database.MySQL.Name = dbe.GetName()
				dbeSpec.Database.MySQL.Weight = 1
				proxyconfig.MysqlNodes = append(proxyconfig.MysqlNodes, *dbeSpec.Database.MySQL)
			}
			if tsSpec.LoadBalance.SimpleLoadBalancer != nil {
				proxyself.SimpleLoadBalancer.Nodes = append(proxyself.SimpleLoadBalancer.Nodes, dbe.GetName())
			}
		}
		proxyconfig.Proxies = append(proxyconfig.Proxies, proxyself)
	}
	return proxyconfig, nil
}
