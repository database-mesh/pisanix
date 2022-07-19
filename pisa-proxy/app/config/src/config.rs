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

#![allow(dead_code)]
use std::{env, fs::File, io::prelude::*};

use api::config::Admin;
use clap::{value_parser, Arg, Command};
use proxy::proxy::{MySQLNode, MySQLNodes, ProxiesConfig, ProxyConfig};
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::env_const::*;

#[derive(Default, Clone)]
pub struct PisaProxyConfigBuilder {
    pub _local: bool,
    pub _config_path: String,
    pub _http_path: String,

    pub _log_level: String,
    pub _host: String,
    pub _port: String,
    pub _version: String,

    pub _deployed_ns: String,
    pub _deployed_name: String,
    pub _pisa_svc: String,
    pub _pisa_ns: String,
    pub _pisa_host: String,

    pub _git_tag: String,
    pub _git_commit: String,
    pub _git_branch: String,
}

impl PisaProxyConfigBuilder {
    pub fn new() -> Self {
        PisaProxyConfigBuilder::default()
    }

    pub fn build_from_file(self, path: String) -> PisaProxyConfig {
        let config: PisaProxyConfig;
        let mut file = match File::open(path) {
            Err(e) => {
                eprintln!("{:?}", e);
                std::process::exit(-1);
            }
            Ok(file) => file,
        };

        let mut config_str = String::new();
        file.read_to_string(&mut config_str).unwrap();
        config = toml::from_str(&config_str).unwrap();
        config
    }

    pub fn build_from_http(
        self,
        path: String,
    ) -> Result<PisaProxyConfig, Box<dyn std::error::Error>> {
        let resp = reqwest::blocking::get(path)?.json::<PisaProxyConfig>()?;

        Ok(resp)
    }

    pub fn collect_from_cmd(mut self) -> Self {
        let mut matches = Command::new("Pisa-Proxy")
            .subcommand(
                Command::new("sidecar")
                    .about("used for sidecar mode")
                    .arg(
                        Arg::new("pisa-controller-host")
                            .long("pisa-controller-host")
                            .help("Pisa Controller Host")
                            .default_value(DEFAULT_PISA_CONTROLLER_HOST)
                            .env(ENV_PISA_CONTROLLER_HOST)
                            .takes_value(true),
                    )
                    .arg(
                        Arg::new("pisa-deployed-namespace")
                            .long("pisa-deployed-namespace")
                            .help("Namespace")
                            .default_value(DEFAULT_PISA_DEPLOYED_NAMESPACE)
                            .env(ENV_PISA_DEPLOYED_NAMESPACE)
                            .takes_value(true),
                    )
                    .arg(
                        Arg::new("pisa-deployed-name")
                            .long("pisa-deployed-name")
                            .help("Name")
                            .default_value(DEFAULT_PISA_DEPLOYED_NAME)
                            .env(ENV_PISA_DEPLOYED_NAME)
                            .takes_value(true),
                    ),
            )
            .subcommand(
                Command::new("daemon").about("used for standalone mode").arg(
                    Arg::new("config")
                        .short('c')
                        .long("config")
                        .help("Config path")
                        .default_value(DEFAULT_LOCAL_CONFIG)
                        .takes_value(true),
                ),
            )
            .version(PisaProxyConfigBuilder::default().build_version()._version.as_str())
            .arg(
                Arg::new("host")
                    .short('h')
                    .long("host")
                    .help("Http host")
                    .default_value(DEFAULT_PISA_PROXY_ADMIN_LISTEN_HOST)
                    .value_parser(value_parser!(String))
                    .env(ENV_PISA_PROXY_ADMIN_LISTEN_HOST)
                    .takes_value(true),
            )
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .help("Http port")
                    .default_value(DEFAULT_PISA_PROXY_ADMIN_LISTEN_PORT)
                    .value_parser(value_parser!(String))
                    .env(ENV_PISA_PROXY_ADMIN_LISTEN_PORT)
                    .takes_value(true),
            )
            .arg(
                Arg::new("loglevel")
                    .long("log-level")
                    .help("Log level")
                    .default_value(DEFAULT_PISA_PROXY_ADMIN_LOG_LEVEL)
                    .value_parser(value_parser!(String))
                    .env(ENV_PISA_PROXY_ADMIN_LOG_LEVEL)
                    .takes_value(true),
            )
            .subcommand_required(true)
            .get_matches();

        self._host = matches.get_one::<String>("host").unwrap().to_string();
        self._port = matches.get_one::<String>("port").unwrap().to_string();
        self._log_level = matches.get_one::<String>("loglevel").unwrap().to_string();

        let (name, cmd) = matches.remove_subcommand().expect("required");
        match (name.as_str(), cmd) {
            ("daemon", cmd) => {
                self._config_path = cmd.value_of("config").unwrap().to_string();
                self._local = true;
            }
            ("sidecar", cmd) => {
                self._pisa_host = cmd.value_of("pisa-controller-host").unwrap().to_string();
                self._deployed_ns = cmd.value_of("pisa-deployed-namespace").unwrap().to_string();
                self._deployed_name = cmd.value_of("pisa-deployed-name").unwrap().to_string();
            }
            (name, _) => {
                unimplemented!("this command '{}' is not supported", name);
            }
        }

        self
    }

    pub fn build_version(mut self) -> Self {
        self._git_tag = env::var(ENV_GIT_TAG).unwrap_or("".to_string());
        self._git_commit = env::var(ENV_GIT_COMMIT).unwrap_or("".to_string());
        self._git_branch = env::var(ENV_GIT_BRANCH).unwrap_or("".to_string());

        if !self._git_tag.is_empty() {
            self._version = format!("{}", self._git_tag);
        } else {
            self._version = format!("{}-{}", self._git_branch, self._git_commit);
        }
        self
    }

    pub fn build(self) -> PisaProxyConfig {
        let builder = PisaProxyConfigBuilder::new();
        let mut config: PisaProxyConfig = if self._local {
            builder.build_from_file(self._config_path)
        } else {
            let http_path = format!(
                "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
                self._pisa_host, self._deployed_ns, self._deployed_name
            );
            builder.build_from_http(http_path).unwrap()
        };

        if !self._log_level.is_empty() {
            config.admin.log_level = self._log_level;
        }
        if !self._port.is_empty() {
            config.admin.port = self._port.parse::<u32>().unwrap();
        }
        if !self._host.is_empty() {
            config.admin.host = self._host;
        }

        config.version = Some(self._version);

        trace!("configs: {:#?}", config);
        config
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PisaProxyConfig {
    pub admin: Admin,
    pub proxy: Option<ProxiesConfig>,
    pub mysql: Option<MySQLNodes>,
    pub shardingsphere_proxy: Option<MySQLNodes>,
    pub version: Option<String>,
}

impl PisaProxyConfig {
    pub fn new() -> Self {
        PisaProxyConfig::default()
    }
    pub fn get_proxy(&self) -> &Vec<ProxyConfig> {
        &self.proxy.as_ref().unwrap().config.as_ref().unwrap()
    }

    pub fn get_mysql(&self) -> &Vec<MySQLNode> {
        &self.mysql.as_ref().unwrap().node.as_ref().unwrap()
    }

    pub fn get_admin(&self) -> &Admin {
        &self.admin
    }

    pub fn get_version(&self) -> &String {
        &self.version.as_ref().unwrap()
    }

    pub fn get_shardingsphere_proxy(&self) -> &Vec<MySQLNode> {
        &self.shardingsphere_proxy.as_ref().unwrap().node.as_ref().unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_build_from_http() {
        let mut builder = PisaProxyConfigBuilder::new();
        builder._pisa_host = "localhost:8080".to_string();
        builder._deployed_ns = "demotest".to_string();
        builder._deployed_name = "catalogue".to_string();
        let http_path = format!(
            "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
            builder._pisa_host, builder._deployed_ns, builder._deployed_name
        );
        let config: PisaProxyConfig = builder.build_from_http(http_path).unwrap();
        assert_eq!(config.admin.host, "0.0.0.0");
    }

    #[test]
    fn test_build_from_file() {
        let mut builder = PisaProxyConfigBuilder::new();
        builder._config_path = "absolute_path".to_string();
        let path = builder._config_path.clone();
        let config: PisaProxyConfig = builder.build_from_file(path);
        assert_eq!(config.admin.host, "0.0.0.0");
    }
}
