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

use std::{
    marker::PhantomData,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use bytes::{Buf, BytesMut};
use common::ast_cache::ParserAstCache;
use conn_pool::Pool;
use endpoint::endpoint::Endpoint;
use futures::{SinkExt, StreamExt};
use loadbalance::balance::{Balance, LoadBalance};
use mysql_parser::parser::Parser;
use mysql_protocol::{
    client::conn::ClientConn,
    err::ProtocolError,
    mysql_const::ComType,
    server::{
        auth::{handshake, ServerHandshakeCodec},
        codec::{make_err_packet, ok_packet, CommonPacket, PacketCodec, PacketSend},
        err::MySQLError,
        stream::LocalStream,
    },
    session::Session,
};
use parking_lot::Mutex;
use pisa_error::error::{Error, ErrorKind};
use plugin::{build_phase::PluginPhase, err::BoxError, layer::Service};
use proxy::{
    listener::Listener,
    proxy::{MySQLNode, Proxy, ProxyConfig},
};
use strategy::{
    config::{NodeGroup, TargetRole},
    readwritesplitting::ReadWriteEndpoint,
    route::{ReadWriteSplittingRouteStrategy, RouteStrategy},
    sharding_rewrite::{self, ShardingRewrite},
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Encoder, Framed};
use tracing::error;

use crate::{
    server::{metrics::*, PisaMySQLService},
    transaction_fsm::*,
};

#[derive(Default)]
pub struct MySQLProxy {
    pub proxy_config: ProxyConfig,
    pub node_group: Option<NodeGroup>,
    pub mysql_nodes: Vec<MySQLNode>,
    pub pisa_version: String,
}

impl MySQLProxy {
    fn build_route(&self) -> Result<RouteStrategy, Error> {
        let length = self.mysql_nodes.len();
        let (mut rw, mut ro) = (Vec::with_capacity(length), Vec::with_capacity(length));
        for node in &self.mysql_nodes {
            let ep = Endpoint::from(node.clone());
            match node.role {
                TargetRole::Read => ro.push(ep),
                TargetRole::ReadWrite => rw.push(ep),
            }
        }

        let strategy = if self.proxy_config.read_write_splitting.is_some()
            && self.proxy_config.sharding.is_some()
        {
            let rw_endpoint = ReadWriteEndpoint { read: ro, readwrite: rw };
            RouteStrategy::new(
                self.proxy_config.read_write_splitting.as_ref().unwrap().clone(),
                &self.node_group,
                rw_endpoint,
                true,
            )
            .map_err(|e| Error::new(ErrorKind::Runtime(e.into())))?
        } else if self.proxy_config.read_write_splitting.is_some() {
            let rw_endpoint = ReadWriteEndpoint { read: ro, readwrite: rw };
            RouteStrategy::new(
                self.proxy_config.read_write_splitting.as_ref().unwrap().clone(),
                &self.node_group,
                rw_endpoint,
                false,
            )
            .map_err(|e| Error::new(ErrorKind::Runtime(e.into())))?
        } else if self.proxy_config.sharding.is_some() {
            RouteStrategy::new_with_sharding_only()
        } else {
            //let rw_endpoint = ReadWriteEndpoint { read: ro, readwrite: rw };
            let balance_type =
                self.proxy_config.simple_loadbalance.as_ref().unwrap().balance_type.clone();
            let mut balance = Balance.build_balance(balance_type);
            rw.append(&mut ro);
            for ep in rw.into_iter() {
                balance.add(ep)
            }

            RouteStrategy::new_with_simple_route(balance)
        };

        Ok(strategy)
    }

    fn build_sharding_rewriter(&self) -> Option<ShardingRewrite> {
        let config = self.proxy_config.sharding.clone();
        if config.is_none() {
            return None;
        }

        let mut endpoints: Vec<Endpoint> = vec![];
        for mysql_node in &self.mysql_nodes {
            let endpoint = Endpoint::from(mysql_node.clone());
            endpoints.push(endpoint);
        }

        let mut has_rw = self.proxy_config.read_write_splitting.is_some();

        Some(ShardingRewrite::new(config.unwrap(), endpoints, has_rw))
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
        let route_strategy = Arc::new(Mutex::new(self.build_route()?));

        // Build sharding rewriter
        let rewriter = self.build_sharding_rewriter();

        let mut plugin: Option<PluginPhase> = None;
        if let Some(config) = &self.proxy_config.plugin {
            plugin = Some(PluginPhase::new(config.clone()))
        };

        let parser = Arc::new(Parser::new());
        //let metrics_collector = MySQLServerMetricsCollector::new();

        let has_rw = self.proxy_config.read_write_splitting.is_some();

        loop {
            // TODO: need refactor
            let socket = proxy.accept(&listener).await.map_err(ErrorKind::Io)?;

            let route_strategy = route_strategy.clone();
            let plugin = plugin.clone();
            let _pcfg = self.proxy_config.clone();
            let parser = parser.clone();
            let ast_cache = ast_cache.clone();
            let pool = pool.clone();
            let proxy_name = self.proxy_config.name.clone();
            let rewriter = rewriter.clone();

            let handshake_codec = ServerHandshakeCodec::new(
                self.proxy_config.user.clone(),
                self.proxy_config.password.clone(),
                self.proxy_config.db.clone(),
                self.proxy_config.server_version.clone(),
            );

            let handshake_framed =
                Framed::with_capacity(LocalStream::from(socket), handshake_codec, 8196);

            let mut ins = MySQLInstance::new(PisaMySQLService::new());

            tokio::spawn(async move {
                let res = handshake(handshake_framed).await;
                if let Err(e) = res {
                    error!("handshake error {:?}", e);
                    return;
                }

                let handshake_framed = res.unwrap().0;
                let parts = handshake_framed.into_parts();

                let packet_codec = PacketCodec::new(parts.codec, 8196);
                let io = parts.io;

                let framed = Framed::with_capacity(io, packet_codec, 16384);
                let context = ReqContext {
                    fsm: TransFsm::new(pool.clone()),
                    route_strategy,
                    pool,
                    ast_cache,
                    plugin,
                    metrics_collector: MySQLServerMetricsCollector,
                    concurrency_control_rule_idx: None,
                    framed,
                    name: proxy_name,
                    mysql_parser: parser,
                    rewriter,
                    has_readwritesplitting: has_rw,
                };

                if let Err(e) = ins.run(context).await {
                    error!("instance run error {:?}", e);
                }
            });
        }
    }
}

/// The Context arg required to handle the command
pub struct ReqContext<T, C> {
    pub name: String,
    pub fsm: TransFsm,
    pub route_strategy: Arc<Mutex<RouteStrategy>>,
    pub pool: Pool<ClientConn>,
    pub mysql_parser: Arc<Parser>,
    pub ast_cache: Arc<Mutex<ParserAstCache>>,
    pub plugin: Option<PluginPhase>,
    pub metrics_collector: MySQLServerMetricsCollector,
    // `concurrency_control_rule_idx` is index of concurrency_control rules
    // `concurrency_control_rule_idx` is required to add permits when the
    //  concurrency_control layer service is enabled
    pub concurrency_control_rule_idx: Option<usize>,
    // The codc for MySQL Protocol
    pub framed: Framed<T, C>,
    pub rewriter: Option<ShardingRewrite>,
    pub has_readwritesplitting: bool,
}

/// Handle the return value of the command
pub struct RespContext {
    // The endpoint of the backend dababase
    pub ep: Option<String>,
    // The duration of handle the command
    pub duration: Duration,
}

/// The MySQLService trait is used to handle the mysql command,
/// Its can be implemeneted by third-party service.
/// The PisaMySQLService is default implementation in the Pisa-Proxy.
#[async_trait]
pub trait MySQLService<T, C> {
    async fn init_db(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error>;
    async fn query(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error>;
    async fn prepare(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error>;
    async fn execute(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error>;
    async fn stmt_close(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error>;
    async fn quit(cx: &mut ReqContext<T, C>) -> Result<RespContext, Error>;
    async fn field_list(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error>;
}

/// Start an instance of the `MySQLService`, its used to execute method
/// of the `MySQLService` trait
pub struct MySQLInstance<S, T, C> {
    // A service implementing the MySQLSerivce trait to handle mysql command
    _inner: S,
    // Mark whether the instance quit
    is_quit: bool,
    _phat: PhantomData<(T, C)>,
}

impl<S, T, C> MySQLInstance<S, T, C>
where
    S: MySQLService<T, C>,
    T: AsyncRead + AsyncWrite + Unpin,
    C: Decoder<Item = BytesMut, Error = ProtocolError>
        + Encoder<PacketSend<Box<[u8]>>, Error = ProtocolError>
        + CommonPacket,
{
    fn new(inner: S) -> Self {
        Self { _inner: inner, is_quit: false, _phat: PhantomData }
    }

    async fn run(&mut self, mut cx: ReqContext<T, C>) -> Result<(), Error>
    where
        C: Decoder<Item = BytesMut, Error = ProtocolError>
            + Encoder<PacketSend<Box<[u8]>>>
            + CommonPacket,
    {
        let db = cx.framed.codec_mut().get_session().get_db();
        cx.fsm.set_db(db);

        while let Some(data) = cx.framed.next().await {
            match data {
                Ok(data) => {
                    if let Err(err) = self.handle_command(&mut cx, data).await {
                        let err_info = make_err_packet(MySQLError::new(
                            2002,
                            "HY000".as_bytes().to_vec(),
                            String::from("There is no healthy backend to connect."),
                        ));
                        cx.framed
                            .send(PacketSend::Encode(err_info[4..].into()))
                            .await
                            .map_err(ErrorKind::from)?;
                        error!("exec command err: {:?}", err);
                    };

                    cx.framed.codec_mut().reset_seq();

                    if let Some(idx) = &cx.concurrency_control_rule_idx {
                        cx.plugin.as_mut().unwrap().concurrency_control.add_permits(*idx);
                        cx.concurrency_control_rule_idx = None;
                    }

                    if self.is_quit {
                        return Ok(());
                    }
                }

                Err(e) => return Err(Error::from(ErrorKind::from(e))),
            }
        }

        return Ok(());
    }

    async fn handle_command(
        &mut self,
        cx: &mut ReqContext<T, C>,
        mut data: BytesMut,
    ) -> Result<RespContext, Error> {
        let now = Instant::now();
        let com = data.get_u8();
        let payload = data.split();

        if let Err(err) = self.plugin_run(cx, &payload) {
            let err_info = make_err_packet(MySQLError::new(
                1047,
                "08S01".as_bytes().to_vec(),
                err.to_string(),
            ));
            cx.framed
                .send(PacketSend::Encode(err_info[4..].into()))
                .await
                .map_err(ErrorKind::from)?;
            return Ok(RespContext { ep: None, duration: now.elapsed() });
        }

        match ComType::from(com) {
            ComType::QUIT => {
                self.is_quit = true;
                S::quit(cx).await
            }
            ComType::INIT_DB => S::init_db(cx, &payload).await,
            ComType::QUERY => S::query(cx, &payload).await,
            ComType::FIELD_LIST => S::field_list(cx, &payload).await,
            ComType::PING => {
                cx.framed
                    .send(PacketSend::Encode(ok_packet()[4..].into()))
                    .await
                    .map_err(ErrorKind::from)?;
                return Ok(RespContext { ep: None, duration: now.elapsed() });
            }
            ComType::STMT_PREPARE => S::prepare(cx, &payload).await,
            ComType::STMT_EXECUTE => S::execute(cx, &payload).await,
            ComType::STMT_CLOSE => S::stmt_close(cx, &payload).await,
            ComType::STMT_RESET => {
                cx.framed
                    .send(PacketSend::Encode(ok_packet()[4..].into()))
                    .await
                    .map_err(ErrorKind::from)?;
                return Ok(RespContext { ep: None, duration: now.elapsed() });
            }
            x => {
                let err_info = make_err_packet(MySQLError::new(
                    1047,
                    "08S01".as_bytes().to_vec(),
                    format!("command {} not support", x.as_ref()),
                ));
                cx.framed
                    .send(PacketSend::Encode(err_info[4..].into()))
                    .await
                    .map_err(ErrorKind::from)?;
                return Ok(RespContext { ep: None, duration: now.elapsed() });
            }
        }
    }

    fn plugin_run(&mut self, cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<(), BoxError> {
        if let Some(plugin) = cx.plugin.as_mut() {
            let input = unsafe { std::str::from_utf8_unchecked(payload).to_string() };

            plugin.circuit_break.handle(input.clone())?;

            let res = plugin.concurrency_control.handle(input);

            match res {
                Ok(data) => {
                    cx.concurrency_control_rule_idx = data.0;
                    return Ok(());
                }

                Err(err) => return Err(err),
            }
        }

        Ok(())
    }
    
}
