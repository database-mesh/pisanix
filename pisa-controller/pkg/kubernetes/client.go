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
	"flag"
	"log"
	"os"
	"path/filepath"
	"strings"
	"sync"

	"k8s.io/client-go/dynamic"
	"k8s.io/client-go/rest"
	"k8s.io/client-go/tools/clientcmd"
	"k8s.io/client-go/util/homedir"
)

var client *KClient
var once sync.Once
var kubeConfigPath *string

func init() {
	kubeConfigPath = flag.String("kubeconfig", "", "(optional) absolute path to the kubeconfig file")
}

type ConfigBuilder struct {
	path string
}

func NewConfigBuilder() *ConfigBuilder {
	return &ConfigBuilder{
		path: "",
	}
}

func (b *ConfigBuilder) WithPath(path string) *ConfigBuilder {
	b.path = path
	return b
}

func (b *ConfigBuilder) Build() (*rest.Config, error) {
	if strings.EqualFold(b.path, "") {
		config, err := rest.InClusterConfig()
		if err != nil {
			return nil, err
		}
		return config, nil
	} else {
		config, err := clientcmd.BuildConfigFromFlags("", b.path)
		if err != nil {
			return nil, err
		}
		return config, nil
	}
}

type ClientBuilder struct {
	config *rest.Config
}

func NewClientBuilder() *ClientBuilder {
	return &ClientBuilder{
		config: &rest.Config{},
	}
}

func (b *ClientBuilder) WithKubeConfig(config *rest.Config) *ClientBuilder {
	b.config = config
	return b
}

func (b *ClientBuilder) Build() (dynamic.Interface, error) {
	clientset, err := dynamic.NewForConfig(b.config)
	if err != nil {
		return nil, err
	}
	return clientset, nil
}

func GetClient() *KClient {
	once.Do(func() {
		if *kubeConfigPath != "" {
			if _, err := os.Stat(*kubeConfigPath); os.IsNotExist(err) {
				if home := homedir.HomeDir(); home != "" {
					*kubeConfigPath = filepath.Join(home, ".kube", "config")
				}
			} 
		}

		config, err := NewConfigBuilder().WithPath(*kubeConfigPath).Build()
		if err != nil {
			log.Fatal(err)
		}
		clientset, err := NewClientBuilder().WithKubeConfig(config).Build()
		if err != nil {
			log.Fatal(err)
		}
		client = &KClient{}
		client.Client = clientset
	})

	return client
}
