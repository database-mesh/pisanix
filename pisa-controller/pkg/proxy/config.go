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

	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"

	"github.com/gin-gonic/gin"
	log "github.com/sirupsen/logrus"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/apimachinery/pkg/util/json"
	"k8s.io/client-go/dynamic"
)

const (
	DatabaseEndpointRoleKey = "database-mesh.io/role"
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

	proxyconfig := PisaProxyConfig{Admin: struct {
		Host     string `json:"host,omitempty"`
		Port     uint32 `json:"port,omitempty"`
		LogLevel string `json:"log_level"`
	}(struct {
		Host     string
		Port     uint32
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

	for _, service := range vdbSpec.Services {
		builder := NewProxyBuilder().SetVirtualDatabaseService(service)

		ts, err := client.Resource(trafficstrategies).Namespace(namespace).Get(ctx, service.TrafficStrategy, metav1.GetOptions{})
		if err != nil {
			return nil, err
		}
		tsobj := &kubernetes.TrafficStrategy{}
		tsdata, _ := json.Marshal(ts)
		_ = json.Unmarshal(tsdata, tsobj)

		builder.SetTrafficStrategy(*tsobj)

		dbeps, err := client.Resource(databaseendpoints).Namespace(namespace).List(ctx, metav1.ListOptions{LabelSelector: labels.FormatLabels(tsobj.Spec.Selector.MatchLabels)})
		if err != nil {
			log.Errorf("%v", err)
			return nil, err
		}

		dbepsobj := &kubernetes.DatabaseEndpointList{}
		dbepsdata, _ := json.Marshal(dbeps)
		_ = json.Unmarshal(dbepsdata, dbepsobj)

		builder.SetDatabaseEndpoints(dbepsobj.Items)
		proxy := builder.Build()

		proxyconfig.Mysql.Nodes = BuildMySQLNodesFromDatabaseEndpoints(dbeps)

		if tsobj.Spec.LoadBalance.SimpleLoadBalance != nil {
			for _, node := range proxyconfig.Mysql.Nodes {
				proxy.SimpleLoadBalance.Nodes = append(proxy.SimpleLoadBalance.Nodes, node.Name)
			}
		}

		proxyconfig.Proxy.Configs = append(proxyconfig.Proxy.Configs, *proxy)
	}
	return proxyconfig, nil
}

func BuildMySQLNodesFromDatabaseEndpoints(dbeps *unstructured.UnstructuredList) []Node {
	nodes := []Node{}
	for _, dbep := range dbeps.Items {
		spec := &kubernetes.DatabaseEndpointSpec{}
		dbeps, _ := json.Marshal(dbep.Object["spec"])
		_ = json.Unmarshal(dbeps, spec)

		if spec.Database.MySQL != nil {
			nodes = append(nodes, Node{
				Name:     dbep.GetName(),
				Db:       spec.Database.MySQL.DB,
				User:     spec.Database.MySQL.User,
				Password: spec.Database.MySQL.Password,
				Host:     spec.Database.MySQL.Host,
				Port:     spec.Database.MySQL.Port,
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
)

func getDbEpRole(annotations map[string]string) (role string) {
	role = annotations[DatabaseEndpointRoleKey]
	if role == "" {
		role = ReadWriteSplittingRoleReadWrite
	}

	return
}
