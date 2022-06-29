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
    io::Error,
    str,
    sync::atomic::{AtomicU32, Ordering},
};

use bytes::{Buf, BufMut, BytesMut};
use tokio::{io::BufStream, net::TcpStream};
use tracing::debug;

use super::{err::MySQLError, stream::LocalStream};
use crate::{charset::*, err, err::ProtocolError, mysql_const::*, server::packet::Packet, util::*};

lazy_static! {
    static ref CONNECTION_ID: AtomicU32 = AtomicU32::new(0);
}

const DEFAULT_CAPABILITY: u32 = CLIENT_LONG_PASSWORD
    | CLIENT_LONG_FLAG
    | CLIENT_CONNECT_WITH_DB
    | CLIENT_PROTOCOL_41
    | CLIENT_TRANSACTIONS
    | CLIENT_SECURE_CONNECTION
    | CLIENT_SSL
    | CLIENT_FOUND_ROWS
    | CLIENT_MULTI_STATEMENTS
    | CLIENT_PS_MULTI_RESULTS
    | CLIENT_LOCAL_FILES
    | CLIENT_CONNECT_ATTRS
    | CLIENT_PLUGIN_AUTH
    | CLIENT_INTERACTIVE
    | CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA;

// client <=> Pisa
pub struct Connection {
    salt: Vec<u8>,
    status: u16,
    capability: u32,
    connection_id: u32,
    user: String,
    password: String,
    auth_data: BytesMut,
    pub auth_plugin_name: String,
    pub charset: String,
    pub db: String,
    pub affected_rows: i64,
    pub pkt: Packet,
    pub autocommit: Option<String>,
    server_version: String,
}

impl Connection {
    //TODO: need refactor
    pub fn new(
        socket: TcpStream,
        user: String,
        password: String,
        db: String,
        server_version: String,
    ) -> Connection {
        CONNECTION_ID.fetch_add(1, Ordering::Relaxed);

        Connection {
            salt: crate::util::random_buf(20),
            status: SERVER_STATUS_AUTOCOMMIT,
            capability: 0,
            connection_id: CONNECTION_ID.load(Ordering::Relaxed),
            charset: DEFAULT_CHARSET_NAME.to_string(),
            auth_plugin_name: "".to_string(),
            user,
            password,
            db,
            auth_data: BytesMut::new(),
            affected_rows: 0,
            //pkt: Packet::new(Arc::new(Mutex::new(BufStream::new(socket)))),
            pkt: Packet::new(BufStream::new(LocalStream::from(socket))),
            server_version,
            autocommit: None,
        }
    }

    pub async fn handshake(&mut self) -> Result<(), ProtocolError> {
        match self.write_initial_handshake().await {
            Err(err) => return Err(err),
            Ok(_) => debug!("handshake write initial ok"),
        }

        match self.read_handshake_response().await {
            Err(err) => {
                return Err(err);
            }
            Ok(_) => {
                self.pkt.write_ok().await?;
                debug!("handshake response ok")
            }
        }

        //match self.write_ok().await {
        //    Err(err) => return Err(err::ProtocolError::Io(err)),
        //    Ok(_) => info!("write ok"),
        //}

        self.pkt.sequence = 0;

        Ok(())
    }

    pub async fn write_initial_handshake(&mut self) -> Result<(), ProtocolError> {
        let mut data = BytesMut::with_capacity(4);

        // init header
        data.put_bytes(0, 4);

        // min version 10
        data.put_u8(10);

        // server version
        data.extend_from_slice(self.server_version.as_bytes());
        data.put_u8(0);

        // connection id
        data.extend_from_slice(&self.connection_id.to_le_bytes());

        // auth-plugin-data-part-1
        // data.extend_from_slice(&mut self.salt[0..8]);
        data.extend_from_slice(&self.salt[0..8]);

        // filter[00]
        data.put_u8(0);

        // capability flag lower 2 bytes, using default capability here
        data.put_u8(DEFAULT_CAPABILITY as u8);
        data.put_u8((DEFAULT_CAPABILITY >> 8) as u8);

        //charset, utf-8 default
        data.put_u8(CHARSET_NAME_ID_MYSQL5[&*self.charset]);

        //status
        data.put_u8(self.status as u8);
        data.put_u8((self.status >> 8) as u8);

        //below 13 byte may not be used
        //capability flag upper 2 bytes, using default capability here
        data.put_u8((DEFAULT_CAPABILITY >> 16) as u8);
        data.put_u8((DEFAULT_CAPABILITY >> 24) as u8);

        // fiter [0x15], for wireshark dump, value is 0x15
        // data.push(0x15);
        data.put_u8(20 + 1);

        //reserved 10 [00]
        //TODO:
        data.put_bytes(0, 10);

        //auth-plugin-data-part-2
        // data.extend_from_slice(&mut self.salt[8..]);
        data.extend_from_slice(&self.salt[8..]);

        //filter [00]
        data.put_u8(0);

        data.extend_from_slice(AUTH_NATIVE_PASSWORD.as_bytes());
        data.put_u8(0);

        self.pkt.make_packet_header(data.len() - 4, &mut data);
        self.pkt.write_buf(&data).await
    }

    async fn read_handshake_response(&mut self) -> Result<(), ProtocolError> {
        let mut data = BytesMut::with_capacity(128);
        'start: loop {
            self.pkt.read_packet_buf(&mut data).await?;
            if data.is_empty() {
                return Err(ProtocolError::InvalidPacket {
                    method: "server_read_handshake_response".to_string(),
                    data: vec![],
                });
            }

            if self.decode_first(&mut data) {
                self.pkt.conn.get_mut().make_tls().await;
                continue;
            }
            break 'start;
        }

        let idx = data.iter().position(|&x| x == 0).unwrap();
        let user = str::from_utf8(&data.split_to(idx)).unwrap().to_string();
        debug!("user: {}, self.user: {}", user, self.user);

        if user != self.user {
            self.user = user;
            return Err(ProtocolError::AuthFailed(self.make_auth_err_info()));
        }

        let _ = data.get_u8();

        // length encoded data
        if self.capability & CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA > 0 {
            let (auth_data, is_null) = length_encoded_string(&mut data);
            if is_null {
                return Err(ProtocolError::AuthFailed(self.make_auth_lenc_err_info()));
            }
            self.auth_data = BytesMut::from(&auth_data[..])
        } else if self.capability & CLIENT_SECURE_CONNECTION > 0 {
            let n = data.get_u8() as usize;
            self.auth_data = data.split_to(n);
        } else {
            let idx = data.iter().position(|&x| x == 0).unwrap();
            self.auth_data = data.split_to(idx);
            let _ = data.get_u8();
        }

        if self.capability & CLIENT_CONNECT_WITH_DB > 0 && !data.is_empty() {
            let idx = data.iter().position(|&x| x == 0).unwrap();
            let db = str::from_utf8(&data.split_to(idx)).unwrap().to_string();
            if self.db != db {
                self.db = db;
                return Err(ProtocolError::AuthFailed(self.make_schema_err_info()));
            }
        }

        if self.capability & CLIENT_PLUGIN_AUTH > 0 {
            let idx = data.iter().position(|&x| x == 0).unwrap();
            self.auth_plugin_name = str::from_utf8(&data.split_to(idx)).unwrap().to_string();
        } else {
            self.auth_plugin_name = AUTH_NATIVE_PASSWORD.to_string()
        }

        if self.auth_plugin_name != AUTH_NATIVE_PASSWORD {
            debug!("auth_plugin_name: {}", self.auth_plugin_name);
            return self.handle_auth_switch().await;
        }

        self.compare_auth_data()
    }

    fn decode_first(&mut self, data: &mut BytesMut) -> bool {
        self.capability = data.get_u32_le();

        // skip max packet size
        data.get_u32_le();

        //charset, skip, if you want to use another charset, use set names
        //c.collation = CollationId(data[pos])
        data.get_u8();

        //skip reserved 23[00]
        let _ = data.split_to(23);

        data.is_empty()
    }

    async fn handle_auth_switch(&mut self) -> Result<(), ProtocolError> {
        self.pkt.write_auth_switch_request(&self.salt, AUTH_NATIVE_PASSWORD.to_string()).await?;
        self.auth_plugin_name = AUTH_NATIVE_PASSWORD.to_string();
        self.handle_auth_switch_response().await
    }

    fn compare_auth_data(&mut self) -> Result<(), ProtocolError> {
        match self.auth_plugin_name.as_str() {
            AUTH_NATIVE_PASSWORD => {
                if !compare(&self.auth_data, &calc_password(&self.salt, self.password.as_bytes())) {
                    return Err(ProtocolError::AuthFailed(self.make_auth_err_info()));
                }
                Ok(())
            }
            _ => Err(ProtocolError::AuthFailed(self.make_auth_plugin_err_info())),
        }
    }

    async fn handle_auth_switch_response(&mut self) -> Result<(), ProtocolError> {
        let mut data = BytesMut::with_capacity(128);
        let _ = self.pkt.read_packet_buf(&mut data).await?;

        match self.auth_plugin_name.as_str() {
            AUTH_NATIVE_PASSWORD => {
                let passwd_salt = calc_password(&self.salt, self.password.as_bytes());
                debug!("passwd_salt: {:?}, data: {:?}", passwd_salt, data);
                if !compare(&passwd_salt, &data) {
                    return Err(ProtocolError::AuthFailed(self.make_auth_err_info()));
                }
                Ok(())
            }
            _ => Err(ProtocolError::AuthFailed(self.make_auth_plugin_err_info())),
        }
    }

    fn make_auth_err_info(&mut self) -> Vec<u8> {
        self.pkt.make_err_packet(MySQLError::new(
            1045,
            "28000".as_bytes().to_vec(),
            format!("Access denied for user {:?}, (using password: Yes)", self.user),
        ))
    }

    fn make_schema_err_info(&mut self) -> Vec<u8> {
        self.pkt.make_err_packet(MySQLError::new(
            1049,
            "42000".as_bytes().to_vec(),
            format!("Unknown database {:?}", self.db),
        ))
    }

    fn make_auth_plugin_err_info(&mut self) -> Vec<u8> {
        self.pkt.make_err_packet(MySQLError::new(
            1045,
            "28000".as_bytes().to_vec(),
            format!("unsupport authentication plugin name {:?}", self.auth_plugin_name,),
        ))
    }

    fn make_auth_lenc_err_info(&mut self) -> Vec<u8> {
        self.pkt.make_err_packet(MySQLError::new(
            1045,
            "28000".as_bytes().to_vec(),
            "auth data is null".to_string(),
        ))
    }
}
