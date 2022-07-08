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
	"reflect"

	"github.com/database-mesh/pisanix/pisa-controller/pkg/kubernetes"

	"github.com/gin-gonic/gin"
	log "github.com/sirupsen/logrus"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
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

	fmt.Printf("Config: %+v", proxyConfig)

	ctx.JSON(http.StatusOK, proxyConfig)
}

func getConfig(client dynamic.Interface, namespace, appname string) (interface{}, error) {
	ctx := context.Background()

	// proxyconfig := PisaProxyConfig{Admin: struct {
	// 	Host     string `json:"host,omitempty"`
	// 	Port     uint32 `json:"port,omitempty"`
	// 	LogLevel string `json:"log_level"`
	// }(struct {
	// 	Host     string
	// 	Port     uint32
	// 	LogLevel string
	// }{LogLevel: "INFO"})}

	// proxyconfig := PisaProxyConfig{
	// 	Admin: AdminConfig{},
	// 	Proxy: ProxyConfig{},
	// 	MySQL: MySQLConfig{},
	// }

	builder := NewPisaProxyConfigBuilder()
	adminConfigBuilder := NewAdminConfigBuilder().SetHost("0.0.0.0").SetPort(0).SetLoglevel("INFO")
	builder.SetAdminConfigBuilder(adminConfigBuilder)
	proxyConfigBuilder := NewProxyConfigBuilder()
	mysqlConfigBuilder := NewMySQLConfigBuilder()

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

	dbeps, err := client.Resource(databaseendpoints).Namespace(namespace).List(ctx, metav1.ListOptions{})
	if err != nil {
		log.Errorf("%v", err)
		return nil, err
	}

	dbepsobj := &kubernetes.DatabaseEndpointList{}
	dbepsdata, _ := json.Marshal(dbeps)
	_ = json.Unmarshal(dbepsdata, dbepsobj)

	builders := []*ProxyBuilder{}
	for _, service := range vdbSpec.Services {
		builder := NewProxyBuilder().SetVirtualDatabaseService(service)

		//TODO: need refactor
		ts, err := client.Resource(trafficstrategies).Namespace(namespace).Get(ctx, service.TrafficStrategy, metav1.GetOptions{})
		if err != nil {
			return nil, err
		}
		tsobj := &kubernetes.TrafficStrategy{}
		tsdata, _ := json.Marshal(ts)
		_ = json.Unmarshal(tsdata, tsobj)

		fmt.Printf("ts: %+v\n", *tsobj)

		builder.SetTrafficStrategy(*tsobj)

		// dbeps, err := client.Resource(databaseendpoints).Namespace(namespace).List(ctx, metav1.ListOptions{LabelSelector: labels.FormatLabels(tsobj.Spec.Selector.MatchLabels)})
		// if err != nil {
		// 	log.Errorf("%v", err)
		// 	return nil, err
		// }

		// dbepsobj := &kubernetes.DatabaseEndpointList{}
		// dbepsdata, _ := json.Marshal(dbeps)
		// _ = json.Unmarshal(dbepsdata, dbepsobj)

		dbeps := &kubernetes.DatabaseEndpointList{Items: []kubernetes.DatabaseEndpoint{}}
		for _, dbep := range dbepsobj.Items {
			if reflect.DeepEqual(dbep.Labels, tsobj.Spec.Selector.MatchLabels) {
				dbeps.Items = append(dbeps.Items, dbep)
			}
		}

		builder.SetDatabaseEndpoints(dbepsobj.Items)

		fmt.Printf("--build: %+v\n", builder)

		builders = append(builders, builder)
	}

	fmt.Printf("--builder: %+v\n", len(builders))
	fmt.Printf("build: %+v\n", builders[0].Build())

	mysqlConfigBuilder.SetDatabaseEndpoints(dbepsobj.Items)

	proxyConfigBuilder.SetProxyBuilders(builders)
	builder.SetProxyConfigBuilder(proxyConfigBuilder)
	builder.SetMySQLConfigBuilder(mysqlConfigBuilder)
	proxyconfig := builder.Build()

	return proxyconfig, nil
}

// func BuildMySQLNodesFromDatabaseEndpoints(dbeps *unstructured.UnstructuredList) []MySQLNode {
func BuildMySQLNodesFromDatabaseEndpoints(dbeps []kubernetes.DatabaseEndpoint) []MySQLNode {
	nodes := []MySQLNode{}
	// for _, dbep := range dbeps.Items {
	for _, dbep := range dbeps {
		// spec := &kubernetes.DatabaseEndpointSpec{}
		// dbeps, _ := json.Marshal(dbep.Object["spec"])
		// _ = json.Unmarshal(dbeps, spec)

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
)

func getDbEpRole(annotations map[string]string) (role string) {
	role = annotations[DatabaseEndpointRoleKey]
	if role == "" {
		role = ReadWriteSplittingRoleReadWrite
	}

	return
}
