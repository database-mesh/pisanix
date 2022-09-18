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

use std::{fmt::Pointer, marker::PhantomData, time::Instant};

use async_trait::async_trait;
use byteorder::{ByteOrder, LittleEndian};
use bytes::BytesMut;
use conn_pool::PoolConn;
use futures::{SinkExt, StreamExt};
use mysql_parser::ast::*;
use mysql_protocol::{
    client::{codec::ResultsetStream, conn::ClientConn},
    err::ProtocolError,
    mysql_const::*,
    server::{
        codec::{make_eof_packet, make_err_packet, ok_packet, CommonPacket, PacketSend},
        err::MySQLError,
    },
    session::{Session, SessionMut},
    util::{is_eof, length_encode_int},
};
use pisa_error::error::{Error, ErrorKind};
use strategy::{
    route::{RouteInput, RouteInputTyp},
    sharding_rewrite::{DataSource, ShardingRewriteOutput},
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Encoder};
use tracing::{debug, error};

use crate::{
    mysql::{MySQLService, ReqContext, RespContext},
    transaction_fsm::{
        build_conn_attrs, query_rewrite, route, route_sharding, TransEventName, TransFsm, check_get_conn,
    },
};

pub struct PisaMySQLService<T, C> {
    _phat: PhantomData<(T, C)>,
}

impl<T, C> PisaMySQLService<T, C>
where
    T: AsyncRead + AsyncWrite + Unpin + Send,
    C: Decoder<Item = BytesMut>
        + Encoder<PacketSend<Box<[u8]>>, Error = ProtocolError>
        + Send
        + CommonPacket,
{
    pub fn new() -> Self {
        Self { _phat: PhantomData }
    }

    async fn fsm_trigger(
        req: &mut ReqContext<T, C>,
        state_name: TransEventName,
        input_typ: RouteInputTyp,
        raw_sql: &str,
    ) -> Result<PoolConn<ClientConn>, Error> {
        let sess = req.framed.codec_mut().get_session();
        let attrs = build_conn_attrs(sess);
        let is_get_conn = req.fsm.trigger(state_name);

        if is_get_conn {
            let endpoint = route(input_typ, raw_sql, req.route_strategy.clone());
            let factory =
                ClientConn::with_opts(endpoint.user, endpoint.password, endpoint.addr.clone());
            req.pool.set_factory(factory);
            return check_get_conn(req.pool.clone(), &endpoint.addr, &attrs).await
        }

        req.fsm.get_conn(&attrs).await
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
            req.framed
                .send(PacketSend::Encode(ok_packet()[4..].into()))
                .await
                .map_err(ErrorKind::from)?;
        } else {
            // supports CLIENT_PROTOCOL_41 default
            // skip sql_state_marker and sql_state packet
            let err_info = make_err_packet(MySQLError::new(
                1049,
                "42000".as_bytes().to_vec(),
                String::from_utf8_lossy(&res.0[13..]).to_string(),
            ));
            req.framed
                .send(PacketSend::Encode(err_info[4..].into()))
                .await
                .map_err(ErrorKind::from)?;
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
        let mut data = vec![0];
        data.extend_from_slice(&u32::to_le_bytes(stmt.stmt_id));
        data.extend_from_slice(&u16::to_le_bytes(stmt.cols_count));
        data.extend_from_slice(&u16::to_le_bytes(stmt.params_count));

        data.extend_from_slice(&[0, 0, 0]);

        let _ = req.framed.codec_mut().encode(PacketSend::EncodeOffset(data.into(), 0), &mut buf);

        if !stmt.params_data.is_empty() {
            for param_data in stmt.params_data {
                let _ = req
                    .framed
                    .codec_mut()
                    .encode(PacketSend::EncodeOffset(param_data[4..].into(), buf.len()), &mut buf);
            }

            let eof_packet = make_eof_packet();
            let _ = req
                .framed
                .codec_mut()
                .encode(PacketSend::EncodeOffset(eof_packet[4..].into(), buf.len()), &mut buf);
        }

        if !stmt.cols_data.is_empty() {
            for col_data in stmt.cols_data {
                let _ = req
                    .framed
                    .codec_mut()
                    .encode(PacketSend::EncodeOffset(col_data[4..].into(), buf.len()), &mut buf);
            }

            let eof_packet = make_eof_packet();
            let _ = req
                .framed
                .codec_mut()
                .encode(PacketSend::EncodeOffset(eof_packet[4..].into(), buf.len()), &mut buf);
        }

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

    async fn query_inner_get_conn(
        req: &mut ReqContext<T, C>,
        sql: &str,
    ) -> Result<PoolConn<ClientConn>, Error> {
        Err(Error::new(ErrorKind::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "fff"))))
    }

    fn query_rewrite<'a>(
        req: &'a mut ReqContext<T, C>,
        sql: &'a str,
    ) -> Result<(bool, RouteInputTyp, Vec<ShardingRewriteOutput>), Error> {
        let ast = Self::get_ast(req, sql);
        let mut ast = match ast {
            Err(err) => {
                error!("parse sql {:?} err: {:?}", sql, err);
                if req.rewriter.is_some() {
                    return Err(err);
                }
                let is_get_conn = req.fsm.trigger(TransEventName::QueryEvent);
                return Ok((is_get_conn, RouteInputTyp::Statement, vec![]));
            }

            Ok(ast) => ast[0].clone(),
        };

        let (is_get_conn, input, can_rewrite) = match &ast {
            SqlStmt::Set(stmt) => {
                let (is_get_conn, input) = Self::handle_set_stmt(req, &stmt);
                (is_get_conn, input, false)
            }
            //TODO: split sql stmt for sql audit
            SqlStmt::BeginStmt(_stmt) => {
                //Self::fsm_trigger(
                //    &mut req,
                //    TransEventName::StartEvent,
                //    RouteInput::Transaction(sql),
                //)
                //.await

                (req.fsm.trigger(TransEventName::StartEvent), RouteInputTyp::Transaction, false)
            }

            SqlStmt::Start(_stmt) => {
                //Self::fsm_trigger(
                //    &mut req,
                //    TransEventName::StartEvent,
                //    RouteInput::Transaction(sql),
                //)
                //.await
                (req.fsm.trigger(TransEventName::StartEvent), RouteInputTyp::Transaction, false)
            }

            SqlStmt::Commit(_stmt) => {
                //Self::fsm_trigger(
                //    &mut req,
                //    TransEventName::CommitRollBackEvent,
                //    RouteInput::Transaction(sql),
                //)
                //.await
                (
                    req.fsm.trigger(TransEventName::CommitRollBackEvent),
                    RouteInputTyp::Transaction,
                    false,
                )
            }

            SqlStmt::Rollback(_stmt) => {
                //Self::fsm_trigger(
                //    &mut req,
                //    TransEventName::CommitRollBackEvent,
                //    RouteInput::Transaction(sql),
                //)
                //.await
                (
                    req.fsm.trigger(TransEventName::CommitRollBackEvent),
                    RouteInputTyp::Transaction,
                    false,
                )
            }
            _ => {
                //Self::fsm_trigger(
                //    &mut req,
                //    TransEventName::QueryEvent,
                //    RouteInput::Statement(sql),
                //)
                //.await
                (req.fsm.trigger(TransEventName::QueryEvent), RouteInputTyp::Statement, true)
            }
        };

        let is_rewriter = req.rewriter.is_some().clone();
        if is_rewriter {
            let outputs =
                query_rewrite(req.rewriter.as_mut().unwrap(), sql.to_string(), ast, can_rewrite)
                    .map_err(|e| ErrorKind::Runtime(e.into()))?;
            return Ok((is_get_conn, input, outputs));
        }

        return Ok((is_get_conn, input, vec![]));
    }

    // Set charset name
    fn handle_set_stmt<'b: 'a, 'a>(
        req: &'b mut ReqContext<T, C>,
        stmt: &'a SetOptValues,
    ) -> (bool, RouteInputTyp) {
        match stmt {
            SetOptValues::OptValues(vals) => match &vals.opt {
                SetOpts::SetNames(name) => {
                    if let Some(name) = &name.charset_name {
                        req.framed.codec_mut().get_session().set_charset(name.clone());
                        req.fsm.set_charset(name.clone());
                        let _ = req.fsm.reset_fsm_state();
                        return (true, RouteInputTyp::Statement);
                    }
                }
                SetOpts::SetVariable(val) => {
                    if val.var.to_uppercase() == "AUTOCOMMIT" {
                        match &val.value {
                            ExprOrDefault::Expr(expr) => match expr {
                                Expr::LiteralExpr(Value::Num { value, .. })
                                | Expr::SimpleIdentExpr(Value::Ident { value, .. }) => {
                                    if value == "0" || value.to_uppercase() == "OFF" {
                                        //req.fsm
                                        //    .trigger(
                                        //        TransEventName::SetSessionEvent,
                                        //        RouteInput::Transaction(input),
                                        //    )
                                        //    .await
                                        //    .unwrap();
                                        let is_get_conn =
                                            req.fsm.trigger(TransEventName::SetSessionEvent);
                                        return (is_get_conn, RouteInputTyp::Transaction);
                                    }

                                    if value == "1" {
                                        //let _ = req
                                        //    .fsm
                                        //    .reset_fsm_state(RouteInput::Statement(input))
                                        //    .await;
                                        req.fsm.reset_fsm_state();
                                    }

                                    req.framed
                                        .codec_mut()
                                        .get_session()
                                        .set_autocommit(value.clone());
                                    req.fsm.set_autocommit(value.clone());
                                    return (true, RouteInputTyp::Statement);
                                }
                                _ => {}
                            },
                            ExprOrDefault::On => {
                                req.framed
                                    .codec_mut()
                                    .get_session()
                                    .set_autocommit(String::from("ON"));
                                req.fsm.set_autocommit(String::from("ON"));
                                //let _ = req.fsm.reset_fsm_state(RouteInput::Statement(input)).await;
                                req.fsm.reset_fsm_state();

                                return (true, RouteInputTyp::Statement);
                            }

                            _ => {}
                        }
                    }
                }
                _ => {}
            },

            _ => {}
        }

        //req.fsm
        //    .trigger(TransEventName::SetSessionEvent, RouteInput::Statement(input))
        //    .await
        //    .unwrap();
        let is_get_conn = req.fsm.trigger(TransEventName::SetSessionEvent);
        (is_get_conn, RouteInputTyp::Statement)
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

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(header[4..].into(), 0), &mut buf);

        for _ in 0..cols {
            let data = stream.next().await;
            let data = match data {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            let _ = req
                .framed
                .codec_mut()
                .encode(PacketSend::EncodeOffset(data[4..].into(), buf.len()), &mut buf);
        }

        // read eof
        let _ = stream.next().await;

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(make_eof_packet()[4..].into(), buf.len()), &mut buf);

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

            let _ = req
                .framed
                .codec_mut()
                .encode(PacketSend::EncodeOffset(row[4..].into(), buf.len()), &mut buf);
        }

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(make_eof_packet()[4..].into(), buf.len()), &mut buf);

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

            let _ = req
                .framed
                .codec_mut()
                .encode(PacketSend::EncodeOffset(data[4..].into(), buf.len()), &mut buf);

            if is_eof(&data) {
                break;
            }
        }

        req.framed.send(PacketSend::Origin(buf[..].into())).await.map_err(ErrorKind::from)?;

        Ok(())
    }

    async fn sharding_command_not_support(
        cx: &mut ReqContext<T, C>,
        command: &str,
    ) -> Result<(), Error> {
        let err_info = make_err_packet(MySQLError::new(
            1047,
            "08S01".as_bytes().to_vec(),
            format!("command {:?} not support in sharding", command),
        ));
        cx.framed.send(PacketSend::Encode(err_info[4..].into())).await.map_err(ErrorKind::from)?;
        Ok(())
    }
}

#[async_trait]
impl<'a, T, C> MySQLService<T, C> for PisaMySQLService<T, C>
where
    T: AsyncRead + AsyncWrite + Unpin + Send,
    C: Decoder<Item = BytesMut>
        + Encoder<PacketSend<Box<[u8]>>, Error = ProtocolError>
        + Send
        + CommonPacket,
{
    async fn init_db(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();

        if cx.rewriter.is_some() {
            Self::sharding_command_not_support(cx, ComType::INIT_DB.as_ref()).await?;
            return Ok(RespContext { ep: None, duration: now.elapsed() });
        }

        let db = std::str::from_utf8(payload).unwrap().trim_matches(char::from(0));
        let mut client_conn =
            Self::fsm_trigger(cx, TransEventName::UseEvent, RouteInputTyp::Statement, db).await?;
        let ep = client_conn.get_endpoint();

        collect_sql_processed_total!(cx, "COM_INIT_DB", ep.as_ref().unwrap());
        collect_sql_under_processing_inc!(cx, "COM_INIT_DB", ep.as_ref().unwrap());

        Self::init_db_inner(cx, &mut client_conn, payload).await?;

        cx.fsm.put_conn(client_conn);

        collect_sql_under_processing_dec!(cx, "COM_INIT_DB", ep.as_ref().unwrap());
        collect_sql_processed_duration!(cx, "COM_INIT_DB", ep.as_ref().unwrap(), now.elapsed());

        Ok(RespContext { ep, duration: now.elapsed() })
    }

    async fn query(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();
        let sql = std::str::from_utf8(payload).unwrap().trim_matches(char::from(0));
        let mut client_conn = Self::query_inner_get_conn(cx, sql).await?;

        let ep = client_conn.get_endpoint();
        collect_sql_processed_total!(cx, "COM_QUERY", ep.as_ref().unwrap());
        collect_sql_under_processing_inc!(cx, "COM_QUERY", ep.as_ref().unwrap());

        let _ = Self::query_inner(cx, &mut client_conn, payload).await?;

        cx.fsm.put_conn(client_conn);

        collect_sql_under_processing_dec!(cx, "COM_QUERY", ep.as_ref().unwrap());
        collect_sql_processed_duration!(cx, "COM_QUERY", ep.as_ref().unwrap(), now.elapsed());

        Ok(RespContext { ep, duration: now.elapsed() })
    }

    async fn prepare(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();
        let sql = std::str::from_utf8(payload).unwrap().trim_matches(char::from(0));

        let mut client_conn = if cx.rewriter.is_some() {
            let (is_get_conn, input_typ, mut rewrite_outputs) = Self::query_rewrite(cx, sql)?;
            route_sharding(input_typ, sql, cx.route_strategy.clone(), &mut rewrite_outputs);
            Self::fsm_trigger(cx, TransEventName::PrepareEvent, RouteInputTyp::Statement, sql)
                .await?
        } else {
            Self::fsm_trigger(cx, TransEventName::PrepareEvent, RouteInputTyp::Statement, sql)
                .await?
        };

        let ep = client_conn.get_endpoint();

        collect_sql_processed_total!(cx, "COM_PREPARE", ep.as_ref().unwrap());
        collect_sql_under_processing_inc!(cx, "COM_PREPARE", ep.as_ref().unwrap());

        let res = Self::prepare_inner(cx, &mut client_conn, payload).await;
        cx.fsm.put_conn(client_conn);

        collect_sql_under_processing_dec!(cx, "COM_PREPARE", ep.as_ref().unwrap());
        collect_sql_processed_duration!(cx, "COM_PREPARE", ep.as_ref().unwrap(), now.elapsed());

        if let Err(ref err) = res {
            if let ErrorKind::Protocol(ProtocolError::PrepareError(data)) = err.kind() {
                cx.framed
                    .send(PacketSend::Encode(data[4..].into()))
                    .await
                    .map_err(ErrorKind::from)?;
            }
        }

        Ok(RespContext { ep, duration: now.elapsed() })
    }

    async fn execute(cx: &mut ReqContext<T, C>, payload: &[u8]) -> Result<RespContext, Error> {
        let now = Instant::now();
        let sess = cx.framed.codec_mut().get_session();
        let mut client_conn = cx.fsm.get_conn(&build_conn_attrs(sess)).await?;
        let ep = client_conn.get_endpoint();

        collect_sql_processed_total!(cx, "COM_EXECUTE", ep.as_ref().unwrap());
        collect_sql_under_processing_inc!(cx, "COM_EXECUTE", ep.as_ref().unwrap());

        let _ = Self::execute_inner(cx, &mut client_conn, payload).await;
        cx.fsm.put_conn(client_conn);

        collect_sql_under_processing_dec!(cx, "COM_EXECUTE", ep.as_ref().unwrap());
        collect_sql_processed_duration!(cx, "COM_EXECUTE", ep.as_ref().unwrap(), now.elapsed());

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

        if cx.rewriter.is_some() {
            Self::sharding_command_not_support(cx, ComType::FIELD_LIST.as_ref()).await?;
            return Ok(RespContext { ep: None, duration: now.elapsed() });
        }

        let mut client_conn =
            Self::fsm_trigger(cx, TransEventName::QueryEvent, RouteInputTyp::None, "").await?;

        let ep = client_conn.get_endpoint();

        collect_sql_processed_total!(cx, "COM_FIELD_LIST", ep.as_ref().unwrap());
        collect_sql_under_processing_inc!(cx, "COM_FIELD_LIST", ep.as_ref().unwrap());

        Self::field_list_inner(cx, &mut client_conn, payload).await?;

        cx.fsm.put_conn(client_conn);

        collect_sql_under_processing_dec!(cx, "COM_FIELD_LIST", ep.as_ref().unwrap());
        collect_sql_processed_duration!(cx, "COM_FIELD_LIST", ep.as_ref().unwrap(), now.elapsed());

        Ok(RespContext { ep, duration: now.elapsed() })
    }
}
