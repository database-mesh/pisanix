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

use config::config::PisaConfig;
use proxy::{
    factory::{Proxy, ProxyFactory, ProxyKind},
    proxy::ProxyConfig,
};

pub struct PisaProxyFactory {
    pub proxy_config: ProxyConfig,
    pub pisa_config: PisaConfig,
}

impl PisaProxyFactory {
    pub fn new(proxy_config: ProxyConfig, pisa_config: PisaConfig) -> Self {
        Self { proxy_config, pisa_config }
    }
}

impl ProxyFactory for PisaProxyFactory {
    fn build_proxy(&self, kind: ProxyKind) -> Box<dyn Proxy + Send> {
        let config = self.proxy_config.clone();
        match kind {
            ProxyKind::MySQL => Box::new(runtime_mysql::mysql::MySQLProxy {
                proxy_config: config,
                mysql_nodes: self.pisa_config.mysql_nodes.clone(),
                pisa_version: self.pisa_config.version.clone(), 
            }),
            ProxyKind::ShardingSphereProxy => {
                Box::new(runtime_shardingsphereproxy::shardingsphereproxy::ShardingSphereProxy {
                    proxy_config: config,
                    shardingsphereproxy_nodes: self.pisa_config.shardingsphere_proxy_nodes.clone(),
                    pisa_version: self.pisa_config.version.clone(), 
                })
            }
        }
    }
}

pub async fn new_proxy_server(mut s: Box<dyn proxy::factory::Proxy + Send>) {
    s.start().await.unwrap();
}
