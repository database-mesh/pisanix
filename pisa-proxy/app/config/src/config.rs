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
use clap::{Arg, Command};
use proxy::proxy::{MySQLNode, MySQLNodes, ProxiesConfig, ProxyConfig};
use serde::{Deserialize, Serialize};
use tracing::trace;

#[derive(Default, Clone)]
pub struct PisaProxyConfigBuilder {
    pub _local: String,
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

const PISA_CONTROLLER_DEFAULT_SERVICE: &str = "pisa-controller";
const PISA_CONTROLLER_DEFAULT_NAMESPACE: &str = "pisa-system";

const PISA_PROXY_DEFAULT_DEPLOYED_NAME: &str = "default";
const PISA_PROXY_DEFAULT_DEPLOYED_NAMESPACE: &str = "default";
const PISA_PROXY_DEFAULT_CONFIG_PATH: &str = "etc/config.toml";

const PISA_PROXY_CONFIG_ENV_LOCAL_CONFIG: &str = "LOCAL_CONFIG";
const PISA_PROXY_CONFIG_ENV_HOST: &str = "PISA_PROXY_ADMIN_LISTEN_HOST";
const PISA_PROXY_CONFIG_ENV_PORT: &str = "PISA_PROXY_ADMIN_LISTEN_PORT";
const PISA_PROXY_CONFIG_ENV_LOG_LEVEL: &str = "PISA_PROXY_ADMIN_LOG_LEVEL";

const PISA_PROXY_VERSION_ENV_GIT_TAG: &str = "GIT_TAG";
const PISA_PROXY_VERSION_ENV_GIT_BRANCH: &str = "GIT_BRANCH";
const PISA_PROXY_VERSION_ENV_GIT_COMMIT: &str = "GIT_COMMIT";

impl PisaProxyConfigBuilder {
    pub fn new() -> Self {
        PisaProxyConfigBuilder::default()
    }

    pub fn build(self) -> PisaProxyConfig {
        let mut config = PisaProxyConfig::new();
        config.admin.log_level = self._log_level;
        config.admin.host = self._host;
        config.admin.port = self._port.parse::<u32>().unwrap();
        config.version = Some(self._version);
        config
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

    pub fn build_from_cmd(mut self) -> Self {
        let matches = Command::new("Pisa-Proxy")
            .subcommand(Command::new("sidecar").about("used for sidecar mode").arg(
                Arg::new("pisa-controller-host")
                    .long("pisa-controller-host")
                    .help("Pisa Controller Host")
                    .default_value("localhost:8080")
                    .takes_value(true),
            ).arg(
                Arg::new("pisa-proxy-target-namespace")
                    .long("pisa-proxy-target-namespace")
                    .help("Namespace")
                    .default_value("default")
                    .takes_value(true),
            ).arg(
                Arg::new("pisa-proxy-target-name")
                    .long("pisa-proxy-target-name")
                    .help("Name")
                    .default_value("default")
                    .takes_value(true),
            ))
            .subcommand(Command::new("daemon").about("used for standalone mode").arg(
                Arg::new("config")
                    .short('c')
                    .long("config")
                    .help("Config path")
                    .default_value(PISA_PROXY_CONFIG_ENV_LOCAL_CONFIG)
                    .takes_value(true),
            ))
            .version(
                PisaProxyConfigBuilder::default()
                    .build_from_env()
                    .build_version()
                    ._version
                    .as_str(),
            )
            .arg(
                Arg::new("port")
                    .short('p')
                    .long("port")
                    .help("Http port")
                    .default_value("5591")
                    .takes_value(true),
            )
            .arg(
                Arg::new("loglevel")
                    .long("log-level")
                    .help("Log level")
                    .default_value("INFO")
                    .takes_value(true),
            )
            .get_matches();

        if let Some(port) = matches.value_of("port") {
            self._port = port.to_string();
        }
        if let Some(loglevel) = matches.value_of("loglevel") {
            self._log_level = loglevel.to_string();
        }

        match matches.subcommand_matches("daemon") {
            Some(cmd) => {
                self._config_path = cmd.value_of("config").unwrap().to_string();
                self._local = "true".to_string();
            }
            None => {}
        }

        match matches.subcommand_matches("sidecar") {
            Some(cmd) => {
                self._pisa_host = cmd.value_of("pisa-controller-host").unwrap().to_string(); 
                self._deployed_ns = cmd.value_of("pisa-proxy-target-namespace").unwrap().to_string(); 
                self._deployed_name = cmd.value_of("pisa-proxy-target-name").unwrap().to_string();
                self._http_path = format!(
                    "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
                    self._pisa_host, self._deployed_ns, self._deployed_name
                );
            }
            None => {}
        }

        self
    }

    pub fn build_from_env(mut self) -> Self {
        self._deployed_ns = env::var("PISA_DEPLOYED_NAMESPACE")
            .unwrap_or(PISA_PROXY_DEFAULT_DEPLOYED_NAMESPACE.to_string());
        self._deployed_name =
            env::var("PISA_DEPLOYED_NAME").unwrap_or(PISA_PROXY_DEFAULT_DEPLOYED_NAME.to_string());
        self._pisa_svc = env::var("PISA_CONTROLLER_SERVICE")
            .unwrap_or(PISA_CONTROLLER_DEFAULT_SERVICE.to_string());
        self._pisa_ns = env::var("PISA_CONTROLLER_NAMESPACE")
            .unwrap_or(PISA_CONTROLLER_DEFAULT_NAMESPACE.to_string());
        self._pisa_host = env::var("PISA_CONTROLLER_HOST")
            .unwrap_or(format!("{}.{}:8080", self._pisa_svc, self._pisa_ns));
        self._local = env::var(PISA_PROXY_CONFIG_ENV_LOCAL_CONFIG).unwrap_or("false".to_string());
        self._host = env::var(PISA_PROXY_CONFIG_ENV_HOST).unwrap_or("0.0.0.0".to_string());
        self._port = env::var(PISA_PROXY_CONFIG_ENV_PORT).unwrap_or("5591".to_string());
        self._log_level = env::var(PISA_PROXY_CONFIG_ENV_LOG_LEVEL).unwrap_or("".to_string());
        self._git_tag = env::var(PISA_PROXY_VERSION_ENV_GIT_TAG).unwrap_or("".to_string());
        self._git_commit = env::var(PISA_PROXY_VERSION_ENV_GIT_COMMIT).unwrap_or("".to_string());
        self._git_branch = env::var(PISA_PROXY_VERSION_ENV_GIT_BRANCH).unwrap_or("".to_string());
        self._http_path = format!(
            "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
            self._pisa_host, self._deployed_ns, self._deployed_name
        );

        self
    }

    pub fn build_version(mut self) -> Self {
        if !self._git_tag.is_empty() {
            self._version = format!("{}", self._git_tag);
        } else {
            self._version = format!("{}-{}", self._git_branch, self._git_commit);
        }
        self
    }

    pub fn load_config(self) -> PisaProxyConfig {
        let cmd_builder = PisaProxyConfigBuilder::default().build_from_cmd();
        let config_path = cmd_builder._config_path.clone();
        let local = cmd_builder._local.clone();
        let http_path = cmd_builder._http_path.clone();

        let cmd_config = cmd_builder.build();

        let env_builder = PisaProxyConfigBuilder::default().build_from_env();
        // let is_local_config = env_builder._local.clone();
        // let http_path = env_builder._http_path.clone();
        let env_config = env_builder.build_version().build();

        let mut config: PisaProxyConfig;
        // if is_local_config.eq("true") {
        if local.eq("true") {
            config = self.build_from_file(config_path);
        } else {
            config = self.build_from_http(http_path).unwrap();
        }

        if cmd_config.admin.log_level.len() != 0 {
            config.admin.log_level = cmd_config.admin.log_level;
        }
        if cmd_config.admin.port != 0 {
            config.admin.port = cmd_config.admin.port;
        }

        if env_config.admin.log_level.len() != 0 {
            config.admin.log_level = env_config.admin.log_level;
        }
        if env_config.admin.port != 0 {
            config.admin.port = env_config.admin.port;
        }

        config.version = env_config.version;

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
