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

use std::{str, sync::Arc, time::SystemTime};

use byteorder::{ByteOrder, LittleEndian};
use bytes::{Buf, BufMut, BytesMut};
use common::ast_cache::ParserAstCache;
use conn_pool::Pool;
use futures::StreamExt;
use loadbalance::balance::BalanceType;
use mysql_parser::{
    ast::{BeginStmt, SqlStmt, SetOptValues, SetOpts},
    parser::{ParseError, Parser},
};
use mysql_protocol::{
    client::{codec::ResultsetStream, conn::{ClientConn, self}},
    err::ProtocolError,
    mysql_const::*,
    server::{conn::Connection, err::MySQLError},
    util::*,
};
use parking_lot::Mutex as plMutex;
use plugin::{build_phase::PluginPhase, err::BoxError, layer::Service};
use proxy::proxy::ProxyConfig;
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};
use tracing::{debug, error};

use crate::{server::metrics::*, transaction_fsm::*};

pub struct MySqlServer {
    // TODO: this should be a common property of proxy runtime
    pub name: String,
    pub metrics_collector: MySqlServerMetricsCollector,
    pub client: Connection,
    pub buf: BytesMut,

    mysql_parser: Arc<Parser>,
    trans_fsm: TransFsm,
    ast_cache: Arc<plMutex<ParserAstCache>>,
    plugin: Option<PluginPhase>,
    is_quit: bool,
    // `concurrency_control_rule_idx` is index of concurrency_control rules
    // `concurrency_control_rule_idx` is required to add permits when the concurrency_control layer service is enabled
    concurrency_control_rule_idx: Option<usize>,
}

pub struct MySqlServerBuilder {
    _name: String,
    _socket: TcpStream,
    _pcfg: ProxyConfig,
    _buf: BytesMut,
    _mysql_parser: Arc<Parser>,
    _ast_cache: Arc<plMutex<ParserAstCache>>,
    _is_quit: bool,
    _concurrency_control_rule_idx: Option<usize>,
    _metrics_collector: MySqlServerMetricsCollector,
    _lb: Arc<Mutex<BalanceType>>,
    _pool: Pool<ClientConn>,
    _plugin: Option<PluginPhase>,
}

impl MySqlServerBuilder {
    pub fn new(
        socket: TcpStream,
        lb: Arc<Mutex<BalanceType>>,
        plugin: Option<PluginPhase>,
    ) -> MySqlServerBuilder {
        MySqlServerBuilder {
            _name: String::new(),
            _pcfg: ProxyConfig::default(),
            _socket: socket,
            _buf: BytesMut::new(),
            _mysql_parser: Arc::new(Parser::new()),
            _ast_cache: Arc::new(plMutex::new(ParserAstCache::new())),
            _is_quit: false,
            _concurrency_control_rule_idx: None,
            _metrics_collector: MySqlServerMetricsCollector::new(),
            _lb: lb,
            _plugin: plugin,
            _pool: Pool::new(1),
        }
    }    

    pub fn with_pool(mut self, pool: Pool<ClientConn>) -> MySqlServerBuilder {
        self._pool = pool;
        self
    }

    pub fn with_pcfg(mut self, pcfg: ProxyConfig) -> MySqlServerBuilder {
        self._pcfg = pcfg;
        self
    }

    pub fn with_buf(mut self, buf: BytesMut) -> MySqlServerBuilder {
        self._buf = buf;
        self
    }

    pub fn with_mysql_parser(mut self, parser: Arc<Parser>) -> MySqlServerBuilder {
        self._mysql_parser = parser;
        self
    }

    pub fn with_ast_cache(mut self, cache: Arc<plMutex<ParserAstCache>>) -> MySqlServerBuilder {
        self._ast_cache = cache;
        self
    }

    pub fn is_quit(mut self, quit: bool) -> MySqlServerBuilder {
        self._is_quit = quit;
        self
    }

    pub fn with_concurrency_control_rule_idx(mut self, idx: Option<usize>) -> MySqlServerBuilder {
        self._concurrency_control_rule_idx = idx;
        self
    }

    pub fn with_metrics_collector(
        mut self,
        collector: MySqlServerMetricsCollector,
    ) -> MySqlServerBuilder {
        self._metrics_collector = collector;
        self
    }

    pub fn build(self) -> MySqlServer {
        MySqlServer {
            client: Connection::new(
                self._socket,
                self._pcfg.user,
                self._pcfg.password,
                self._pcfg.db,
            ),
            buf: self._buf,
            mysql_parser: self._mysql_parser,
            trans_fsm: TransFsm::new_trans_fsm(self._lb, self._pool),
            ast_cache: self._ast_cache,
            plugin: self._plugin,
            is_quit: self._is_quit,
            concurrency_control_rule_idx: self._concurrency_control_rule_idx,
            metrics_collector: self._metrics_collector,
            name: self._pcfg.name,
        }
    }
}

impl MySqlServer {
    pub async fn new(
        client: TcpStream,
        pool: Pool<ClientConn>,
        lb: Arc<Mutex<BalanceType>>,
        proxy_config: ProxyConfig,
        parser: Arc<Parser>,
        ast_cache: Arc<plMutex<ParserAstCache>>,
        plugin: Option<PluginPhase>,
        metrics_collector: MySqlServerMetricsCollector,
    ) -> MySqlServer {
        MySqlServer {
            client: Connection::new(
                client,
                proxy_config.user,
                proxy_config.password,
                proxy_config.db,
            ),
            buf: BytesMut::with_capacity(8192),
            mysql_parser: parser,
            trans_fsm: TransFsm::new_trans_fsm(lb, pool),
            ast_cache,
            plugin,
            is_quit: false,
            concurrency_control_rule_idx: None,
            metrics_collector,
            name: proxy_config.name,
        }
    }

    pub async fn handshake(&mut self) -> Result<(), ProtocolError> {
        if let Err(err) = self.client.handshake().await {
            if let ProtocolError::AuthFailed(err) = err {
                return self.client.pkt.write_buf(&err).await.map_err(ProtocolError::Io);
            }
            return Err(err);
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), ProtocolError> {
        if let Err(err) = self.trans_fsm.trigger(TransEventName::DummyEvent).await {
            //TODO: need refactor
            error!("err: {:?}", err);
        };

        // set db to trans_fsm
        self.trans_fsm.set_db(self.client.db.clone());

        let mut buf = BytesMut::with_capacity(4096);

        loop {
            self.client.pkt.sequence = 0;

            let length = match self.client.pkt.read_packet_buf(&mut buf).await {
                Err(err) => return Err(ProtocolError::Io(err)),
                Ok(length) => length,
            };

            if self.is_quit {
                return Ok(());
            }

            if length == 0 {
                //TODO
                //KNOWN ISSUE
                return Ok(());
            }

            if let Err(err) = self.handle_command(&mut buf).await {
                error!("exec command err: {:?}", err);
            };

            if let Some(idx) = &self.concurrency_control_rule_idx {
                self.plugin.as_mut().unwrap().concurrency_control.add_permits(*idx);
                self.concurrency_control_rule_idx = None;
            }
        }
    }

    pub async fn handle_command(&mut self, data: &mut BytesMut) -> Result<(), ProtocolError> {
        let cmd = data.get_u8();
        let payload = data.split();

        if let Err(err) = self.plugin_run(&payload) {
            return self.handle_err(err.to_string()).await;
        }

        match cmd {
            COM_INIT_DB => self.handle_init_db(&payload, true).await,
            COM_QUERY => self.handle_query(&payload).await,
            COM_FIELD_LIST => self.handle_field_list(&payload).await,
            COM_QUIT => self.handle_quit().await,
            COM_PING => self.handle_ok().await,
            COM_STMT_PREPARE => self.handle_prepare(&payload).await,
            COM_STMT_EXECUTE => self.handle_execute(&payload).await,
            COM_STMT_CLOSE => self.handle_stmt_close(&payload).await,
            COM_STMT_RESET => self.handle_ok().await,
            _ => self.handle_err(format!("command {} not support", cmd)).await,
        }
    }

    pub async fn handle_init_db(
        &mut self,
        payload: &[u8],
        is_send_ok: bool,
    ) -> Result<(), ProtocolError> {
        let earlier = SystemTime::now();
        if let Err(err) = self.trans_fsm.trigger(TransEventName::UseEvent).await {
            error!("err:{:?}", err);
        }
        let mut client_conn = self.trans_fsm.get_conn().await.unwrap();
        collect_sql_processed_total!(
            self,
            "COM_INIT_DB",
            client_conn.get_endpoint().unwrap().as_str()
        );
        collect_sql_under_processing_inc!(
            self,
            "COM_INIT_DB",
            client_conn.get_endpoint().unwrap().as_str()
        );
        let sql = str::from_utf8(payload).unwrap().trim_matches(char::from(0));

        self.trans_fsm.set_db(sql.to_string());
        let res = client_conn.send_use_db(sql).await?;
        let ep = client_conn.get_endpoint().unwrap();
        self.trans_fsm.put_conn(client_conn);
        collect_sql_under_processing_dec!(self, "COM_INIT_DB", ep.as_str());
        collect_sql_processed_duration!(self, "COM_INIT_DB", ep.as_str(), earlier);

        if res.1 {
            if is_send_ok {
                self.client.pkt.write_ok().await.map_err(ProtocolError::Io)
            } else {
                Ok(())
            }
        } else {
            // supports CLIENT_PROTOCOL_41 default
            // skip sql_state_marker and sql_state packet
            let err_info = self.client.pkt.make_err_packet(MySQLError::new(
                1049,
                "42000".as_bytes().to_vec(),
                String::from_utf8_lossy(&res.0[13..]).to_string(),
            ));
            self.client.pkt.write_buf(&err_info).await.map_err(ProtocolError::Io)
        }
    }

    pub async fn handle_field_list(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let earlier = SystemTime::now();
        if let Err(err) = self.trans_fsm.trigger(TransEventName::QueryEvent).await {
            error!("err: {:?}", err);
        }

        let mut client_conn = self.trans_fsm.get_conn().await.unwrap();
        collect_sql_processed_total!(
            self,
            "COM_FIELD_LIST",
            client_conn.get_endpoint().unwrap().as_str()
        );
        collect_sql_under_processing_inc!(
            self,
            "COM_FIELD_LIST",
            client_conn.get_endpoint().unwrap().as_str()
        );
        let mut stream = client_conn.send_common_command(COM_FIELD_LIST, payload).await?;

        let mut buf = BytesMut::with_capacity(128);

        loop {
            let mut data = match stream.next().await {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            self.client.pkt.construct_packet_buf(&mut data, &mut buf).await;

            if is_eof(&data) {
                break;
            }
        }

        self.client.pkt.write_buf(&buf).await?;
        let ep = client_conn.get_endpoint().unwrap();
        self.trans_fsm.put_conn(client_conn);
        collect_sql_under_processing_dec!(self, "COM_FIELD_LIST", ep.as_str());
        collect_sql_processed_duration!(self, "COM_FIELD_LIST", ep.as_str(), earlier);
        Ok(())
    }

    pub async fn handle_prepare(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let earlier = SystemTime::now();
        if let Err(err) = self.trans_fsm.trigger(TransEventName::PrepareEvent).await {
            error!("error: {:?}", err);
        };

        let mut client_conn = self.trans_fsm.get_conn().await.unwrap();
        collect_sql_processed_total!(
            self,
            "COM_PREPARE",
            client_conn.get_endpoint().unwrap().as_str()
        );
        collect_sql_under_processing_inc!(
            self,
            "COM_PREPARE",
            client_conn.get_endpoint().unwrap().as_str()
        );
        let try_stmt = client_conn.send_prepare(payload).await;
        if let Err(ProtocolError::PrepareError(mut data)) = try_stmt {
            self.client.pkt.make_packet_header(data.len() - 4, &mut data);
            self.trans_fsm.put_conn(client_conn);
            return self.client.pkt.write_buf(&data).await.map_err(ProtocolError::Io);
        }

        let stmt = try_stmt.unwrap();
        let ep = client_conn.get_endpoint().unwrap();
        self.trans_fsm.put_conn(client_conn);
        collect_sql_under_processing_dec!(self, "COM_PREPARE", ep.as_str());
        collect_sql_processed_duration!(self, "COM_PREPARE", ep.as_str(), earlier);

        let mut data = BytesMut::from(&vec![0; 4][..]);
        data.put_u8(0);
        data.extend_from_slice(&u32::to_le_bytes(stmt.stmt_id));
        data.extend_from_slice(&u16::to_le_bytes(stmt.cols_count));
        data.extend_from_slice(&u16::to_le_bytes(stmt.params_count));

        data.extend_from_slice(&[0, 0, 0]);

        self.client.pkt.make_packet_header(data.len() - 4, &mut data);

        if !stmt.params_data.is_empty() {
            for mut param_data in stmt.params_data {
                self.client.pkt.make_packet_header(param_data.len() - 4, &mut param_data);
                data.extend_from_slice(&param_data);
            }

            data.extend_from_slice(&self.client.pkt.make_eof_packet());
        }

        if !stmt.cols_data.is_empty() {
            for mut col_data in stmt.cols_data {
                self.client.pkt.make_packet_header(col_data.len() - 4, &mut col_data);
                data.extend_from_slice(&col_data);
            }

            data.extend_from_slice(&self.client.pkt.make_eof_packet());
        }

        self.client.pkt.write_buf(&data).await?;
        Ok(())
    }

    pub async fn handle_query(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let earlier = SystemTime::now();
        if let Err(err) = self.trans_fsm.trigger(TransEventName::QueryEvent).await {
            error!("err:{:?}", err);
        }
        let mut client_conn = self.trans_fsm.get_conn().await.unwrap();
        collect_sql_processed_total!(
            self,
            "COM_QUERY",
            client_conn.get_endpoint().unwrap().as_str()
        );
        collect_sql_under_processing_inc!(
            self,
            "COM_QUERY",
            client_conn.get_endpoint().unwrap().as_str()
        );

        let stream = match self.get_ast(payload) {
            Err(err) => {
                error!("err: {:?}", err);
                client_conn.send_query(payload).await?
            }

            Ok(stmt) => match &stmt[0] {
                SqlStmt::Set(stmt) => {
                    self.handle_set_stmt(stmt);
                    client_conn.send_query(payload).await?
                },
                //TODO: split sql stmt for sql audit
                SqlStmt::BeginStmt(_stmt) => client_conn.send_query(payload).await?,
                _ => client_conn.send_query(payload).await?,
            },
        };
        
        self.handle_query_resultset(stream).await?;

        let ep = client_conn.get_endpoint().unwrap();
        self.trans_fsm.put_conn(client_conn);
        collect_sql_under_processing_dec!(self, "COM_QUERY", ep.as_str());
        collect_sql_processed_duration!(self, "COM_QUERY", ep.as_str(), earlier);
        Ok(())
    }

    // Set charset name 
    fn handle_set_stmt(&mut self, stmt: &SetOptValues) {
        match stmt {
            SetOptValues::OptValues(vals) => {
                match &vals.opt {
                    SetOpts::SetNames(name) => {
                        if let Some(name) = &name.charset_name {
                            self.client.charset = name.clone();
                            self.trans_fsm.set_charset(name.clone())
                        }
                    },
                    _ => {}
                }
            },

            _ => {}
        }
    }

    pub async fn handle_query_resultset<'b>(
        &mut self,
        mut stream: ResultsetStream<'b>,
    ) -> Result<(), ProtocolError> {
        let data = stream.next().await;

        let mut header = match data {
            Some(Ok(data)) => data,
            Some(Err(e)) => return Err(e),
            None => return Ok(()),
        };

        let ok_or_err = header[4];

        if ok_or_err == OK_HEADER || ok_or_err == ERR_HEADER {
            self.client.pkt.write_buf(&header).await?;
            return Ok(());
        }

        let (cols, ..) = length_encode_int(&header[4..]);
        // first clear buf
        self.buf.clear();

        self.client.pkt.construct_packet_buf(&mut header, &mut self.buf).await;

        for _ in 0..cols {
            let data = stream.next().await;
            let mut data = match data {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            self.client.pkt.construct_packet_buf(&mut data, &mut self.buf).await;
        }

        // read eof
        let _ = stream.next().await;

        self.buf.extend_from_slice(&self.client.pkt.make_eof_packet());

        loop {
            let data = stream.next().await;

            let mut row = match data {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            if is_eof(&row) {
                break;
            }

            self.client.pkt.construct_packet_buf(&mut row, &mut self.buf).await;
        }

        self.buf.extend_from_slice(&self.client.pkt.make_eof_packet());
        self.client.pkt.write_buf(&self.buf).await.map_err(ProtocolError::Io)?;

        Ok(())
    }

    pub async fn handle_execute(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let earlier = SystemTime::now();
        let mut client_conn = self.trans_fsm.get_conn().await.unwrap();
        collect_sql_processed_total!(
            self,
            "COM_EXECUTE",
            client_conn.get_endpoint().unwrap().as_str()
        );
        collect_sql_under_processing_inc!(
            self,
            "COM_EXECUTE",
            client_conn.get_endpoint().unwrap().as_str()
        );
        let stream = client_conn.send_execute(payload).await?;
        self.handle_query_resultset(stream).await?;
        let ep = client_conn.get_endpoint().unwrap();
        self.trans_fsm.put_conn(client_conn);
        collect_sql_under_processing_dec!(self, "COM_EXECUTE", ep.as_str());
        collect_sql_processed_duration!(self, "COM_EXECUTE", ep.as_str(), earlier);
        Ok(())
    }

    pub async fn handle_ok(&mut self) -> Result<(), ProtocolError> {
        self.client.pkt.write_ok().await.map_err(ProtocolError::Io)
    }

    pub async fn handle_err(&mut self, msg: String) -> Result<(), ProtocolError> {
        let err_info = self.client.pkt.make_err_packet(MySQLError::new(
            1047,
            "08S01".as_bytes().to_vec(),
            msg,
        ));

        self.client.pkt.write_buf(&err_info).await.map_err(ProtocolError::Io)
    }

    pub async fn handle_stmt_close(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let stmt_id = LittleEndian::read_u32(payload);
        debug!("stmt close {:?}", stmt_id);

        Ok(())
    }

    pub async fn handle_quit(&mut self) -> Result<(), ProtocolError> {
        self.is_quit = true;
        self.client.pkt.conn.shutdown().await.map_err(ProtocolError::Io)
    }

    // TODO, add handle for begin stmt
    async fn _handle_begin_stmt<'b>(
        &mut self,
        stream: ResultsetStream<'b>,
        _stmt: &BeginStmt,
    ) -> Result<(), ProtocolError> {
        if let Err(err) = self.trans_fsm.trigger(TransEventName::StartEvent).await {
            error!("err: {:?}", err);
        }
        self.handle_query_resultset(stream).await
    }

    fn get_ast(&mut self, payload: &[u8]) -> Result<Vec<SqlStmt>, ParseError> {
        let sql = str::from_utf8(payload).unwrap();
        let mut ast_cache = self.ast_cache.lock();
        let try_ast = ast_cache.get(sql.to_string());

        match try_ast {
            Some(stmt) => Ok(stmt.to_vec()),
            None => match self.mysql_parser.parse(sql) {
                Err(err) => Err(err[0].clone()),
                Ok(stmt) => {
                    ast_cache.set(sql.to_string(), stmt.clone());
                    Ok(stmt)
                }
            },
        }
    }

    fn plugin_run(&mut self, payload: &[u8]) -> Result<(), BoxError> {
        if let Some(plugin) = self.plugin.as_mut() {
            let input = unsafe { String::from(str::from_utf8_unchecked(payload)) };

            plugin.circuit_break.handle(input.clone())?;

            let res = plugin.concurrency_control.handle(input);

            match res {
                Ok(data) => {
                    self.concurrency_control_rule_idx = data.0;
                    return Ok(());
                }

                Err(err) => return Err(err),
            }
        }

        Ok(())
    }
}
