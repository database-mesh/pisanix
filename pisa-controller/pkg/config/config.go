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

package config

import (
	"context"
	"net/http"

	"github.com/gin-gonic/gin"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/client-go/dynamic"
	"k8s.io/client-go/rest"
)

func GetConfig(ctx *gin.Context) {
	namespace := ctx.Param("namespace")
	appname := ctx.Param("appname")
	proxyConfig, err := getConfig(namespace, appname)
	if err != nil {
		ctx.JSON(http.StatusBadRequest, err)
		return
	}
	ctx.JSON(http.StatusOK, proxyConfig)
}

func getConfig(namespace, appname string) (interface{}, error) {
	client, err := initClient()
	if err != nil {
		return nil, err
	}
	// FIXME(#xxx): synchronise with CustomResourceDefinitions
	proxyConfigRes := schema.GroupVersionResource{
		Group:    "networking.pisanix.io",
		Version:  "v1alpha1",
		Resource: "proxyconfigs",
	}

	proxyConfig, err := client.(dynamic.Interface).Resource(proxyConfigRes).Namespace(namespace).Get(context.Background(), appname, metav1.GetOptions{})
	if err != nil {
		return nil, err
	}
	return proxyConfig.Object["spec"], nil
}

func initClient() (interface{}, error) {
	// creates the in-cluster config
	config, err := rest.InClusterConfig()
	if err != nil {
		return nil, err
	}
	// creates the clientset
	clientset, err := dynamic.NewForConfig(config)
	if err != nil {
		return nil, err
	}

	return clientset, nil
}
