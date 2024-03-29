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

use config::config::PisaProxyConfig;
use proxy::{
    factory::{Proxy, ProxyFactory, ProxyKind},
    proxy::ProxyConfig,
};

pub struct PisaProxyFactory {
    pub proxy_config: ProxyConfig,
    pub pisa_config: PisaProxyConfig,
}

impl PisaProxyFactory {
    pub fn new(proxy_config: ProxyConfig, pisa_config: PisaProxyConfig) -> Self {
        Self { proxy_config, pisa_config }
    }
}

impl ProxyFactory for PisaProxyFactory {
    fn build_proxy(&self, kind: ProxyKind) -> Box<dyn Proxy + Send> {
        let config = self.proxy_config.clone();
        match kind {
            ProxyKind::MySQL => Box::new(runtime_mysql::mysql::MySQLProxy {
                proxy_config: config,
                mysql_nodes: self.pisa_config.get_mysql().to_vec(),
                node_group: self.pisa_config.node_group.clone(),
                pisa_version: self.pisa_config.get_version().to_string(),
                // mysql_nodes: self.pisa_config.mysql.as_ref().unwrap().node.as_ref().unwrap().to_vec(),
                // pisa_version: self.pisa_config.version.as_ref().unwrap().to_string(),
            }),
            ProxyKind::ShardingSphereProxy => {
                Box::new(runtime_shardingsphereproxy::shardingsphereproxy::ShardingSphereProxy {
                    proxy_config: config,
                    shardingsphereproxy_nodes: self.pisa_config.get_shardingsphere_proxy().to_vec(),
                    // shardingsphereproxy_nodes: self.pisa_config.clone().get_shardingsphere_proxy(),
                    // shardingsphereproxy_nodes: self.pisa_config.get_shardingsphere_proxy(),
                    // pisa_version: self.pisa_config.clone().get_version(),
                    // pisa_version: self.pisa_config.get_version(),
                    pisa_version: self.pisa_config.get_version().to_string(),
                    // shardingsphereproxy_nodes: self.pisa_config.shardingsphere_proxy.as_ref().unwrap().node.as_ref().unwrap().to_vec(),
                    // pisa_version: self.pisa_config.version.as_ref().unwrap().to_string(),
                })
            }
        }
    }
}

pub async fn new_proxy_server(mut s: Box<dyn proxy::factory::Proxy + Send>) {
    s.start().await.unwrap();
}
