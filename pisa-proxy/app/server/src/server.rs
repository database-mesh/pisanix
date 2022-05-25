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
    factory::{Factory, Proxy, ProxyKind},
    proxy::ProxyConfig,
};

pub struct SimpleFactory {
    pub proxy_config: ProxyConfig,
    pub pisa_config: PisaConfig,
}

impl SimpleFactory {
    pub fn new(proxy_config: ProxyConfig, pisa_config: PisaConfig) -> Self {
        Self { proxy_config, pisa_config }
    }
}

impl Factory for SimpleFactory {
    fn build_proxy(&self, kind: ProxyKind) -> Box<dyn Proxy + Send> {
        let config = self.proxy_config.clone();
        match kind {
            ProxyKind::MySQL => Box::new(runtime_mysql::mysql::MySQLProxy {
                proxy_config: config,
                mysql_nodes: self.pisa_config.mysql_nodes.clone(),
            }),
            ProxyKind::ShardingProxy => {
                Box::new(runtime_shardingproxy::shardingproxy::ShardingProxy {
                    proxy_config: config,
                    shardingproxy_nodes: self.pisa_config.sharding_proxy_nodes.clone(),
                })
            }
        }
    }
}

pub async fn new_proxy_server(mut s: Box<dyn proxy::factory::Proxy + Send>) {
    s.start().await.unwrap();
}
