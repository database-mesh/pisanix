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

package kubernetes

import (
	"context"

	"github.com/database-mesh/golang-sdk/client"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/apimachinery/pkg/util/json"
	"k8s.io/client-go/dynamic"
)

var (
	VirtualDatabaseSchema = schema.GroupVersionResource{
		Group:    "core.database-mesh.io",
		Version:  "v1alpha1",
		Resource: "virtualdatabases",
	}

	TrafficStrategySchema = schema.GroupVersionResource{
		Group:    "core.database-mesh.io",
		Version:  "v1alpha1",
		Resource: "trafficstrategies",
	}

	DatabaseEndpointSchema = schema.GroupVersionResource{
		Group:    "core.database-mesh.io",
		Version:  "v1alpha1",
		Resource: "databaseendpoints",
	}
)

func GetVirtualDatabaseWithContext(ctx context.Context, c dynamic.Interface, namespace, name string) (*client.VirtualDatabase, error) {
	raw, err := c.Resource(VirtualDatabaseSchema).Namespace(namespace).Get(ctx, name, metav1.GetOptions{})
	if err != nil {
		return nil, err
	}

	vdb := &client.VirtualDatabase{}
	data, err := json.Marshal(raw)
	if err != nil {
		return nil, err
	}

	err = json.Unmarshal(data, vdb)
	if err != nil {
		return nil, err
	}

	return vdb, nil
}

func GetTrafficStrategyListWithContext(ctx context.Context, c dynamic.Interface, namespace string) (*client.TrafficStrategyList, error) {
	tslist := &client.TrafficStrategyList{}
	raw, err := c.Resource(TrafficStrategySchema).Namespace(namespace).List(ctx, metav1.ListOptions{})
	if err != nil {
		return nil, err
	}

	data, err := json.Marshal(raw)
	if err != nil {
		return nil, err
	}

	err = json.Unmarshal(data, tslist)
	if err != nil {
		return nil, err
	}

	return tslist, nil
}

func GetDatabaseEndpointListWithContext(ctx context.Context, c dynamic.Interface, namespace string) (*client.DatabaseEndpointList, error) {
	dbeplist := &client.DatabaseEndpointList{}
	raw, err := c.Resource(DatabaseEndpointSchema).Namespace(namespace).List(ctx, metav1.ListOptions{})
	if err != nil {
		return nil, err
	}

	data, err := json.Marshal(raw)
	if err != nil {
		return nil, err
	}

	err = json.Unmarshal(data, dbeplist)
	if err != nil {
		return nil, err
	}

	return dbeplist, nil
}
