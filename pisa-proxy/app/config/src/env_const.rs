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

pub const DEFAULT_LOCAL_CONFIG: &str = "etc/config.toml";
pub const DEFAULT_PISA_PROXY_ADMIN_LISTEN_HOST: &str = "";
pub const DEFAULT_PISA_PROXY_ADMIN_LISTEN_PORT: &str = "";
pub const DEFAULT_PISA_PROXY_ADMIN_LOG_LEVEL: &str = "";
pub const DEFAULT_PISA_CONTROLLER_HOST: &str = "";
pub const DEFAULT_PISA_CONTROLLER_NAMESPACE: &str = "pisa-system";
pub const DEFAULT_PISA_CONTROLLER_SERVICE: &str = "pisa-controller";
pub const DEFAULT_PISA_DEPLOYED_NAMESPACE: &str = "default";
pub const DEFAULT_PISA_DEPLOYED_NAME: &str = "default";

pub const ENV_PISA_PROXY_ADMIN_LISTEN_HOST: &str = "PISA_PROXY_ADMIN_LISTEN_HOST";
pub const ENV_PISA_PROXY_ADMIN_LISTEN_PORT: &str = "PISA_PROXY_ADMIN_LISTEN_PORT";
pub const ENV_PISA_PROXY_ADMIN_LOG_LEVEL: &str = "PISA_PROXY_ADMIN_LOG_LEVEL";
pub const ENV_PISA_CONTROLLER_HOST: &str = "PISA_CONTROLLER_HOST";
pub const ENV_PISA_CONTROLLER_NAMESPACE: &str = "PISA_CONTROLLER_NAMESPACE";
pub const ENV_PISA_CONTROLLER_SERVICE: &str = "PISA_CONTROLLER_SERVICE";
pub const ENV_PISA_DEPLOYED_NAMESPACE: &str = "PISA_DEPLOYED_NAMESPACE";
pub const ENV_PISA_DEPLOYED_NAME: &str = "PISA_DEPLOYED_NAME";

pub const ENV_GIT_TAG: &str = "GIT_TAG";
pub const ENV_GIT_BRANCH: &str = "GIT_BRANCH";
pub const ENV_GIT_COMMIT: &str = "GIT_COMMIT";
pub const ENV_VERGEN_GIT_COMMIT: &str = "VERGEN_GIT_COMMIT";
pub const ENV_VERGEN_GIT_BRANCH: &str = "VERGEN_GIT_BRANCH";
