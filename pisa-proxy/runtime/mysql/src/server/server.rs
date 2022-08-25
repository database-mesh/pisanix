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


use pisa_error::error::Error;
use crate::mysql::ReqContext;
use crate::mysql::RespContext;
use std::time::Instant;
use byteorder::LittleEndian;
use mysql_protocol::server::codec::PacketSend;
use strategy::route::RouteInput;
use bytes::BytesMut;
use tokio_util::codec::Encoder;
use tokio_util::codec::Decoder;
use tokio::io::AsyncRead;
use crate::mysql::MySQLService;
use mysql_protocol::util::is_eof;
use pisa_error::error::ErrorKind;
use mysql_protocol::mysql_const::*;
use conn_pool::PoolConn;
use mysql_protocol::client::conn::ClientConn;
use mysql_protocol::server::codec::make_eof_packet;
use mysql_protocol::util::length_encode_int;
use mysql_protocol::err::ProtocolError;
use mysql_protocol::client::codec::ResultsetStream;
use mysql_parser::ast::*;
use mysql_protocol::server::err::MySQLError;
use mysql_protocol::server::codec::make_err_packet;
use mysql_protocol::server::codec::ok_packet;
use crate::transaction_fsm::TransEventName;
use crate::transaction_fsm::TransFsm;
use std::marker::PhantomData;
use mysql_protocol::server::codec::CommonPacket;
use tokio::io::AsyncWrite;
use futures::SinkExt;
use byteorder::ByteOrder;
use futures::StreamExt;
use mysql_protocol::session::SessionMut;
use async_trait::async_trait;
use tracing::error;
use tracing::debug;

pub struct PisaMySQLService<T, C> {
    _phat: PhantomData<(T, C)>,
}

impl<T, C> PisaMySQLService<T, C>
where
    T: AsyncRead + AsyncWrite + Unpin + Send,
    C: Decoder<Item = BytesMut> + Encoder<PacketSend<Box<[u8]>>, Error = ProtocolError> + Send + CommonPacket,
{
    pub fn new() -> Self {
        Self { _phat: PhantomData }
    }

    async fn fsm_trigger(
        fsm: &mut TransFsm,
        state_name: TransEventName,
        input: RouteInput<'_>,
    ) -> Result<PoolConn<ClientConn>, Error> {
        fsm.trigger(state_name, input).await?;
        fsm.get_conn().await
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

    async fn query_inner_get_conn(req: &mut ReqContext<T, C>, sql: &str) -> Result<PoolConn<ClientConn>, Error> {
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
                    req.fsm.get_conn().await
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
                .await?;
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
        let mut client_conn = Self::query_inner_get_conn(cx, sql).await?;

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
        .await?;
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
            Self::fsm_trigger(&mut cx.fsm, TransEventName::QueryEvent, RouteInput::None).await?;
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