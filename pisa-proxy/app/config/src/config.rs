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
    // pub _admin: Admin,
    // pub _proxy: Option<ProxiesConfig>,
    // pub _mysql: Option<MySQLNodes>,
    // pub _shardingsphere_proxy: Option<MySQLNodes>,
    // pub _version: Option<String>, 

    pub _local: String,
    pub _config_path: String,
    pub _log_level: String,
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
    pub fn New() -> Self{
        let mut builder = PisaProxyConfigBuilder::default();
        
        let matches = Command::new("Pisa-Proxy")
        // .version(&*PisaProxyConfig::get_version())
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

    // pub fn set_admin(mut self, admin: Admin) -> Self {
    //     self._admin = admin;
    //     self
    // }

    // pub fn set_proxy(mut self, proxy: ProxiesConfig) -> Self {
    //     self._proxy = Some(proxy);
    //     self
    // }

    // pub fn set_mysql(mut self, mysql: MySQLNodes) -> Self {
    //     self._mysql = Some(mysql);
    //     self
    // }

    // pub fn set_shardingsphere_proxy(mut self, proxy: MySQLNodes) -> Self {
    //     self._shardingsphere_proxy = Some(proxy);
    //     self
    // }

    // pub fn set_version(mut self, version: String) -> Self {
    //     self._version = Some(version);
    //     self
    // }

    pub fn build(mut self) -> PisaProxyConfig {
        PisaProxyConfig::new()
        // PisaProxyConfig{
        //     admin: self._admin, 
        //     proxy: self._proxy,
        //     mysql: self._mysql,
        //     shardingsphere_proxy: self._shardingsphere_proxy,
        //     version: self._version,
        // }
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
        self._config_path = 
            env::var(PISA_PROXY_CONFIG_ENV_LOCAL_CONFIG).unwrap_or(PISA_PROXY_DEFAULT_CONFIG_PATH.to_string());

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
        // resp
    }

    // pub fn build_from_cmd(mut self) -> Self {
    //     let matches = Command::new("Pisa-Proxy")
    //     .version(&*PisaProxyConfig::get_version())
    //     .arg(Arg::new("port").short('p').long("port").help("Http port").takes_value(true))
    //     .arg(Arg::new("config").short('c').long("config").help("Config path").takes_value(true))
    //     .arg(Arg::new("loglevel").long("log-level").help("Log level").takes_value(true))
    //     .get_matches();

    //     if let Some(path) = matches.value_of("config") {
    //         config_path = path;
    //     }        
    //     if let Some(path) = matches.value_of("config") {
    //         config_path = path;
    //     }
    //     if let Some(path) = matches.value_of("config") {
    //         config_path = path;
    //     }

    // }

    pub fn build_version(mut self) -> Self {
        if !self._git_tag.is_empty() {
            self._version = format!("{}", self._git_tag);
        } else {
            self._version = format!("{}-{}", self._git_branch, self._git_commit);
        }
        self
    }

    pub fn load_config(mut self) -> PisaProxyConfig {
        let env_builder = PisaProxyConfigBuilder::default().build_from_env();
        // let env_builder = builder.build_from_env();
        // self.build_version();

        let mut config = PisaProxyConfig::new();
        // let path = self._config_path.clone();
        // let version = Some(self._version.clone());

        if env_builder._local.eq("true") {
            config = self.build_from_file(env_builder._config_path.clone());
        } else {
            config = self.build_from_http().unwrap();
        }

        config.admin.log_level = env_builder._log_level;
        config.admin.port = env_builder._port.parse::<u32>().unwrap();
        // config.version =  version; 

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

    // pub fn load_http() -> Result<PisaProxyConfig, Box<dyn std::error::Error>> {
    //     let deployed_ns = env::var("PISA_DEPLOYED_NAMESPACE")
    //         .unwrap_or(PISA_PROXY_DEFAULT_DEPLOYED_NAMESPACE.to_string());
    //     let deployed_name =
    //         env::var("PISA_DEPLOYED_NAME").unwrap_or(PISA_PROXY_DEFAULT_DEPLOYED_NAME.to_string());
    //     let pisa_svc = env::var("PISA_CONTROLLER_SERVICE")
    //         .unwrap_or(PISA_CONTROLLER_DEFAULT_SERVICE.to_string());
    //     let pisa_ns = env::var("PISA_CONTROLLER_NAMESPACE")
    //         .unwrap_or(PISA_CONTROLLER_DEFAULT_NAMESPACE.to_string());
    //     let pisa_host =
    //         env::var("PISA_CONTROLLER_HOST").unwrap_or(format!("{}.{}:8080", pisa_svc, pisa_ns));

    //     info!(
    //         "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
    //         pisa_host, deployed_ns, deployed_name
    //     );

    //     let resp = reqwest::blocking::get(format!(
    //         "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
    //         pisa_host, deployed_ns, deployed_name
    //     ))?
    //     // .json::<ProtocolConfig>()?;
    //     .json::<PisaProxyConfig>()?;
    //     Ok(resp)
    // }

    // #[tracing::instrument]
    // pub fn load_config() -> Self {
    //     let matches = Command::new("Pisa-Proxy")
    //         .version(&*PisaProxyConfig::get_version())
    //         .arg(Arg::new("port").short('p').long("port").help("Http port").takes_value(true))
    //         .arg(Arg::new("config").short('c').long("config").help("Config path").takes_value(true))
    //         .arg(Arg::new("loglevel").long("log-level").help("Log level").takes_value(true))
    //         .get_matches();
    //     // let config: ProtocolConfig;
    //     let config: PisaProxyConfig;

    //     let local = match env::var(PISA_PROXY_CONFIG_ENV_LOCAL_CONFIG) {
    //         Ok(local) => local,
    //         Err(_) => "false".to_string(),
    //     };

    //     if local.eq("true") {
    //         let mut config_path = PISA_PROXY_DEFAULT_CONFIG_PATH;

    //         if let Some(path) = matches.value_of("config") {
    //             config_path = path;
    //         }

    //         let mut file = match File::open(config_path) {
    //             Err(e) => {
    //                 eprintln!("{:?}", e);
    //                 std::process::exit(-1);
    //             }
    //             Ok(file) => file,
    //         };

    //         let mut config_str = String::new();
    //         file.read_to_string(&mut config_str).unwrap();
    //         config = toml::from_str(&config_str).unwrap();
    //     } else {
    //         config = PisaProxyConfig::load_http().unwrap();
    //     }
    //     trace!("configs: {:#?}", config);

    //     let mut pisa_config = PisaProxyConfig {
    //         admin: config.admin,
    //         proxy: Some(ProxiesConfig{
    //             config: Some(Vec::<ProxyConfig>::new()),
    //         }),
    //         mysql: Some(MySQLNodes{
    //             node: Some(Vec::<MySQLNode>::new()),
    //         }),
    //         shardingsphere_proxy: Some(MySQLNodes{
    //             node: Some(Vec::<MySQLNode>::new()),
    //         }),
    //         version: Some(String::default()),
    //     };

    //     pisa_config.version = Some(PisaProxyConfig::get_version());

    //     if let Ok(env_host) = env::var(PISA_PROXY_CONFIG_ENV_HOST) {
    //         pisa_config.admin.host = env_host;
    //     }

    //     if let Ok(env_port) = env::var(PISA_PROXY_CONFIG_ENV_PORT) {
    //         let port = env_port.to_string();
    //         pisa_config.admin.port = port.parse::<u32>().unwrap();
    //     }

    //     if let Ok(log_level) = env::var(PISA_PROXY_CONFIG_ENV_LOG_LEVEL) {
    //         pisa_config.admin.log_level = log_level;
    //     }

    //     pisa_config.proxy = config.proxy;
    //     pisa_config.mysql = config.mysql;
    //     trace!("{}", serde_json::to_string(&pisa_config).unwrap());

    //     pisa_config
    // }

    // #[tracing::instrument]
    // pub fn get_version() -> String {
    //     let mut git_tag: String = "".to_string();
    //     let mut git_commit: String = "".to_string();
    //     let mut git_branch: String = "".to_string();

    //     if let Ok(tag) = env::var(PISA_PROXY_VERSION_ENV_GIT_TAG) {
    //         git_tag = tag;
    //     };

    //     if let Ok(commit) = env::var(PISA_PROXY_VERSION_ENV_GIT_COMMIT) {
    //         git_commit = commit;
    //     };

    //     if let Ok(branch) = env::var(PISA_PROXY_VERSION_ENV_GIT_BRANCH) {
    //         git_branch = branch;
    //     };

    //     if !git_tag.is_empty() {
    //         format!("{}", git_tag)
    //     } else {
    //         format!("{}-{}", git_branch, git_commit)
    //     }
    // }
}
