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
    str,
    sync::atomic::{AtomicU32, Ordering},
};

use bytes::{Buf, BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use tokio_util::codec::{Decoder, Encoder, Framed};
use tracing::debug;

use super::{err::MySQLError, stream::LocalStream};
use crate::{
    charset::{COLLATION_NAME_ID_MYSQL5, DEFAULT_CHARSET_NAME},
    err::ProtocolError,
    mysql_const::*,
    server::codec::{make_err_packet, ok_packet},
    session::{Session, SessionMut},
    util::*,
};

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

lazy_static! {
    static ref CONNECTION_ID: AtomicU32 = AtomicU32::new(0);
}

#[derive(Debug, PartialEq)]
pub enum ServerHandshakeStatus {
    WriteInitial,
    ReadResponseFirst,
    ReadResponse,
    SwitchToTLS,
    WriteAutoSwitch,
    ReadAutoSwitchResponse,
    CompareAuthData,
    Complete,
}

pub struct ServerHandshakeCodec {
    seq: u8,
    server_version: String,
    connection_id: u32,
    capability: u32,
    charset: String,
    salt: Vec<u8>,
    status: u16,
    user: String,
    db: String,
    password: String,
    auth_data: BytesMut,
    auth_plugin_name: String,
    autocommit: Option<String>,
    next_handshake_status: ServerHandshakeStatus,
}

impl ServerHandshakeCodec {
    pub fn new(user: String, password: String, db: String, server_version: String) -> Self {
        CONNECTION_ID.fetch_add(1, Ordering::Relaxed);

        Self {
            seq: 0,
            server_version,
            connection_id: CONNECTION_ID.load(Ordering::Relaxed),
            capability: 0,
            charset: DEFAULT_CHARSET_NAME.to_string(),
            salt: random_buf(20),
            status: SERVER_STATUS_AUTOCOMMIT,
            user,
            db,
            password,
            auth_data: BytesMut::with_capacity(20),
            auth_plugin_name: "".to_string(),
            autocommit: None,
            next_handshake_status: ServerHandshakeStatus::ReadResponseFirst,
        }
    }

    fn encode_initial_handshake(&self) -> BytesMut {
        let mut data = BytesMut::with_capacity(128);

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
        data.put_u8(COLLATION_NAME_ID_MYSQL5[&*self.charset]);

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

        data
        //self.pkt.make_packet_header(data.len() - 4, &mut data);
        //self.pkt.write_buf(&data).await
    }

    fn decode_handshake_response(&mut self, data: &mut BytesMut) -> Result<(), ProtocolError> {
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
            let (auth_data, is_null) = length_encoded_string(data);
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

            // Eat db \0
            let _ = data.get_u8();
        }

        if self.capability & CLIENT_PLUGIN_AUTH > 0 {
            let idx = data.iter().position(|&x| x == 0).unwrap();
            self.auth_plugin_name = str::from_utf8(&data.split_to(idx)).unwrap().to_string();
            let _ = data.get_u8();
        } else {
            self.auth_plugin_name = AUTH_NATIVE_PASSWORD.to_string()
        }

        // Currently, we don't use CLIENT_CONNECT_ATTRS
        data.clear();

        if &self.auth_plugin_name != AUTH_NATIVE_PASSWORD {
            debug!("auth_plugin_name: {}", self.auth_plugin_name);
            self.auth_plugin_name = AUTH_NATIVE_PASSWORD.to_string();
            self.next_handshake_status = ServerHandshakeStatus::WriteAutoSwitch;
            return Ok(());
        }

        self.next_handshake_status = ServerHandshakeStatus::CompareAuthData;
        Ok(())
        //self.compare_auth_data()
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

    fn generate_auth_switch_request(&self) -> BytesMut {
        let mut dst = BytesMut::with_capacity(128);
        dst.extend_from_slice(&[0; 4]);
        dst.put_u8(EOF_HEADER);
        dst.extend_from_slice(self.auth_plugin_name.as_bytes());
        dst.put_u8(0);
        dst.extend_from_slice(&self.salt);
        dst.put_u8(0);

        dst
    }

    fn compare_auth_data(&mut self) -> Result<(), ProtocolError> {
        self.next_handshake_status = ServerHandshakeStatus::Complete;

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

    fn handle_auth_switch_response(&mut self, data: &mut BytesMut) -> Result<(), ProtocolError> {
        self.next_handshake_status = ServerHandshakeStatus::Complete;

        match self.auth_plugin_name.as_str() {
            AUTH_NATIVE_PASSWORD => {
                let passwd_salt = calc_password(&self.salt, self.password.as_bytes());
                debug!("passwd_salt: {:?}, data: {:?}", passwd_salt, data);
                if !compare(&passwd_salt, &data[..]) {
                    return Err(ProtocolError::AuthFailed(self.make_auth_err_info()));
                }
                Ok(())
            }
            _ => Err(ProtocolError::AuthFailed(self.make_auth_plugin_err_info())),
        }
    }

    fn make_auth_err_info(&mut self) -> Vec<u8> {
        make_err_packet(MySQLError::new(
            1045,
            "28000".as_bytes().to_vec(),
            format!("Access denied for user {:?}, (using password: Yes)", self.user),
        ))
    }

    fn make_schema_err_info(&mut self) -> Vec<u8> {
        make_err_packet(MySQLError::new(
            1049,
            "42000".as_bytes().to_vec(),
            format!("Unknown database {:?}", self.db),
        ))
    }

    fn make_auth_plugin_err_info(&mut self) -> Vec<u8> {
        make_err_packet(MySQLError::new(
            1045,
            "28000".as_bytes().to_vec(),
            format!("unsupport authentication plugin name {:?}", self.auth_plugin_name,),
        ))
    }

    fn make_auth_lenc_err_info(&mut self) -> Vec<u8> {
        make_err_packet(MySQLError::new(
            1045,
            "28000".as_bytes().to_vec(),
            "auth data is null".to_string(),
        ))
    }
}

impl Decoder for ServerHandshakeCodec {
    type Item = ();
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() || src.len() < 3 {
            return Ok(None);
        }

        let length = get_length(&*src) as usize;

        if 4 + length > src.len() {
            return Ok(None);
        }

        let _ = src.split_to(4);
        self.seq += 1;

        match self.next_handshake_status {
            ServerHandshakeStatus::ReadResponseFirst => {
                let is_empty = self.decode_first(src);

                if is_empty {
                    self.next_handshake_status = ServerHandshakeStatus::SwitchToTLS;
                } else {
                    self.decode_handshake_response(src)?;
                    if self.next_handshake_status == ServerHandshakeStatus::CompareAuthData {
                        self.compare_auth_data()?;
                    }
                }

                Ok(Some(()))
            }

            ServerHandshakeStatus::ReadResponse => {
                self.decode_first(src);
                self.decode_handshake_response(src)?;

                if self.next_handshake_status == ServerHandshakeStatus::CompareAuthData {
                    self.compare_auth_data()?;
                }
                Ok(Some(()))
            }

            ServerHandshakeStatus::ReadAutoSwitchResponse => {
                self.handle_auth_switch_response(src)?;
                Ok(Some(()))
            }

            _ => Ok(Some(())),
        }
    }
}

impl Encoder<BytesMut> for ServerHandshakeCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if self.next_handshake_status == ServerHandshakeStatus::WriteAutoSwitch {
            self.next_handshake_status = ServerHandshakeStatus::ReadAutoSwitchResponse;
        }

        dst.extend_from_slice(&item[..]);

        let length = item.len() - 4;
        // we have ensured length is 3bytes, so we can use unsafe block
        unsafe {
            let bytes = *(&(length as u64).to_le() as *const u64 as *const [u8; 8]);
            let data_ptr = dst.as_mut_ptr();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data_ptr, 3);
            *data_ptr.add(3) = self.seq;
        }

        self.seq += 1;

        Ok(())
    }
}

impl Session for ServerHandshakeCodec {
    fn get_db(&self) -> Option<String> {
        if self.db.is_empty() {
            None
        } else {
            Some(self.db.clone())
        }
    }

    fn get_charset(&self) -> Option<String> {
        Some(self.charset.clone())
    }

    fn get_autocommit(&self) -> Option<String> {
        self.autocommit.clone()
    }
}

impl SessionMut for ServerHandshakeCodec {
    fn set_db(&mut self, db: String) {
        self.db = db
    }

    fn set_charset(&mut self, charset: String) {
        self.charset = charset
    }

    fn set_autocommit(&mut self, autocommit: String) {
        self.autocommit = Some(autocommit)
    }
}

pub async fn handshake(
    mut framed: Framed<LocalStream, ServerHandshakeCodec>,
) -> Result<(Framed<LocalStream, ServerHandshakeCodec>, bool), ProtocolError> {
    // Send initial handshake
    let initial_handshake = framed.codec().encode_initial_handshake();
    framed.send(initial_handshake).await?;

    loop {
        if let Err(ProtocolError::AuthFailed(data)) = framed.next().await.unwrap() {
            framed.send(BytesMut::from(&data[..])).await?;
            return Ok((framed, false));
        }

        let next_state = &framed.codec().next_handshake_status;
        match next_state {
            ServerHandshakeStatus::SwitchToTLS => {
                let mut parts = framed.into_parts();
                parts.io.make_tls().await?;

                framed = Framed::from_parts(parts);
                framed.codec_mut().next_handshake_status = ServerHandshakeStatus::ReadResponse;
            }

            ServerHandshakeStatus::WriteAutoSwitch => {
                framed.send(framed.codec().generate_auth_switch_request()).await?;
            }

            ServerHandshakeStatus::Complete => {
                break;
            }
            _ => {}
        }
    }

    framed.send(BytesMut::from(&ok_packet()[..])).await?;

    Ok((framed, true))
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use futures::{SinkExt, StreamExt};
    use tokio_util::codec::Framed;

    use crate::{
        err::ProtocolError,
        server::{
            auth::{ServerHandshakeCodec, ServerHandshakeStatus},
        },
    };

    #[tokio::test]
    async fn test_handshake() {
        //let packet_codec = PacketCodec::new(8192);
        let user = "root".to_string();
        let password = "123456".to_string();
        let db = "sbtest_pisa".to_string();
        let server_version = "5.7.36".to_string();
        let hs = ServerHandshakeCodec::new(user, password, db, server_version);

        let (client, server) = tokio::io::duplex(512);
        let mut framed = Framed::new(client, hs);

        let client_reponse_data = [
            0xaf, 0x00, 0x00, 0x01, 0x8d, 0xa2, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x72, 0x6f, 0x6f, 0x74, 0x00, 0x14,
            0x38, 0x80, 0xb3, 0xc9, 0xc7, 0x29, 0x71, 0x1d, 0xeb, 0xf2, 0xe9, 0x43, 0x36, 0x24,
            0x4c, 0x71, 0xef, 0x32, 0x1a, 0x5d, 0x73, 0x62, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70,
            0x69, 0x73, 0x61, 0x00, 0x6d, 0x79, 0x73, 0x71, 0x6c, 0x5f, 0x6e, 0x61, 0x74, 0x69,
            0x76, 0x65, 0x5f, 0x70, 0x61, 0x73, 0x73, 0x77, 0x6f, 0x72, 0x64, 0x00, 0x52, 0x03,
            0x5f, 0x6f, 0x73, 0x05, 0x4c, 0x69, 0x6e, 0x75, 0x78, 0x0c, 0x5f, 0x63, 0x6c, 0x69,
            0x65, 0x6e, 0x74, 0x5f, 0x6e, 0x61, 0x6d, 0x65, 0x08, 0x6c, 0x69, 0x62, 0x6d, 0x79,
            0x73, 0x71, 0x6c, 0x04, 0x5f, 0x70, 0x69, 0x64, 0x04, 0x35, 0x32, 0x38, 0x31, 0x0f,
            0x5f, 0x63, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x5f, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f,
            0x6e, 0x06, 0x35, 0x2e, 0x36, 0x2e, 0x35, 0x31, 0x09, 0x5f, 0x70, 0x6c, 0x61, 0x74,
            0x66, 0x6f, 0x72, 0x6d, 0x06, 0x78, 0x38, 0x36, 0x5f, 0x36, 0x34,
        ];

        let _ = framed.send(BytesMut::from(&client_reponse_data[..])).await;

        let mut parts = framed.into_parts();
        parts.io = server;

        framed = Framed::from_parts(parts);
        let res = framed.next().await.unwrap();
        assert!(framed.codec().next_handshake_status == ServerHandshakeStatus::Complete);

        if let Err(ProtocolError::AuthFailed(_data)) = res {
            assert!(true);
        }
    }

    #[tokio::test]
    async fn test_handshake_auto_switch() {
        let user = "root".to_string();
        let password = "123456".to_string();
        let db = "sbtest_pisa".to_string();
        let server_version = "5.7.36".to_string();
        let hs = ServerHandshakeCodec::new(user, password, db, server_version);

        let (mut client, mut server) = tokio::io::duplex(512);
        let mut framed = Framed::new(client, hs);

        let client_reponse_data = [
            0xae, 0x00, 0x00, 0x01, 0x8d, 0xa2, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x40, 0x08, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x72, 0x6f, 0x6f, 0x74, 0x00, 0x14,
            0x38, 0x80, 0xb3, 0xc9, 0xc7, 0x29, 0x71, 0x1d, 0xeb, 0xf2, 0xe9, 0x43, 0x36, 0x24,
            0x4c, 0x71, 0xef, 0x32, 0x1a, 0x5d, 0x73, 0x62, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70,
            0x69, 0x73, 0x61, 0x00, 116, 101, 115, 116, 95, 112, 97, 115, 115, 119, 111, 114, 100,
            95, 112, 108, 117, 103, 105, 110, 0x00, 0x52, 0x03, 0x5f, 0x6f, 0x73, 0x05, 0x4c, 0x69,
            0x6e, 0x75, 0x78, 0x0c, 0x5f, 0x63, 0x6c, 0x69, 0x65, 0x6e, 0x74, 0x5f, 0x6e, 0x61,
            0x6d, 0x65, 0x08, 0x6c, 0x69, 0x62, 0x6d, 0x79, 0x73, 0x71, 0x6c, 0x04, 0x5f, 0x70,
            0x69, 0x64, 0x04, 0x35, 0x32, 0x38, 0x31, 0x0f, 0x5f, 0x63, 0x6c, 0x69, 0x65, 0x6e,
            0x74, 0x5f, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x06, 0x35, 0x2e, 0x36, 0x2e,
            0x35, 0x31, 0x09, 0x5f, 0x70, 0x6c, 0x61, 0x74, 0x66, 0x6f, 0x72, 0x6d, 0x06, 0x78,
            0x38, 0x36, 0x5f, 0x36, 0x34,
        ];

        let _ = framed.send(BytesMut::from(&client_reponse_data[..])).await;

        let mut parts = framed.into_parts();
        client = parts.io;
        parts.io = server;
        framed = Framed::from_parts(parts);

        let _res = framed.next().await.unwrap();

        assert_eq!(framed.codec().next_handshake_status, ServerHandshakeStatus::WriteAutoSwitch);

        framed.codec_mut().next_handshake_status = ServerHandshakeStatus::ReadAutoSwitchResponse;

        let mut parts = framed.into_parts();
        server = parts.io;
        parts.io = client;
        framed = Framed::from_parts(parts);

        let auto_switch_response_data = [
            0x14, 0x00, 0x00, 0x03, 0x38, 0x80, 0xb3, 0xc9, 0xc7, 0x29, 0x71, 0x1d, 0xeb, 0xf2,
            0xe9, 0x43, 0x36, 0x24, 0x4c, 0x71, 0xef, 0x32, 0x1a, 0x5d,
        ];

        let _ = framed.send(BytesMut::from(&auto_switch_response_data[..])).await;

        let mut parts = framed.into_parts();
        parts.io = server;
        framed = Framed::from_parts(parts);

        let res = framed.next().await.unwrap();
        assert_eq!(framed.codec().next_handshake_status, ServerHandshakeStatus::Complete);

        if let Err(ProtocolError::AuthFailed(_data)) = res {
            assert!(true);
        }
    }
}
