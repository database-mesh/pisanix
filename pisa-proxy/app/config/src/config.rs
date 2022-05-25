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
use serde::Deserialize;
use tracing::{info, trace};

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct PisaConfig {
    pub admin: Admin,
    pub proxies: Vec<ProxyConfig>,
    pub mysql_nodes: Vec<MySQLNode>,
    pub sharding_proxy_nodes: Vec<MySQLNode>,
}

const DEFAULT_CONFIG_PATH: &str = "etc/config.toml";
const DEFAULT_PISA_CONTROLLER_SERVICE: &str = "pisa-controller";
const DEFAULT_PISA_CONTROLLER_NAMESPACE: &str = "pisa-system";
const DEFAULT_NAMESPACE: &str = "default";
const DEFAULT_NAME: &str = "default";
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
        let ns = match env::var("NAMESPACE") {
            Ok(ns) => ns,
            Err(_) => DEFAULT_NAMESPACE.to_string(),
        };

        let name = match env::var("NAME") {
            Ok(ns) => ns,
            Err(_) => DEFAULT_NAME.to_string(),
        };

        let pisa_svc = match env::var("PISA_CONTROLLER_SERVICE") {
            Ok(pisa_svc) => pisa_svc,
            Err(_) => DEFAULT_PISA_CONTROLLER_SERVICE.to_string(),
        };

        let pisa_ns = match env::var("PISA_CONTROLLER_NAMESPACE") {
            Ok(pisa_ns) => pisa_ns,
            Err(_) => DEFAULT_PISA_CONTROLLER_NAMESPACE.to_string(),
        };

        let pisa_host = match env::var("PISA_CONTROLLER_HOST") {
            Ok(pisa_host) => pisa_host,
            Err(_) => format!("{}.{}:8080", pisa_svc, pisa_ns),
        };
        println!(
            "http://{}/apis/proxy.pisanix.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
            pisa_host, ns, name
        );
        let resp = reqwest::blocking::get(format!(
            "http://{}/apis/proxy.pisanix.io/v1alpha1/namespaces/{}/proxyconfigs/{}",
            pisa_host, ns, name
        ))?
        .json::<Config>()?;
        Ok(resp)
    }

    #[tracing::instrument]
    pub fn load_config() -> Self {
        let matches = Command::new("Pisa-Proxy")
            .arg(Arg::new("port").short('p').long("port").help("Http port").takes_value(true))
            .arg(Arg::new("config").short('c').long("config").help("Config path").takes_value(true))
            .arg(Arg::new("loglevel").long("log-level").help("Log level").takes_value(true))
            .get_matches();
        let config: Config;

        let local = match env::var(ENV_LOCAL_CONFIG) {
            Ok(local) => local,
            Err(_) => "false".to_string(),
        };

        if local.eq("true") {
            // if env::var(ENV_LOCAL_CONFIG).unwrap().eq("true") {

            // }

            // if env::var(ENV_LOCAL_CONFIG).unwrap().eq("true") {
            let mut config_path = DEFAULT_CONFIG_PATH;

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

        if let Some(port) = matches.value_of("port") {
            pisa_config.admin.port = port.to_owned();
        }

        if let Some(env_port) = env::var(ENV_PORT).ok() {
            pisa_config.admin.port = env_port;
        }

        if let Some(log_level) = env::var(ENV_LOG_LEVEL).ok() {
            pisa_config.admin.log_level = log_level;
        }

        if let Some(config) = config.proxy {
            if let Some(app_config) = config.configs {
                for app in app_config {
                    pisa_config.proxies.push(app);
                }
            }
        }

        if let Some(mysql) = config.mysql {
            if let Some(mysql_nodes) = mysql.nodes {
                pisa_config.mysql_nodes = mysql_nodes;
            }
        }

        pisa_config
    }
}

pub const ENV_LOCAL_CONFIG: &str = "LOCAL_CONFIG";
pub const ENV_PISA_CONTROLLER_SERVICE: &str = "PISA_CONTROLLER_SERVICE";
pub const ENV_PORT: &str = "PORT";
pub const ENV_LOG_LEVEL: &str = "LOG_LEVEL";
