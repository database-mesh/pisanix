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

// var (
// 	pisaProxyImage, pisaControllerService, pisaControllerNamespace, pisaProxyAdminListenHost, pisaProxyLoglevel string
// 	pisaProxyAdminListenPort                                                                                    uint32
// )

const (
	EnvPisaProxyAdminListenHost   = "PISA_PROXY_ADMIN_LISTEN_HOST"
	EnvPisaProxyAdminListenPort   = "PISA_PROXY_ADMIN_LISTEN_PORT"
	EnvPisaProxyAdminLoglevel     = "PISA_PROXY_ADMIN_LOG_LEVEL"
	EnvPisaProxyImage             = "PISA_PROXY_IMAGE"
	EnvPisaProxyDeployedNamespace = "PISA_DEPLOYED_NAMESPACE"
	EnvPisaProxyDeployedName      = "PISA_DEPLOYED_NAME"
	EnvPisaControllerService      = "PISA_CONTROLLER_SERVICE"
	EnvPisaControllerNamespace    = "PISA_CONTROLLER_NAMESPACE"

	DefaultPisaProxyAdminListenHost = "0.0.0.0"
	DefaultPisaProxyAdminListenPort = 5591
	DefaultPisaProxyAdminLoglevel   = "INFO"
	DefaultPisaProxyImage           = "pisanixio/proxy:latest"
	DefaultPisaProxyContainerName   = "pisa-proxy"
	DefaultPisaControllerService    = "default"
	DefaultPisaControllerNamespace  = "default"
)
