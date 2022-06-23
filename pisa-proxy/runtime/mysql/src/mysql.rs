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

use bytes::BytesMut;
use common::ast_cache::ParserAstCache;
use conn_pool::Pool;
use endpoint::endpoint::Endpoint;
use loadbalance::balance::{Balance, LoadBalance};
use mysql_parser::parser::Parser;
use mysql_protocol::client::conn::ClientConn;
use parking_lot::Mutex;
use pisa_error::error::{Error, ErrorKind};
use plugin::build_phase::PluginPhase;
use proxy::{
    listener::Listener,
    proxy::{MySQLNode, Proxy, ProxyConfig},
};
use strategy::{config::TargetRole, route::RouteStrategy, readwritesplitting::ReadWriteEndpoint};
use tracing::error;

use crate::server::{metrics::MySqlServerMetricsCollector, server::MySqlServerBuilder};

#[derive(Default)]
pub struct MySQLProxy {
    pub proxy_config: ProxyConfig,
    pub mysql_nodes: Vec<MySQLNode>,
    pub pisa_version: String,
}

impl MySQLProxy {
    fn build_route(&self) -> RouteStrategy {
        let length = self.mysql_nodes.len();
        let (mut rw, mut ro) = (Vec::with_capacity(length), Vec::with_capacity(length));
        for node in &self.mysql_nodes {
            let ep = Endpoint::from(node.clone());
            match node.role {
                TargetRole::Read => ro.push(ep),
                TargetRole::ReadWrite => rw.push(ep)
            }
        }

        if self.proxy_config.read_write_splitting.is_none() {
            let balance_type = self.proxy_config.simple_loadbalance.as_ref().unwrap().balance_type.clone();
            let mut balance = Balance.build_balance(balance_type);
            rw.append(&mut ro);
            for ep in rw.into_iter() {
                balance.add(ep)
            }

            return RouteStrategy::new_with_simple_route(balance)
        }


        let rw_endpoint = ReadWriteEndpoint { read: ro, readwrite: rw };

        RouteStrategy::new(self.proxy_config.read_write_splitting.as_ref().unwrap().clone(), rw_endpoint)
    }
}

#[async_trait::async_trait]
impl proxy::factory::Proxy for MySQLProxy {
    async fn start(&mut self) -> Result<(), Error> {
        let listener = Listener {
            name: self.proxy_config.name.clone(),
            backend_type: "mysql".to_string(),
            listen_addr: self.proxy_config.listen_addr.clone(),
            server_version: self.proxy_config.server_version.clone(),
        };

        let mut proxy = Proxy {
            listener,
            app: self.proxy_config.clone(),
            backend_nodes: self.mysql_nodes.clone(),
        };

        let listener = proxy.build_listener().unwrap();

        let pool = Pool::<ClientConn>::new(self.proxy_config.pool_size as usize);

        let ast_cache = Arc::new(Mutex::new(ParserAstCache::new()));
        // TODO: using a loadbalancer factory for different load balance strategy.
        // Currently simple_loadbalancer purely provide a list of nodes without any strategy.
        //let lb = proxy
        //    .build_loadbalance(self.proxy_config.simple_loadbalance.clone().unwrap().nodes)
        //    .unwrap();

        let lb = Arc::new(tokio::sync::Mutex::new(self.build_route()));

        let mut plugin: Option<PluginPhase> = None;
        if let Some(config) = &self.proxy_config.plugin {
            plugin = Some(PluginPhase::new(config.clone()))
        };

        let parser = Arc::new(Parser::new());
        let metrics_collector = MySqlServerMetricsCollector::new();

        loop {
            // TODO: need refactor
            let socket = proxy.accept(&listener).await.map_err(ErrorKind::Io)?;
            let lb = Arc::clone(&lb);
            let plugin = plugin.clone();
            let pcfg = self.proxy_config.clone();
            let parser = parser.clone();
            let ast_cache = ast_cache.clone();
            let pool = pool.clone();

            let mut mysql_server = MySqlServerBuilder::new(socket, lb, plugin)
                .with_pcfg(pcfg)
                .with_pool(pool)
                .with_buf(BytesMut::with_capacity(8192))
                .with_mysql_parser(parser)
                .with_ast_cache(ast_cache)
                .is_quit(false)
                .with_concurrency_control_rule_idx(None)
                .with_metrics_collector(metrics_collector)
                .with_pisa_version(self.pisa_version.clone())
                .build();

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
