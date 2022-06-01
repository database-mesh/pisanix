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

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub admin: Admin,
    pub mysql: Option<MySQLNodes>,
    pub proxy: Option<ProxiesConfig>,
}

#[derive(Debug, Clone)]
pub enum Node {
    MySQL(Vec<MySQLNode>),
    ShardingProxy(Vec<MySQLNode>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PisaConfig {
    pub admin: Admin,
    pub proxies: Vec<ProxyConfig>,
    pub mysql_nodes: Vec<MySQLNode>,
    pub sharding_proxy_nodes: Vec<MySQLNode>,
}

const PISA_CONTROLLER_DEFAULT_SERVICE: &str = "pisa-controller";
const PISA_CONTROLLER_DEFAULT_NAMESPACE: &str = "pisa-system";

const PISA_PROXY_DEFAULT_DEPLOYED_NAME: &str = "default";
const PISA_PROXY_DEFAULT_DEPLOYED_NAMESPACE: &str = "default";
const PISA_PROXY_DEFAULT_CONFIG_PATH: &str = "etc/config.toml";

const PISA_PROXY_CONFIG_ENV_LOCAL_CONFIG: &str = "LOCAL_CONFIG";
const PISA_PROXY_CONFIG_ENV_PORT: &str = "PORT";
const PISA_PROXY_CONFIG_ENV_LOG_LEVEL: &str = "LOG_LEVEL";

const PISA_PROXY_VERSION_ENV_GIT_TAG: &str = "GIT_TAG";
const PISA_PROXY_VERSION_ENV_GIT_BRANCH: &str = "GIT_BRANCH";
const PISA_PROXY_VERSION_ENV_GIT_COMMIT: &str = "GIT_COMMIT";

impl PisaConfig {
    pub fn get_proxies(&self) -> &Vec<ProxyConfig> {
        &self.proxies
    }

    pub fn get_mysql_nodes(&self) -> &Vec<MySQLNode> {
        &self.mysql_nodes
    }

    pub fn get_admin(&self) -> &Admin {
        &self.admin
    }

    pub fn load_http() -> Result<Config, Box<dyn std::error::Error>> {
        let deployed_ns = env::var("PISA_DEPLOYED_NAMESPACE")
            .unwrap_or(PISA_PROXY_DEFAULT_DEPLOYED_NAMESPACE.to_string());
        let deployed_name =
            env::var("PISA_DEPLOYED_NAME").unwrap_or(PISA_PROXY_DEFAULT_DEPLOYED_NAME.to_string());
        let pisa_svc = env::var("PISA_CONTROLLER_SERVICE")
            .unwrap_or(PISA_CONTROLLER_DEFAULT_SERVICE.to_string());
        let pisa_ns = env::var("PISA_CONTROLLER_NAMESPACE")
            .unwrap_or(PISA_CONTROLLER_DEFAULT_NAMESPACE.to_string());
        let pisa_host =
            env::var("PISA_CONTROLLER_HOST").unwrap_or(format!("{}.{}:8080", pisa_svc, pisa_ns));

        info!(
            "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
            pisa_host, deployed_ns, deployed_name
        );

        let resp = reqwest::blocking::get(format!(
            "http://{}/apis/configs.database-mesh.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
            pisa_host, deployed_ns, deployed_name
        ))?
        .json::<Config>()?;
        Ok(resp)
    }

    #[tracing::instrument]
    pub fn load_config() -> Self {
        let matches = Command::new("Pisa-Proxy")
            .version(&*PisaConfig::get_version())
            .arg(Arg::new("port").short('p').long("port").help("Http port").takes_value(true))
            .arg(Arg::new("config").short('c').long("config").help("Config path").takes_value(true))
            .arg(Arg::new("loglevel").long("log-level").help("Log level").takes_value(true))
            .get_matches();
        let config: Config;

        let local = match env::var(PISA_PROXY_CONFIG_ENV_LOCAL_CONFIG) {
            Ok(local) => local,
            Err(_) => "false".to_string(),
        };

        if local.eq("true") {
            let mut config_path = PISA_PROXY_DEFAULT_CONFIG_PATH;

            if let Some(path) = matches.value_of("config") {
                config_path = path;
            }

            let mut file = match File::open(config_path) {
                Err(e) => {
                    eprintln!("{:?}", e);
                    std::process::exit(-1);
                }
                Ok(file) => file,
            };

            let mut config_str = String::new();
            file.read_to_string(&mut config_str).unwrap();
            config = toml::from_str(&config_str).unwrap();
        } else {
            config = PisaConfig::load_http().unwrap();
        }
        trace!("configs: {:#?}", config);

        let mut pisa_config = PisaConfig {
            admin: config.admin,
            proxies: vec![],
            mysql_nodes: vec![],
            sharding_proxy_nodes: vec![],
        };

        if let Ok(env_port) = env::var(PISA_PROXY_CONFIG_ENV_PORT) {
            pisa_config.admin.port = env_port;
        }

        if let Ok(log_level) = env::var(PISA_PROXY_CONFIG_ENV_LOG_LEVEL) {
            pisa_config.admin.log_level = log_level;
        }

        if let Some(config) = config.proxy {
            if let Some(app_config) = config.config {
                for app in app_config {
                    pisa_config.proxies.push(app);
                }
            }
        }

        if let Some(mysql) = config.mysql {
            if let Some(mysql_nodes) = mysql.node {
                pisa_config.mysql_nodes = mysql_nodes;
            }
        }

        trace!("{}", serde_json::to_string(&pisa_config).unwrap());

        pisa_config
    }

    #[tracing::instrument]
    pub fn get_version() -> String {
        let mut git_tag: String = "".to_string();
        let mut git_commit: String = "".to_string();
        let mut git_branch: String = "".to_string();

        if let Ok(tag) = env::var(PISA_PROXY_VERSION_ENV_GIT_TAG) {
            git_tag = tag;
        };

        if let Ok(commit) = env::var(PISA_PROXY_VERSION_ENV_GIT_COMMIT) {
            git_commit = commit;
        };

        if let Ok(branch) = env::var(PISA_PROXY_VERSION_ENV_GIT_BRANCH) {
            git_branch = branch;
        };

        if !git_tag.is_empty() {
            format!("{:?}", git_tag)
        } else {
            format!("{:?}-{:?}", git_branch, git_commit)
        }
    }
}
