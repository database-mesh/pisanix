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

use std::sync::Arc;

use common::ast_cache::ParserAstCache;
use conn_pool::Pool;
use mysql_parser::parser::Parser;
use pisa_error::error::{Error, ErrorKind};
use plugin::build_phase::PluginPhase;
use proxy::{
    listener::Listener,
    proxy::{MySQLNode, Proxy, ProxyConfig},
};
use tracing::error;
use parking_lot::Mutex;

use crate::server::MySqlServer;

pub struct MySQLProxy {
    pub proxy_config: ProxyConfig,
    pub mysql_nodes: Vec<MySQLNode>,
}

#[async_trait::async_trait]
impl proxy::factory::Proxy for MySQLProxy {
    async fn start(&mut self) -> Result<(), Error> {
        let listener = Listener {
            backend_type: "mysql".to_string(),
            listen_addr: self.proxy_config.listen_addr.clone(),
        };

        let mut proxy = Proxy {
            listener,
            app: self.proxy_config.clone(),
            backend_nodes: self.mysql_nodes.clone(),
        };

        let listener = proxy.build_listener().unwrap();

        let pool = Pool::new(self.proxy_config.pool_size as usize);

        let ast_cache = Arc::new(Mutex::new(ParserAstCache::new()));
        // TODO: using a loadbalancer factory for different load balance strategy.
        // Currently simple_loadbalancer purely provide a list of nodes without any strategy.
        let lb = proxy
            .build_loadbalance(self.proxy_config.simple_loadbalance.clone().unwrap().nodes)
            .unwrap();

        let mut plugin: Option<PluginPhase> = None;
        if let Some(config) = &self.proxy_config.plugin {
            plugin = Some(PluginPhase::new(config.clone()))
        };

        let parser = Arc::new(Parser::new());

        loop {
            let socket = proxy.accept(&listener).await.map_err(ErrorKind::Io)?;
            let pool = pool.clone();
            let lb = Arc::clone(&lb);
            let pcfg = self.proxy_config.clone();
            let parser = parser.clone();
            let ast_cache = ast_cache.clone();
            let plugin = plugin.clone();

            //TODO: add multiple thread limit with Semaphore
            let mut mysql_server =
                MySqlServer::new(socket, pool, lb, pcfg, parser, ast_cache, plugin).await;

            if let Err(err) = mysql_server.handshake().await {
                error!("{:?}", err);
                continue;
            }

            tokio::spawn(async move {
                if let Err(err) = mysql_server.run().await {
                    error!("{:?}", err);
                }
            });
        }
    }
}
