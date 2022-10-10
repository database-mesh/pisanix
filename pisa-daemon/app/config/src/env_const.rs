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

pub const DEFAULT_PISA_CONTROLLER_HOST: &str = "";
pub const DEFAULT_PISA_CONTROLLER_NAMESPACE: &str = "pisa-system";
pub const DEFAULT_PISA_CONTROLLER_SERVICE: &str = "pisa-controller";
pub const DEFAULT_PISA_DAEMON_GLOBAL_BRIDGE_DEVICE: &str = "docker0";
pub const DEFAULT_PISA_DAEMON_GLOBAL_EGRESS_DEVICE: &str = "eth0";

pub const ENV_PISA_CONTROLLER_HOST: &str = "PISA_CONTROLLER_HOST";
pub const ENV_PISA_CONTROLLER_NAMESPACE: &str = "PISA_CONTROLLER_NAMESPACE";
pub const ENV_PISA_CONTROLLER_SERVICE: &str = "PISA_CONTROLLER_SERVICE";
pub const ENV_PISA_DAEMON_GLOBAL_BRIDGE_DEVICE: &str = "PISA_DAEMON_GLOBAL_BRIDGE_DEVICE";
pub const ENV_PISA_DAEMON_GLOBAL_EGRESS_DEVICE: &str = "PISA_DAEMON_GLOBAL_EGRESS_DEVICE";
