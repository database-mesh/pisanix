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
use tracing::{info, trace};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PisaProxyConfig {
    pub admin: Admin,
    pub proxy: Option<ProxiesConfig>,
    pub mysql: Option<MySQLNodes>,
    pub shardingsphere_proxy: Option<MySQLNodes>,
    pub version: Option<String>,
}

#[derive(Default, Clone)]
pub struct PisaProxyConfigBuilder {
    pub _admin: Admin,
    pub _proxy: Option<ProxiesConfig>,
    pub _mysql: Option<MySQLNodes>,
    pub _shardingsphere_proxy: Option<MySQLNodes>,
    pub _version: Option<String>, 

    pub _local: String,
    pub _config_path: String,
    pub _log_level: String,
    pub _port: String,

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
    pub fn new() -> Self{
        PisaProxyConfigBuilder::default()
    }

    pub fn build(mut self) -> PisaProxyConfig {
        PisaProxyConfig{
            admin: self._admin, 
            proxy: self._proxy,
            mysql: self._mysql,
            shardingsphere_proxy: self._shardingsphere_proxy,
            version: self._version,
        }
    }

    pub fn build_from_env(mut self) -> Self {
        self._deployed_ns = 
            env::var("PISA_DEPLOYED_NAMESPACE").unwrap_or(PISA_PROXY_DEFAULT_DEPLOYED_NAMESPACE.to_string());
        self._deployed_name =
            env::var("PISA_DEPLOYED_NAME").unwrap_or(PISA_PROXY_DEFAULT_DEPLOYED_NAME.to_string());
        self._pisa_svc = 
            env::var("PISA_CONTROLLER_SERVICE").unwrap_or(PISA_CONTROLLER_DEFAULT_SERVICE.to_string());
        self._pisa_ns = 
            env::var("PISA_CONTROLLER_NAMESPACE").unwrap_or(PISA_CONTROLLER_DEFAULT_NAMESPACE.to_string());
        self._pisa_host  =
            env::var("PISA_CONTROLLER_HOST").unwrap_or(format!("{}.{}:8080", self._pisa_svc, self._pisa_ns));
        self._local =     
            env::var(PISA_PROXY_CONFIG_ENV_LOCAL_CONFIG).unwrap_or("false".to_string());
        self._git_tag =     
            env::var(PISA_PROXY_VERSION_ENV_GIT_TAG).unwrap_or("".to_string());
        self._git_commit =     
            env::var(PISA_PROXY_VERSION_ENV_GIT_COMMIT).unwrap_or("".to_string());
        self._git_branch =     
            env::var(PISA_PROXY_VERSION_ENV_GIT_BRANCH).unwrap_or("".to_string());

        self
    }

    pub fn build_from_file(self, path: String) -> PisaProxyConfig {
        let mut config = PisaProxyConfig::new();
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

    pub fn build_from_http(self) -> Result<PisaProxyConfig, Box<dyn std::error::Error>> {
        info!(
            "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
                self._pisa_host, self._deployed_ns, self._deployed_name
        ); 
        let resp = reqwest::blocking::get(format!(
            "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
            self._pisa_host, self._deployed_ns, self._deployed_name
        ))?
        .json::<PisaProxyConfig>()?;

        Ok(resp)
    }

    pub fn build_from_cmd(self) -> Self {
        let mut builder = PisaProxyConfigBuilder::default();
        
        let matches = Command::new("Pisa-Proxy")
        .version(PisaProxyConfigBuilder::default().build_from_env().build_version()._version.unwrap().as_str())
        .arg(Arg::new("port").short('p').long("port").help("Http port").takes_value(true))
        .arg(Arg::new("config").short('c').long("config").help("Config path").takes_value(true))
        .arg(Arg::new("loglevel").long("log-level").help("Log level").takes_value(true))
        .get_matches();

        if let Some(port) = matches.value_of("port") {
            builder._port = port.to_string();
        }
        if let Some(path) = matches.value_of("config") {
            builder._config_path = path.to_string();
        }
        if let Some(loglevel) = matches.value_of("loglevel") {
            builder._log_level = loglevel.to_string();
        }
        builder
    }

    pub fn build_version(mut self) -> Self {
        if !self._git_tag.is_empty() {
            self._version = Some(format!("{}", self._git_tag));
        } else {
            self._version = Some(format!("{}-{}", self._git_branch, self._git_commit));
        }
        self
    }

    pub fn load_config(mut self) -> PisaProxyConfig {
        let cmd_builder = PisaProxyConfigBuilder::default().build_from_cmd();
        let config_path = cmd_builder._config_path.clone();
        let cmd_config = cmd_builder.build();

        let env_builder = PisaProxyConfigBuilder::default().build_from_env();
        let is_local_config = env_builder._local.clone();
        let env_config = env_builder.build_version().build();

        let mut config = PisaProxyConfig::new();
        if is_local_config.eq("true") {
            config = self.build_from_file(config_path);
        } else {
            config = self.build_from_http().unwrap();
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

        // TODO: need fix the log level
        trace!("configs: {:#?}", config);
        config
    }
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

impl PisaProxyConfig {
    pub fn new() -> Self {
        PisaProxyConfig::default()
    }
    pub fn get_proxies(&self) -> &Vec<ProxyConfig> {
        &self.proxy.as_ref().unwrap().config.as_ref().unwrap()
    }

    pub fn get_mysql_nodes(&self) -> &Vec<MySQLNode> {
        &self.mysql.as_ref().unwrap().node.as_ref().unwrap()
    }

    pub fn get_admin(&self) -> &Admin {
        &self.admin
    }
}
