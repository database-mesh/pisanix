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
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use byteorder::{ByteOrder, LittleEndian};
use bytes::{Buf, BufMut, BytesMut};
use common::ast_cache::ParserAstCache;
use conn_pool::{Pool, PoolConn};
use endpoint::endpoint::Endpoint;
use futures::{Future, Sink, SinkExt, StreamExt};
use loadbalance::balance::{Balance, LoadBalance};
use mysql_parser::{
    ast::{Expr, ExprOrDefault, SetOptValues, SetOpts, SqlStmt, Value},
    parser::{ParseError, Parser},
};
use mysql_protocol::{
    client::{codec::ResultsetStream, conn::ClientConn},
    err::ProtocolError,
    mysql_const::{ComType, *},
    server::{
        auth::{handshake, ServerHandshakeCodec},
        codec::{make_eof_packet, ok_packet, CommonPacket, PacketCodec, PacketSend},
        err::MySQLError,
        stream::LocalStream,
    },
    util::*,
    session::{Session, SessionMut},
};
use parking_lot::Mutex;
use pisa_error::error::{Error, ErrorKind};
use plugin::{
    build_phase::PluginPhase,
    err::BoxError,
    layer::{service_fn, Service},
};
use proxy::{
    listener::Listener,
    proxy::{MySQLNode, Proxy, ProxyConfig},
};
use strategy::{
    config::TargetRole,
    readwritesplitting::ReadWriteEndpoint,
    route::{RouteInput, RouteStrategy},
};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{Decoder, Encoder, Framed, FramedWrite};
use tracing::{debug, error};

use crate::{
    server::{
        metrics::*
    },
    transaction_fsm::*,
};

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
                TargetRole::ReadWrite => rw.push(ep),
            }
        }

        if self.proxy_config.read_write_splitting.is_none() {
            let balance_type =
                self.proxy_config.simple_loadbalance.as_ref().unwrap().balance_type.clone();
            let mut balance = Balance.build_balance(balance_type);
            rw.append(&mut ro);
            for ep in rw.into_iter() {
                balance.add(ep)
            }

            return RouteStrategy::new_with_simple_route(balance);
        }

        let rw_endpoint = ReadWriteEndpoint { read: ro, readwrite: rw };

        RouteStrategy::new(
            self.proxy_config.read_write_splitting.as_ref().unwrap().clone(),
            rw_endpoint,
        )
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
        let lb = Arc::new(tokio::sync::Mutex::new(self.build_route()));

        let mut plugin: Option<PluginPhase> = None;
        if let Some(config) = &self.proxy_config.plugin {
            plugin = Some(PluginPhase::new(config.clone()))
        };

        let parser = Arc::new(Parser::new());
        //let metrics_collector = MySQLServerMetricsCollector::new();

        loop {
            // TODO: need refactor
            let socket = proxy.accept(&listener).await.map_err(ErrorKind::Io)?;

            let lb = Arc::clone(&lb);
            let plugin = plugin.clone();
            let _pcfg = self.proxy_config.clone();
            let parser = parser.clone();
            let ast_cache = ast_cache.clone();
            let pool = pool.clone();
            let proxy_name = self.proxy_config.name.clone();

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
                    fsm: TransFsm::new_trans_fsm(lb, pool),
                    ast_cache,
                    plugin,
                    metrics_collector: MySQLServerMetricsCollector,
                    concurrency_control_rule_idx: None,
                    framed,
                    name: proxy_name,
                    mysql_parser: parser,
                };

                if let Err(e) = ins.run(context).await {
                    error!("instance run error {:?}", e);
                }
            });
        }
    }
}

pub struct ReqContext<T, C> {
    name: String,
    fsm: TransFsm,
    mysql_parser: Arc<Parser>,
    ast_cache: Arc<Mutex<ParserAstCache>>,
    plugin: Option<PluginPhase>,
    metrics_collector: MySQLServerMetricsCollector,
    concurrency_control_rule_idx: Option<usize>,
    framed: Framed<T, C>,
}

pub struct RespContext {
    pub ep: Option<String>,
    pub duration: Duration,
}

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

pub struct MySQLInstance<S, T, C> {
    inner: S,
    is_quit: bool,
    _phat: PhantomData<(T, C)>,
}

impl<S, T, C> MySQLInstance<S, T, C>
where
    S: MySQLService<T, C>,
    T: AsyncRead + AsyncWrite + Unpin,
    C: Decoder<Item = BytesMut, Error = ProtocolError> + Encoder<PacketSend<Box<[u8]>>, Error = ProtocolError> + CommonPacket,
{
    fn new(inner: S) -> Self {
        Self { inner, is_quit: false, _phat: PhantomData }
    }

    async fn run(&mut self, mut cx: ReqContext<T, C>) -> Result<(), Error>
    where
        C: Decoder<Item = BytesMut, Error = ProtocolError> + Encoder<PacketSend<Box<[u8]>>> + CommonPacket,
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
                            .send(PacketSend::Encode(err_info.into_boxed_slice()))
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
            cx.framed.send(PacketSend::Encode(err_info.into_boxed_slice())).await.map_err(ErrorKind::from)?;
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
                cx.framed.send(PacketSend::Encode(ok_packet()[..].into())).await.map_err(ErrorKind::from)?;
                return Ok(RespContext { ep: None, duration: now.elapsed() });
            }
            ComType::STMT_PREPARE => S::prepare(cx, &payload).await,
            ComType::STMT_EXECUTE => S::execute(cx, &payload).await,
            ComType::STMT_CLOSE => S::stmt_close(cx, &payload).await,
            ComType::STMT_RESET => {
                cx.framed.send(PacketSend::Encode(ok_packet()[..].into())).await.map_err(ErrorKind::from)?;
                return Ok(RespContext { ep: None, duration: now.elapsed() });
            }
            x => {
                let err_info = make_err_packet(MySQLError::new(
                    1047,
                    "08S01".as_bytes().to_vec(),
                    format!("command {} not support", x.as_ref()),
                ));
                cx.framed.send(PacketSend::Encode(err_info.into_boxed_slice())).await.map_err(ErrorKind::from)?;
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

use mysql_protocol::server::auth::make_err_packet;

pub struct PisaMySQLService<T, C> {
    _phat: PhantomData<(T, C)>,
}

impl<T, C> PisaMySQLService<T, C>
where
    T: AsyncRead + AsyncWrite + Unpin + Send,
    C: Decoder<Item = BytesMut> + Encoder<PacketSend<Box<[u8]>>, Error = ProtocolError> + Send + CommonPacket,
{
    fn new() -> Self {
        Self { _phat: PhantomData }
    }

    //async fn fsm_trigger<'a, 'b: 'a, F>(mut f: F, fsm: &'b mut TransFsm) -> PoolConn<ClientConn>
    //where
    //    F: FnMut(&'a mut TransFsm) -> PoolConn<ClientConn>
    //{
    //    f(fsm)
    //}
    async fn fsm_trigger(
        fsm: &mut TransFsm,
        state_name: TransEventName,
        input: RouteInput<'_>,
    ) -> PoolConn<ClientConn> {
        fsm.trigger(state_name, input).await;
        fsm.get_conn().await.unwrap()
    }

    async fn init_db_inner<'b>(
        req: &mut ReqContext<T, C>,
        client_conn: &mut PoolConn<ClientConn>,
        payload: &[u8],
    ) -> Result<(), Error> {
        let db = std::str::from_utf8(payload).unwrap().trim_matches(char::from(0));

        req.fsm.set_db(Some(db.to_string()));

        let res = client_conn.send_use_db(db).await.map_err(ErrorKind::from)?;

        if res.1 {
            req.framed.send(PacketSend::Encode(ok_packet()[..].into())).await.map_err(ErrorKind::from)?;
        } else {
            // supports CLIENT_PROTOCOL_41 default
            // skip sql_state_marker and sql_state packet
            let err_info = make_err_packet(MySQLError::new(
                1049,
                "42000".as_bytes().to_vec(),
                String::from_utf8_lossy(&res.0[13..]).to_string(),
            ));
            req.framed.send(PacketSend::Encode(err_info[4..].into())).await.map_err(ErrorKind::from)?;
        }

        Ok(())
    }

    async fn prepare_inner(
        req: &mut ReqContext<T, C>,
        client_conn: &mut PoolConn<ClientConn>,
        payload: &[u8],
    ) -> Result<(), Error> {
        let stmt = client_conn.send_prepare(payload).await.map_err(ErrorKind::from)?;

        let mut buf = BytesMut::with_capacity(128);
        //let mut data = BytesMut::from(&vec![0; 4][..]);
        let mut data = vec![0];
        //data.put_u8(0);
        data.extend_from_slice(&u32::to_le_bytes(stmt.stmt_id));
        data.extend_from_slice(&u16::to_le_bytes(stmt.cols_count));
        data.extend_from_slice(&u16::to_le_bytes(stmt.params_count));

        data.extend_from_slice(&[0, 0, 0]);

        //self.client.pkt.make_packet_header(data.len() - 4, &mut data);
        let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(data.into(), 0), &mut buf);
        //req.framed.codec_mut().make_packet_header(data.len() - 4, &mut data, 0);

        if !stmt.params_data.is_empty() {
            for param_data in stmt.params_data {
                let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(param_data[4..].into(), buf.len()), &mut buf);
                //req.framed.codec_mut().make_packet_header(param_data.len() - 4, &mut param_data, 0);
                //data.extend_from_slice(&param_data);
            }

            let eof_packet = make_eof_packet();
            let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(eof_packet[4..].into(), buf.len()), &mut buf);
            //req.framed.codec_mut().make_packet_header(5, &mut eof_packet, 0);
            //data.extend_from_slice(&eof_packet);
        }

        if !stmt.cols_data.is_empty() {
            for col_data in stmt.cols_data {
                let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(col_data[4..].into(), buf.len()), &mut buf);
                //req.framed.codec_mut().make_packet_header(col_data.len() - 4, &mut col_data, 0);
                //data.extend_from_slice(&col_data);
            }

            let eof_packet = make_eof_packet();
            let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(eof_packet[4..].into(), buf.len()), &mut buf);
            //req.framed.codec_mut().make_packet_header(5, &mut eof_packet, 0);
            //data.extend_from_slice(&eof_packet);
        }

        //req.framed
        //    .get_mut()
        //    .write_all(&data[..])
        //    .await
        //    .map_err(|e| Error::from(ErrorKind::from(e)))?;
        //req.framed.get_mut().flush().await.map_err(|e| Error::from(ErrorKind::from(e)))?;
        req.framed.send(PacketSend::Origin(buf[..].into())).await.map_err(ErrorKind::from)?;

        Ok(())
    }

    async fn execute_inner(
        req: &mut ReqContext<T, C>,
        client_conn: &mut PoolConn<ClientConn>,
        payload: &[u8],
    ) -> Result<RespContext, Error> {
        let stream = client_conn.send_execute(payload).await.map_err(ErrorKind::from)?;

        Self::handle_query_resultset(req, stream).await.map_err(ErrorKind::from)?;

        Ok(RespContext { ep: None, duration: Instant::now().elapsed() })
    }

    async fn query_inner(
        req: &mut ReqContext<T, C>,
        client_conn: &mut PoolConn<ClientConn>,
        payload: &[u8],
    ) -> Result<(), Error> {
        let stream = match client_conn.send_query(payload).await {
            Ok(stream) => stream,
            Err(err) => return Err(Error::new(ErrorKind::Protocol(err))),
        };

        Self::handle_query_resultset(req, stream).await.map_err(ErrorKind::from)?;

        Ok(())
    }

    async fn query_inner_get_conn(req: &mut ReqContext<T, C>, sql: &str) -> PoolConn<ClientConn> {
        match Self::get_ast(req, sql) {
            Err(err) => {
                error!("err: {:?}", err);
                Self::fsm_trigger(
                    &mut req.fsm,
                    TransEventName::QueryEvent,
                    RouteInput::Statement(sql),
                )
                .await
            }

            Ok(stmt) => match &stmt[0] {
                SqlStmt::Set(stmt) => {
                    Self::handle_set_stmt( req, stmt, sql).await;
                    req.fsm.get_conn().await.unwrap()
                }
                //TODO: split sql stmt for sql audit
                SqlStmt::BeginStmt(_stmt) => {
                    Self::fsm_trigger(
                        &mut req.fsm,
                        TransEventName::StartEvent,
                        RouteInput::Transaction(sql),
                    )
                    .await
                }

                SqlStmt::Start(_stmt) => {
                    Self::fsm_trigger(
                        &mut req.fsm,
                        TransEventName::StartEvent,
                        RouteInput::Transaction(sql),
                    )
                    .await
                }

                SqlStmt::Commit(_stmt) => {
                    Self::fsm_trigger(
                        &mut req.fsm,
                        TransEventName::CommitRollBackEvent,
                        RouteInput::Transaction(sql),
                    )
                    .await
                }

                SqlStmt::Rollback(_stmt) => {
                    Self::fsm_trigger(
                        &mut req.fsm,
                        TransEventName::CommitRollBackEvent,
                        RouteInput::Transaction(sql),
                    )
                    .await
                }
                _ => {
                    Self::fsm_trigger(
                        &mut req.fsm,
                        TransEventName::QueryEvent,
                        RouteInput::Statement(sql),
                    )
                    .await
                }
            },
        }
    }

    // Set charset name
    async fn handle_set_stmt(req: &mut ReqContext<T, C>, stmt: &SetOptValues, input: &str) {
        match stmt {
            SetOptValues::OptValues(vals) => match &vals.opt {
                SetOpts::SetNames(name) => {
                    if let Some(name) = &name.charset_name {
                        req.framed.codec_mut().get_session().set_charset(name.clone());
                        req.fsm.set_charset(name.clone());
                        let _ = req.fsm.reset_fsm_state(RouteInput::Statement(input)).await;
                        return;
                    }
                }
                SetOpts::SetVariable(val) => {
                    if val.var.to_uppercase() == "AUTOCOMMIT" {
                        match &val.value {
                            ExprOrDefault::Expr(expr) => match expr {
                                Expr::LiteralExpr(Value::Num { value, .. })
                                | Expr::SimpleIdentExpr(Value::Ident { value, .. }) => {
                                    if value == "0" || value.to_uppercase() == "OFF" {
                                        req.fsm.trigger(
                                            TransEventName::SetSessionEvent,
                                            RouteInput::Transaction(input),
                                        )
                                        .await
                                        .unwrap();
                                    }

                                    if value == "1" {
                                        let _ =
                                            req.fsm.reset_fsm_state(RouteInput::Statement(input)).await;
                                    }

                                    req.framed.codec_mut().get_session().set_autocommit(value.clone());
                                    req.fsm.set_autocommit(value.clone());
                                    return;
                                }
                                _ => {}
                            },
                            ExprOrDefault::On => {
                                req.framed.codec_mut().get_session().set_autocommit(String::from("ON"));
                                req.fsm.set_autocommit(String::from("ON"));
                                let _ = req.fsm.reset_fsm_state(RouteInput::Statement(input)).await;
                                return;
                            }

                            _ => {}
                        }
                    }
                }
                _ => {}
            },

            _ => {}
        }

        req.fsm.trigger(TransEventName::SetSessionEvent, RouteInput::Statement(input)).await.unwrap();
    }

    fn get_ast(req: &mut ReqContext<T, C>, sql: &str) -> Result<Vec<SqlStmt>, Error> {
        let mut ast_cache = req.ast_cache.lock();
        let try_ast = ast_cache.get(sql.to_string());

        match try_ast {
            Some(stmt) => Ok(stmt.to_vec()),
            None => match req.mysql_parser.parse(sql) {
                Err(err) => Err(Error::from(ErrorKind::from(err[0].clone()))),
                Ok(stmt) => {
                    ast_cache.set(sql.to_string(), stmt.clone());
                    Ok(stmt)
                }
            },
        }
    }

    pub async fn handle_query_resultset<'b>(
        req: &mut ReqContext<T, C>,
        mut stream: ResultsetStream<'b>,
    ) -> Result<(), ProtocolError> {
        let data = stream.next().await;

        let header = match data {
            Some(Ok(data)) => data,
            Some(Err(e)) => return Err(e),
            None => return Ok(()),
        };

    
        let ok_or_err = header[4];

        if ok_or_err == OK_HEADER || ok_or_err == ERR_HEADER {
            req.framed.send(PacketSend::Encode(header[4..].into())).await?;
            return Ok(());
        }

        let (cols, ..) = length_encode_int(&header[4..]);

        let mut buf = BytesMut::with_capacity(1 << 16);

        let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(header[4..].into(), 0), &mut buf);
        //self.client.pkt.construct_packet_buf(&mut header, &mut self.buf).await;

        for _ in 0..cols {
            let data = stream.next().await;
            let data = match data {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(data[4..].into(), buf.len()), &mut buf);
        }


        // read eof
        let _ = stream.next().await;

        let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(make_eof_packet()[4..].into(), buf.len()), &mut buf);

        loop {
            let data = stream.next().await;

            let row = match data {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            if is_eof(&row) {
                break;
            }

            let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(row[4..].into(), buf.len()), &mut buf);
        }

        let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(make_eof_packet()[4..].into(), buf.len()), &mut buf);

        req.framed.send(PacketSend::Origin(buf[..].into())).await?;

        Ok(())
    }

    pub async fn field_list_inner(
        req: &mut ReqContext<T, C>,
        client_conn: &mut PoolConn<ClientConn>,
        payload: &[u8],
    ) -> Result<(), Error> {
        let mut stream = match client_conn.send_common_command(COM_FIELD_LIST, payload).await {
            Ok(stream) => stream,
            Err(err) => return Err(Error::new(ErrorKind::Protocol(err))),
        };

        let mut buf = BytesMut::with_capacity(128);

        loop {
            let data = match stream.next().await {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(Error::new(ErrorKind::Protocol(e))),
                None => break,
            };

            let _ = req.framed.codec_mut().encode(PacketSend::Encode(data[..].into()), &mut buf);

            if is_eof(&data) {
                break;
            }
        }

        req.framed.send(PacketSend::Origin(buf[..].into())).await.map_err(ErrorKind::from)?;
        //self.client.pkt.write_buf(&buf).await.map_err(|e| Error::new(ErrorKind::Protocol(e)))?;

        Ok(())
    }
}

#[async_trait]
impl<'a, T, C> MySQLService<T, C> for PisaMySQLService<T, C>
where
    T: AsyncRead + AsyncWrite + Unpin + Send,
    C: Decoder<Item = BytesMut> + Encoder<PacketSend<Box<[u8]>>, Error = ProtocolError> + Send + CommonPacket,
{
    async fn init_db(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();

        let db = std::str::from_utf8(payload).unwrap().trim_matches(char::from(0));
        let mut client_conn =
            Self::fsm_trigger(&mut cx.fsm, TransEventName::UseEvent, RouteInput::Statement(db))
                .await;
        let ep = client_conn.get_endpoint();

        crate::collect_sql_processed_total!(cx, "COM_INIT_DB", ep.as_ref().unwrap());
        crate::collect_sql_under_processing_inc!(cx, "COM_INIT_DB", ep.as_ref().unwrap());

        Self::init_db_inner(cx, &mut client_conn, payload).await?;

        cx.fsm.put_conn(client_conn);

        crate::collect_sql_under_processing_dec!(cx, "COM_INIT_DB", ep.as_ref().unwrap());
        crate::collect_sql_processed_duration1!(
            cx,
            "COM_INIT_DB",
            ep.as_ref().unwrap(),
            now.elapsed()
        );

        Ok(RespContext { ep, duration: now.elapsed() })
    }

    async fn query(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();
        let sql = std::str::from_utf8(payload).unwrap().trim_matches(char::from(0));
        let mut client_conn = Self::query_inner_get_conn(cx, sql).await;

        let ep = client_conn.get_endpoint();
        crate::collect_sql_processed_total!(cx, "COM_QUERY", ep.as_ref().unwrap());
        crate::collect_sql_under_processing_inc!(cx, "COM_QUERY", ep.as_ref().unwrap());

        let _ = Self::query_inner(cx, &mut client_conn, payload).await?;

        cx.fsm.put_conn(client_conn);

        crate::collect_sql_under_processing_dec!(cx, "COM_QUERY", ep.as_ref().unwrap());
        crate::collect_sql_processed_duration1!(
            cx,
            "COM_QUERY",
            ep.as_ref().unwrap(),
            now.elapsed()
        );

        Ok(RespContext { ep, duration: now.elapsed() })
    }

    async fn prepare(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();
        let sql = std::str::from_utf8(payload).unwrap().trim_matches(char::from(0));
        let mut client_conn = Self::fsm_trigger(
            &mut cx.fsm,
            TransEventName::PrepareEvent,
            RouteInput::Statement(sql),
        )
        .await;
        let ep = client_conn.get_endpoint();

        crate::collect_sql_processed_total!(cx, "COM_PREPARE", ep.as_ref().unwrap());
        crate::collect_sql_under_processing_inc!(cx, "COM_PREPARE", ep.as_ref().unwrap());

        let res = Self::prepare_inner(cx, &mut client_conn, payload).await;
        cx.fsm.put_conn(client_conn);

        crate::collect_sql_under_processing_dec!(cx, "COM_PREPARE", ep.as_ref().unwrap());
        crate::collect_sql_processed_duration1!(
            cx,
            "COM_PREPARE",
            ep.as_ref().unwrap(),
            now.elapsed()
        );

        if let Err(ref err) = res {
            if let ErrorKind::Protocol(ProtocolError::PrepareError(data)) = err.kind() {
                cx.framed.send(PacketSend::Encode(data[4..].into())).await.map_err(ErrorKind::from)?;
            }
        }

        Ok(RespContext { ep, duration: now.elapsed() })
    }

    async fn execute(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();
        let mut client_conn = cx.fsm.get_conn().await?;
        let ep = client_conn.get_endpoint();

        crate::collect_sql_processed_total!(cx, "COM_EXECUTE", ep.as_ref().unwrap());
        crate::collect_sql_under_processing_inc!(cx, "COM_EXECUTE", ep.as_ref().unwrap());

        let _ = Self::execute_inner(cx, &mut client_conn, payload).await;
        cx.fsm.put_conn(client_conn);

        crate::collect_sql_under_processing_dec!(cx, "COM_EXECUTE", ep.as_ref().unwrap());
        crate::collect_sql_processed_duration1!(
            cx,
            "COM_EXECUTE",
            ep.as_ref().unwrap(),
            now.elapsed()
        );

        Ok(RespContext { ep, duration: now.elapsed() })
    }

    async fn stmt_close(_cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();
        let stmt_id = LittleEndian::read_u32(payload);
        debug!("stmt close {:?}", stmt_id);

        Ok(RespContext { ep: None, duration: now.elapsed() })
    }

    async fn quit(_cx: &mut ReqContext<T, C>) -> Result<RespContext, Error> {
        let now = Instant::now();
        Ok(RespContext { ep: None, duration: now.elapsed() })
    }

    async fn field_list(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();
        let mut client_conn =
            Self::fsm_trigger(&mut cx.fsm, TransEventName::QueryEvent, RouteInput::None).await;
        let ep = client_conn.get_endpoint();

        crate::collect_sql_processed_total!(cx, "COM_FIELD_LIST", ep.as_ref().unwrap());
        crate::collect_sql_under_processing_inc!(cx, "COM_FIELD_LIST", ep.as_ref().unwrap());

        Self::field_list_inner(cx, &mut client_conn, payload).await?;

        cx.fsm.put_conn(client_conn);

        crate::collect_sql_under_processing_dec!(cx, "COM_FIELD_LIST", ep.as_ref().unwrap());
        crate::collect_sql_processed_duration1!(
            cx,
            "COM_FIELD_LIST",
            ep.as_ref().unwrap(),
            now.elapsed()
        );

        Ok(RespContext { ep, duration: now.elapsed() })
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        assert_eq!(1, 1)
    }
}
