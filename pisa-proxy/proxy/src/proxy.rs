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

use std::{collections::HashMap, sync::Arc};

use endpoint::endpoint::Endpoint;
use loadbalancer::balancer::{Balancer, BalancerStrategy, LoadBalancer};
use pisa_error::error::Error;
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use crate::listener::listener::Listener;

#[derive(Debug, Deserialize)]
pub struct ProxiesConfig {
    pub configs: Option<Vec<ProxyConfig>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    #[serde(default = "default_auto_proxy_name")]
    pub name: String,
    #[serde(default = "default_auto_proxy_listen_addr")]
    pub listen_addr: String,
    #[serde(default = "default_auto_proxy_username")]
    pub username: String,
    #[serde(default = "default_auto_proxy_password")]
    pub password: String,
    #[serde(default = "default_auto_proxy_db")]
    pub db: String,
    #[serde(default = "default_auto_proxy_backend_type")]
    pub backend_type: String,
    #[serde(default = "default_auto_pool_size")]
    pub pool_size: u8,
    #[serde(default = "default_auto_strategy")]
    pub strategy: String,
    pub master_slave: Option<ProxyConfigMasterSlave>,
    pub sharding: Option<ProxyConfigSharding>,
    pub sharding_master_slave: Option<ProxyConfigShardingMasterSlave>,
    pub simple_loadbalancer: Option<ProxySimpleLoadBalancer>,
    pub plugin: Option<plugin::config::Plugin>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfigMasterSlave {
    master: Option<Vec<String>>,
    slave: Option<Vec<String>>,
    balance_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfigSharding {
    table: Option<String>,
    sharding_key: Option<String>,
    sharding_type: Option<String>,
    nodes: Option<Vec<String>>,
    shard: Option<HashMap<String, u64>>,
    defaults: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfigShardingMasterSlave {
    master: Option<Vec<String>>,
    slave: Option<Vec<String>>,
    defaults: Option<Vec<String>>,
    shard: Option<HashMap<String, u64>>,
    sharding_key: Option<String>,
    sharding_type: Option<String>,
    balance_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxySimpleLoadBalancer {
    #[serde(default = "default_auto_balancer_type")]
    pub balancer_type: String,
    pub nodes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MySQLNodes {
    pub nodes: Option<Vec<MySQLNode>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MySQLNode {
    pub name: String,
    pub db: String,
    pub user: String,
    pub password: String,
    pub addr: String,
    pub weight: i64,
}

fn default_auto_proxy_name() -> String {
    "".into()
}

fn default_auto_proxy_listen_addr() -> String {
    "".into()
}

fn default_auto_proxy_username() -> String {
    "".into()
}

fn default_auto_proxy_password() -> String {
    "".into()
}

fn default_auto_proxy_backend_type() -> String {
    "".into()
}

fn default_auto_pool_size() -> u8 {
    64
}

fn default_auto_strategy() -> String {
    "simple_balancer".into()
}

fn default_auto_proxy_db() -> String {
    "".into()
}

fn default_auto_balancer_type() -> String {
    "random".into()
}

pub struct Proxy {
    pub listener: Listener,
    pub app: ProxyConfig,
    pub backend_nodes: Vec<MySQLNode>,
}

impl Proxy {
    pub fn build_listener(&mut self) -> Result<TcpListener, std::io::Error> {
        self.listener.build_listener()
    }

    pub async fn accept(&mut self, listener: &TcpListener) -> Result<TcpStream, std::io::Error> {
        self.listener.accept(listener).await
    }

    pub fn build_loadbalancer(
        &mut self,
        nodes: Vec<String>,
    ) -> Result<Arc<Mutex<Box<dyn LoadBalancer + Send + Sync>>>, std::io::Error> {
        let mut balancer = Balancer {};

        let lb = match &self.app.simple_loadbalancer {
            Some(lb) => lb,
            None => return Err(std::io::Error::new(std::io::ErrorKind::Other, "config error")),
        };

        let mut b = balancer.build_balancer(lb.balancer_type.clone()).unwrap();
        for node in self.backend_nodes.clone() {
            match nodes.iter().find(|&x| x == node.name.as_str()) {
                Some(_) => {
                    let endpoint = Endpoint {
                        name: node.name,
                        addr: node.addr,
                        db: node.db,
                        user: node.user,
                        password: node.password,
                        weight: node.weight,
                    };
                    b.add(endpoint);
                }
                _ => continue,
            }
        }
        Ok(Arc::new(Mutex::new(b)))
    }
}
