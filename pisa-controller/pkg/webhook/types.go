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

package webhook

import (
	"os"
	"strconv"
)

var (
	pisaProxyImage, pisaControllerService, pisaControllerNamespace, pisaProxyAdminListenHost, pisaProxyLoglevel string
	pisaProxyAdminListenPort                                                                                    uint32
)

func init() {
	if pisaProxyImage = os.Getenv(EnvPisaProxyImage); pisaProxyImage == "" {
		pisaProxyImage = DefaultPisaProxyImage
	}

	if pisaControllerService = os.Getenv(EnvPisaControllerService); pisaControllerService == "" {
		pisaControllerService = DefaultPisaControllerService
	}
	if pisaControllerNamespace = os.Getenv(EnvPisaControllerNamespace); pisaControllerNamespace == "" {
		pisaControllerNamespace = DefaultPisaControllerNamespace
	}

	if host := os.Getenv(EnvPisaProxyAdminListenHost); host == "" {
		pisaProxyAdminListenHost = DefaultPisaProxyAdminListenHost
	} else {
		pisaProxyAdminListenHost = host
	}

	if port, err := strconv.Atoi(os.Getenv(EnvPisaProxyAdminListenPort)); port <= 0 || err != nil {
		pisaProxyAdminListenPort = DefaultPisaProxyAdminListenPort
	} else {
		pisaProxyAdminListenPort = uint32(port)
	}

	if lv := os.Getenv(EnvPisaProxyLoglevel); lv == "" {
		pisaProxyLoglevel = DefaultPisaProxyLoglevel
	} else {
		pisaProxyLoglevel = lv
	}
}

const (
	pisaProxyContainerName = "pisa-proxy"

	EnvPisaProxyAdminListenHost = "PISA_PROXY_ADMIN_LISTEN_HOST"
	EnvPisaProxyAdminListenPort = "PISA_PROXY_ADMIN_LISTEN_PORT"
	EnvPisaProxyLoglevel        = "PISA_PROXY_ADMIN_LOG_LEVEL"
	EnvPisaProxyImage           = "PISA_PROXY_IMAGE"
	EnvPisaControllerService    = "PISA_CONTROLLER_SERVIE"
	EnvPisaControllerNamespace  = "PISA_CONTROLLER_NAMESPACE"

	DefaultPisaProxyAdminListenHost = "0.0.0.0"
	DefaultPisaProxyAdminListenPort = 5591
	DefaultPisaProxyLoglevel        = "INFO"
	DefaultPisaProxyImage           = "pisanixio/proxy:latest"
	DefaultPisaControllerService    = "default"
	DefaultPisaControllerNamespace  = "default"
)

const (
	podsSidecarPatch = `[
		{
			"op":"add", 
			"path":"/spec/containers/-",
			"value":{
				"image":"%v",
				"name":"%s",
				"args": ["sidecar"],
				"ports": [
					{
						"containerPort": %d,
						"name": "pisa-admin",
						"protocol": "TCP"
					}	
				],
				"resources":{},
				"env": [
					{
						"name": "PISA_CONTROLLER_SERVICE",
						"value": "%s"
					},{
						"name": "PISA_CONTROLLER_NAMESPACE",
						"value": "%s"
					},{
						"name": "PISA_DEPLOYED_NAMESPACE",
						"valueFrom": {
                            "fieldRef": {
                                "apiVersion": "v1",
                                "fieldPath": "metadata.namespace"
                            }
                        }
					},{
						"name": "PISA_DEPLOYED_NAME",
						"value": "%s"
					},{
						"name": "PISA_PROXY_ADMIN_LISTEN_HOST",
						"value": "%s"
					},{
						"name": "PISA_PROXY_ADMIN_LISTEN_PORT",
						"value": "%d"
					},{
						"name": "PISA_PROXY_ADMIN_LOG_LEVEL",
						"value": "%s"
					}
				]
			}
		}
	]`
)
