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

use clap::{value_parser, Arg, Command};
use serde::Deserialize;
use std::{env, fs::File, io::prelude::*};

use crate::env_const::*;

#[derive(Debug, Deserialize, Default)]
pub struct PisaDaemonConfig {
    pub global: Global,
    pub app: Vec<App>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Global {
    pub egress_device: String,
    pub bridge_device: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct App {
    pub name: String,
    pub service: Vec<Service>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Service {
    pub name: String,
    pub endpoints: Vec<Endpoint>,
    //TODO: qos_class need to be removed in later release
    pub qos_class: Option<QosClass>,
    pub qos_group: Option<QosGroup>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Endpoint {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QosClass {
    Guaranteed,
    Burstable,
    Besteffort,
}

impl Default for QosClass {
    fn default() -> Self {
        Self::Besteffort
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct QosGroup {
    pub rate: Option<String>,
    pub ceil: Option<String>,
}

#[derive(Default, Clone)]
pub struct PisaDaemonConfigBuilder {
    pub _http_path: String,

    pub _pisa_controller_host: String,
    pub _pisa_controller_svc: String,
    pub _pisa_controller_ns: String,

    pub _global_egress_device: String,
    pub _global_bridge_device: String,
}

impl PisaDaemonConfigBuilder {
    pub fn new() -> Self {
        PisaDaemonConfigBuilder::default()
    }

    pub fn build_from_http(
        self,
        path: String,
    ) -> Result<PisaDaemonConfig, Box<dyn std::error::Error>> {
        let resp = reqwest::blocking::get(path)?.json::<PisaDaemonConfig>()?;

        Ok(resp)
    }

    pub fn collect_from_cmd(mut self) -> Self {
        let mut matches = Command::new("Pisa-Daemon")
            .arg(
                Arg::new("global-bridge-device")
                    .long("global-bridge-device")
                    .help("Global Bridge Device")
                    .default_value(DEFAULT_PISA_DAEMON_GLOBAL_BRIDGE_DEVICE)
                    .env(ENV_PISA_DAEMON_GLOBAL_BRIDGE_DEVICE)
                    .takes_value(true),
            )
            .arg(
                Arg::new("global-egress-device")
                    .long("global-egress-device")
                    .help("Global Egress Device")
                    .default_value(DEFAULT_PISA_DAEMON_GLOBAL_EGRESS_DEVICE)
                    .env(ENV_PISA_DAEMON_GLOBAL_EGRESS_DEVICE)
                    .takes_value(true),
            )
            .arg(
                Arg::new("pisa-controller-service")
                    .long("pisa-controller-service")
                    .help("Pisa Controller Service")
                    .default_value(DEFAULT_PISA_CONTROLLER_SERVICE)
                    .env(ENV_PISA_CONTROLLER_SERVICE)
                    .takes_value(true),
            )
            .arg(
                Arg::new("pisa-controller-namespace")
                    .long("pisa-controller-namespace")
                    .help("Pisa Controller Namespace")
                    .default_value(DEFAULT_PISA_CONTROLLER_NAMESPACE)
                    .env(ENV_PISA_CONTROLLER_NAMESPACE)
                    .takes_value(true),
            )
            .arg(
                Arg::new("pisa-controller-host")
                    .long("pisa-controller-host")
                    .help("Pisa Controller Host")
                    .default_value(DEFAULT_PISA_CONTROLLER_HOST)
                    .env(ENV_PISA_CONTROLLER_HOST)
                    .takes_value(true),
            )
            .get_matches();

        self._global_bridge_device = matches
            .get_one::<String>("global-bridge-device")
            .unwrap()
            .to_string();
        self._global_egress_device = matches
            .get_one::<String>("global-egress-device")
            .unwrap()
            .to_string();

        self._pisa_controller_ns = matches
            .get_one::<String>("pisa-controller-namespace")
            .unwrap()
            .to_string();
        self._pisa_controller_svc = matches
            .get_one::<String>("pisa-controller-service")
            .unwrap()
            .to_string();
        self._pisa_controller_host = matches
            .get_one::<String>("pisa-controller-host")
            .unwrap()
            .to_string();

        if self._pisa_controller_host.is_empty() {
            self._pisa_controller_host = format!(
                "{}.{}:8080",
                self._pisa_controller_svc, self._pisa_controller_ns
            );
        }

        self
    }

    pub fn build(self) -> PisaDaemonConfig {
        let builder = PisaDaemonConfigBuilder::new();
        let http_path = format!("http://{}/daemonconfigs", self._pisa_controller_host);
        let mut config: PisaDaemonConfig = builder.build_from_http(http_path).unwrap_or_default();

        config
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_str_to_config() {
        let toml_str = r#"
        [global]
egress_device = "eth0"
bridge_device = "cni0"

[[app]]
name = "testapp"

[[app.service]]
name = "test"
qos_class = "guaranteed" # "burstable" | "besteffort"
[[app.service.endpoints]]
ip = "1.1.1.1"
port = 3306

[[app.service.endpoints]]
ip = "1.1.1.1"
port = 3307

[app.service.qos_group]
rate = "1MB"
ceil = "1MB"

"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        println!("{:?}", config);
        assert_eq!(config.app[0].name, "testapp");
        assert_eq!(
            config.app[0].service[0].qos_class,
            Some(QosClass::Guaranteed)
        );
    }
}
