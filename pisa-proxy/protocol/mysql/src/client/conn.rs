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

use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use bytes::BytesMut;
use conn_pool::{ConnAttr, ConnAttrMut, ConnLike};
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

use super::{
    auth::{handshake, ClientAuth},
    codec::{ClientCodec, CommonStream, QueryResultStream, ResultsetStream},
    resultset::ResultSendCommand,
    stmt::Stmt,
    stream::LocalStream,
};
use crate::{
    column::{Column, ColumnInfo},
    err::ProtocolError,
    mysql_const::*,
    row::{RowDataText, RowDataTyp},
    util::{is_ok_header, BufExt},
};

#[derive(Debug, Default)]
pub struct ClientConn {
    pub framed: Option<Box<ClientCodec>>,
    user: String,
    password: String,
    endpoint: String,
}

impl ClientConn {
    pub fn with_opts(user: String, password: String, endpoint: String) -> ClientConn {
        ClientConn { user, password, endpoint, ..Default::default() }
    }

    #[cfg(test)]
    pub async fn test_conn(
        user: String,
        password: String,
        endpoint: String,
    ) -> Result<ClientConn, ProtocolError> {
        let conn = ClientConn { user, password, endpoint, ..Default::default() };
        conn.connect().await
    }

    pub async fn connect(&self) -> Result<ClientConn, ProtocolError> {
        let sock = TcpStream::connect(&self.endpoint).await?;
        sock.set_nodelay(true).unwrap();

        let local_stream = LocalStream::from(sock);

        let mut auth_codec = ClientAuth::new();
        auth_codec.user = self.user.clone();
        auth_codec.password = self.password.clone();
        auth_codec.tls_config = None;

        let mut framed = Some(Box::new(ClientCodec::ClientAuth(Framed::with_capacity(
            local_stream,
            auth_codec,
            16384,
        ))));

        let res = handshake(*(framed.take().unwrap())).await?;
        let framed = Some(Box::new(ClientCodec::ClientAuth(res.0)));

        Ok(ClientConn {
            user: self.user.clone(),
            password: self.password.clone(),
            endpoint: self.endpoint.clone(),
            framed,
        })
    }

    pub async fn handshake(&mut self) -> Result<(bool, Vec<u8>), ProtocolError> {
        let res = handshake(*(self.framed.take().unwrap())).await?;
        self.framed = Some(Box::new(ClientCodec::ClientAuth(res.0)));

        Ok((res.1, res.2))
    }

    pub async fn send_query<'a>(
        &'a mut self,
        val: &'a [u8],
    ) -> Result<ResultsetStream<'a>, ProtocolError> {
        let framed = self.framed.take().unwrap();

        let mut resultset_codec = framed.into_resultset();

        resultset_codec.send(ResultSendCommand::Binary((COM_QUERY, val))).await?;

        self.framed = Some(Box::new(ClientCodec::Resultset(resultset_codec)));

        Ok(ResultsetStream::new(self.framed.as_mut()))
    }

    pub async fn send_prepare<'a>(&'a mut self, val: &[u8]) -> Result<Stmt, ProtocolError> {
        let framed = self.framed.take().unwrap();

        let mut stmt_codec = framed.into_stmt();
        stmt_codec.send((COM_STMT_PREPARE, val)).await?;

        loop {
            // Decode prepare complete
            if stmt_codec.codec().is_complete() {
                break;
            }

            match stmt_codec.next().await {
                Some(Ok(None)) => {}

                Some(Ok(Some(data))) => {
                    // If data.len() > 0, means that `Prepare` return error.
                    if !data.is_empty() {
                        self.framed = Some(Box::new(ClientCodec::Stmt(stmt_codec)));
                        return Err(ProtocolError::PrepareError(data.to_vec()));
                    }
                }

                Some(Err(e)) => {
                    self.framed = Some(Box::new(ClientCodec::Stmt(stmt_codec)));
                    return Err(e);
                }

                None => {}
            }
        }

        let stmt = stmt_codec.codec().clone();
        self.framed = Some(Box::new(ClientCodec::Stmt(stmt_codec)));

        Ok(stmt)
    }

    pub async fn send_execute<'a>(
        &'a mut self,
        val: &'a [u8],
    ) -> Result<ResultsetStream<'a>, ProtocolError> {
        let framed = self.framed.take().unwrap();
        let mut resultset_codec = framed.into_resultset();
        resultset_codec.codec_mut().with_binary(true);

        resultset_codec.send(ResultSendCommand::Binary((COM_STMT_EXECUTE, val))).await?;

        self.framed = Some(Box::new(ClientCodec::Resultset(resultset_codec)));

        Ok(ResultsetStream::new(self.framed.as_mut()))
    }

    pub async fn send_use_db<'a>(
        &'a mut self,
        val: &str,
    ) -> Result<(BytesMut, bool), ProtocolError> {
        let framed = self.framed.take().unwrap();
        let mut common_codec = framed.into_common();

        common_codec.send((COM_INIT_DB, val.as_bytes())).await?;

        let res = match common_codec.next().await {
            Some(Ok(data)) => {
                if is_ok_header(data.0[4]) {
                    common_codec.codec_mut().auth_info.as_mut().unwrap().db = val.to_string();
                    Ok((data.0, true))
                } else {
                    Ok((data.0, false))
                }
            }

            Some(Err(e)) => Err(e),

            _ => unreachable!(),
        };

        self.framed = Some(Box::new(ClientCodec::Common(common_codec)));
        res
    }

    pub async fn send_ping(&mut self) -> Result<bool, ProtocolError> {
        let framed = self.framed.take().unwrap();
        let mut common_codec = framed.into_common();

        common_codec.send((COM_PING, &[])).await?;
        let res = match common_codec.next().await {
            Some(Ok(data)) => {
                if is_ok_header(data.0[4]) {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Some(Err(e)) => Err(e),

            _ => Ok(false),
        };

        self.framed = Some(Box::new(ClientCodec::Common(common_codec)));
        res
    }

    // Send PING, QUIT,etc... command
    pub async fn send_common_command<'a>(
        &'a mut self,
        code: u8,
        val: &'a [u8],
    ) -> Result<CommonStream<'a>, ProtocolError> {
        let framed = self.framed.take().unwrap();
        let mut common_codec = framed.into_common();

        common_codec.send((code, val)).await?;

        self.framed = Some(Box::new(ClientCodec::Common(common_codec)));

        Ok(CommonStream::new(self.framed.as_mut()))
    }

    pub async fn query_result<'a>(
        &'a mut self,
        val: &'a [u8],
    ) -> Result<Option<QueryResultStream<'a, BytesMut>>, ProtocolError> {
        let mut stream = self.send_query(val).await?;

        let mut header = match stream.next().await {
            Some(Ok(data)) => data,
            Some(Err(e)) => return Err(e),
            None => return Ok(None),
        };

        if header[4] == OK_HEADER || header[4] == ERR_HEADER {
            return Ok(None);
        }

        let  _ = header.split_to(4);
        let (cols, ..) = header.get_lenc_int();

        let mut col_buf = vec![];
        for _ in 0..cols {
            let data = stream.next().await;
            let data = match data {
                Some(Ok(data)) => data,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            col_buf.extend_from_slice(&data)
        }

        // read eof
        let _ = stream.next().await;

        let col_info = col_buf.as_slice().decode_columns();

        let arc_col_info: Arc<[ColumnInfo]> = col_info.into_boxed_slice().into();

        let is_binary = stream.is_binary();
        match is_binary {
            false => {
                let row_data_text = RowDataText::new(arc_col_info, BytesMut::new());
                let row_data = RowDataTyp::Text(row_data_text);
                Ok(Some(QueryResultStream::new(stream, row_data)))
            }
            true => todo!(),
        }
    }

    // Send query, but discard result
    pub async fn send_query_discard_result(&mut self, val: &str) -> Result<(), ProtocolError> {
        let mut stream = self.send_common_command(COM_QUERY, val.as_bytes()).await?;

        while stream.next().await.is_some() {}

        Ok(())
    }

    pub async fn send_query_without_stream<'a>(
        &'a mut self,
        val: &'a [u8],
    ) -> Result<(), ProtocolError> {
        let framed = self.framed.take().unwrap();

        let mut resultset_codec = framed.into_resultset();

        resultset_codec.send(ResultSendCommand::Binary((COM_QUERY, val))).await?;

        self.framed = Some(Box::new(ClientCodec::Resultset(resultset_codec)));

        Ok(())
    }

    pub async fn send_execute_without_stream<'a>(
        &'a mut self,
        val: &'a [u8],
    ) -> Result<(), ProtocolError> {
        let framed = self.framed.take().unwrap();
        let mut resultset_codec = framed.into_resultset();
        resultset_codec.codec_mut().with_binary(true);

        resultset_codec.send(ResultSendCommand::Binary((COM_STMT_EXECUTE, val))).await?;

        self.framed = Some(Box::new(ClientCodec::Resultset(resultset_codec)));

        Ok(())
    }

    pub async fn is_ready(&self) -> bool {
        self.framed.as_ref().unwrap().is_ready().await
    }

    pub fn get_endpoint(&self) -> Option<String> {
        Some(self.endpoint.clone())
    }

    pub fn set_charset(&mut self, name: &str) {
        self.framed.as_mut().unwrap().charset = name.to_string()
    }

    pub fn set_autocommit(&mut self, status: &str) {
        self.framed.as_mut().unwrap().auotcommit = Some(status.to_string())
    }
}

impl Clone for ClientConn {
    fn clone(&self) -> Self {
        ClientConn {
            user: self.user.clone(),
            password: self.password.clone(),
            endpoint: self.endpoint.clone(),
            framed: None,
        }
    }
}

/// Implements `ConnLike` trait
#[async_trait]
impl ConnLike for ClientConn {
    type Error = ProtocolError;

    async fn build_conn(&self) -> Result<Self, Self::Error> {
        Ok(self.connect().await?)
    }
}

/// Implements `ConnAttr` trait
impl ConnAttr for ClientConn {
    fn get_host(&self) -> String {
        let sock_addr: SocketAddr = self.endpoint.parse().unwrap();
        sock_addr.ip().to_string()
    }

    fn get_port(&self) -> u16 {
        let sock_addr: SocketAddr = self.endpoint.parse().unwrap();
        sock_addr.port()
    }

    fn get_user(&self) -> String {
        self.user.clone()
    }

    fn get_endpoint(&self) -> String {
        self.endpoint.clone()
    }

    fn get_db(&self) -> Option<String> {
        let codec = self.framed.as_ref();

        if let Some(codec) = codec {
            if codec.db.is_empty() {
                None
            } else {
                Some(codec.db.clone())
            }
        } else {
            None
        }
    }

    fn get_charset(&self) -> Option<String> {
        let codec = self.framed.as_ref();
        if let Some(codec) = codec {
            Some(codec.charset.clone())
        } else {
            None
        }
    }

    fn get_autocommit(&self) -> Option<String> {
        let codec = self.framed.as_ref();
        if let Some(codec) = codec {
            codec.auotcommit.clone()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SessionAttr {
    DB(Option<String>),
    Charset(String),
    Autocommit(Option<String>),
}

#[async_trait]
impl ConnAttrMut for ClientConn {
    type Item = SessionAttr;
    async fn init(&mut self, items: &[SessionAttr]) {
        for attr in items.iter() {
            match attr {
                SessionAttr::DB(val) => {
                    if val != &self.get_db() {
                        if let Some(db) = val {
                            let _ = self.send_use_db(db).await;
                        }
                    }
                }
                SessionAttr::Charset(val) => {
                    if Some(val) != self.get_charset().as_ref() {
                        self.set_charset(val);
                        let _ = self.send_query_discard_result(&format!("SET NAMES {}", val)).await;
                    }
                }
                SessionAttr::Autocommit(val) => {
                    if val != &self.get_autocommit() {
                        if let Some(val) = val {
                            self.set_autocommit(val);
                            let _ = self
                                .send_query_discard_result(&format!("SET AUTOCOMMIT = {}", val))
                                .await;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use conn_pool::*;
    use tokio_stream::StreamExt;

    use crate::{client::conn::ClientConn, err::ProtocolError, row::RowData};

    #[tokio::test]
    async fn pool() {
        let user = "root".to_string();
        let password = "123456".to_string();
        let addr = "127.0.0.1:13306".to_string();

        let conn = ClientConn::with_opts(user, password, addr.clone());

        let mut pool = Pool::new(3);
        pool.set_factory(conn);

        assert_eq!(0, pool.len(&addr));

        {
            let mut conn = pool.get_conn_with_endpoint(&addr).await.unwrap();
            let res = conn.send_query("show databases".as_bytes()).await;
            let mut res = res.unwrap();
            loop {
                match res.next().await {
                    Some(data) => match data {
                        Ok(data) => {
                            println!("data {:?}", data);
                        }

                        Err(e) => {
                            if let ProtocolError::PacketError(data) = e {
                                println!("{:?}", String::from_utf8_lossy(&data));
                                break;
                            }
                        }
                    },
                    _ => break,
                }
            }

            assert_eq!("127.0.0.1", &conn.get_host());
            assert_eq!(13306, conn.get_port());
            assert_eq!("root", &conn.get_user());
        }

        assert_eq!(1, pool.len(&addr));
    }

    #[tokio::test]
    async fn test_query_result() {
        let mut driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "127.0.0.1:13307".to_string(),
        )
        .await
        .unwrap();

        let query = "select user,host from mysql.user".as_bytes();
        let mut res = driver.query_result(query).await.unwrap().unwrap();

        while let Some(data) = res.next().await {
            let mut row = data.unwrap();

            let user = row.decode_with_name::<String>("user");
            let host = row.decode_with_name::<String>("host");

            assert!(user.is_ok());
            assert!(host.is_ok());
        }
    }
}