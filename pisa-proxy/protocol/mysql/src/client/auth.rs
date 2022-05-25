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

use std::{convert::From, str};

use byteorder::{ByteOrder, LittleEndian};
use bytes::{BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use rand::rngs::OsRng;
use rsa::{pkcs8::DecodePublicKey, PaddingScheme, PublicKey, RsaPublicKey};
use sha1::Sha1;
use tokio_util::codec::{Decoder, Encoder, Framed};

use super::{codec::ClientCodec, stream::LocalStream};
use crate::{charset::DEFAULT_COLLATION_ID, err::ProtocolError, mysql_const::*, util::*};

/// Handshake state
#[derive(Debug, Clone)]
pub enum HandshakeState {
    InitialHandshake,
    WriteHandshake,
    WriteSslRequest,
    WriteClearAuth,
    HandshakeAutoSwitch,
    HandshakeResult,
    RequestPublicKey,
    GetPublicKey,
    WriteAuthPassword,
}

impl Default for HandshakeState {
    fn default() -> Self {
        HandshakeState::InitialHandshake
    }
}

#[derive(Debug, Default, Clone)]
pub struct ClientAuth {
    pub next_state: HandshakeState,
    pub connection_id: u32,
    pub salt: Vec<u8>,
    pub capability: u32,
    pub client_capability: u32,
    pub charset: u8,
    pub status: u16,
    pub auth_plugin_name: String,
    pub tls_config: Option<()>,
    pub user: String,
    pub password: String,
    pub db: String,
    pub seq: u8,
}

impl ClientAuth {
    pub fn new() -> ClientAuth {
        ClientAuth {
            next_state: HandshakeState::InitialHandshake,
            connection_id: 0,
            salt: Vec::with_capacity(21),
            client_capability: 0,
            capability: 0,
            status: 0,
            auth_plugin_name: "".to_string(),
            tls_config: None,
            charset: 0,
            user: "".to_string(),
            password: "".to_string(),
            db: "".to_string(),
            seq: 0,
        }
    }

    // Read initial handshake packet from server
    fn read_initial_handshake(&mut self, data: &mut BytesMut) -> Result<Self, ProtocolError> {
        if data[0] == ERR_HEADER {
            //return Err(Error::new(ErrorKind::Other, "read initial handshake error"));
            return Err(ProtocolError::HandshakeError(data.to_vec()));
        }

        if data[0] < MIN_PROTOCOL_VERSION {
            return Err(ProtocolError::ProtocolVersion(data[0]));
        }

        // skip protocol version
        let _ = data.split_to(1);

        // skip server version, end with 0x00
        let pos = data.iter().position(|&x| x == 0x00).unwrap();
        let _ = data.split_to(pos + 1);

        // connection id length is 4
        self.connection_id = LittleEndian::read_u32(&data.split_to(4));

        self.salt.extend_from_slice(&data.split_to(8));

        // skip filter
        let _ = data.split_to(1);

        // capability lower 2 bytes
        self.capability = u32::from(LittleEndian::read_u16(&(data.split_to(2))));

        if self.capability & CLIENT_PROTOCOL_41 == 0 {
            return Err(ProtocolError::ServerProtocolVersion);
        }

        if self.capability & CLIENT_SSL == 0 && self.tls_config.is_some() {
            return Err(ProtocolError::Tls);
        }

        if data.is_empty() {
            return Ok(self.clone());
        }

        // skip server charset
        // self.charset = data[pos]
        self.charset = data.split_to(1)[0] as u8;

        self.status = LittleEndian::read_u16(&data.split_to(2));

        self.capability |= u32::from(LittleEndian::read_u16(&data.split_to(2))) << 16;

        // skip auth data len or [00]
        // skip reserved (all [00])
        let _ = data.split_to(10);

        let mut max_auth_data_len = 13 + 8;

        let auth_data_len = data.split_to(1)[0] as u8;
        if self.capability & CLIENT_PLUGIN_AUTH != 0 && auth_data_len > max_auth_data_len {
            max_auth_data_len = data[pos];
        }

        // trim 0x00
        self.salt.extend_from_slice(&data.split_to((max_auth_data_len - 8 - 1) as usize));

        // skip 0x00
        let _ = data.split_to(1);

        // auth plugin
        if self.capability & CLIENT_PLUGIN_AUTH != 0 {
            let other_data = data.split();
            self.auth_plugin_name =
                str::from_utf8(&other_data[0..&other_data.len() - 1]).unwrap().to_string();
        }

        Ok(self.clone())
    }

    // gen_auth_response: generate auth response data according to auth plugin.
    pub fn gen_auth_response(&mut self, auth_data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        match self.auth_plugin_name.as_str() {
            AUTH_NATIVE_PASSWORD => Ok(calc_password(&auth_data[..20], self.password.as_bytes())),

            AUTH_CACHING_SHA2_PASSWORD => {
                Ok(calc_caching_sha2password(auth_data, self.password.as_bytes()))
            }

            AUTH_SHA256_PASSWORD => {
                if self.password.is_empty() {
                    return Ok(vec![0x00]);
                }

                if self.tls_config.is_some() {
                    // write cleartext password
                    let mut password_vec = self.password.as_bytes().to_vec();
                    password_vec.push(0x00);
                    Ok(password_vec)
                } else {
                    Ok(vec![0x01])
                }
            }

            plugin => Err(ProtocolError::AuthPluginUnsupport(plugin.to_string())),
        }
    }

    // Write auth handshake.
    fn write_auth_handshake(&mut self, data: &mut BytesMut) -> Result<bool, ProtocolError> {
        let mut capability = CLIENT_PROTOCOL_41
            | CLIENT_SECURE_CONNECTION
            | CLIENT_LONG_PASSWORD
            | CLIENT_TRANSACTIONS
            | CLIENT_PLUGIN_AUTH
            | self.capability & CLIENT_LONG_FLAG;

        capability |= self.client_capability & CLIENT_FOUND_ROWS
            | self.client_capability & CLIENT_IGNORE_SPACE
            | self.client_capability & CLIENT_MULTI_STATEMENTS
            | self.client_capability & CLIENT_MULTI_RESULTS
            | self.client_capability & CLIENT_PS_MULTI_RESULTS
            | self.client_capability & CLIENT_CONNECT_ATTRS;

        if self.tls_config.is_some() {
            capability |= CLIENT_SSL
        }

        let auth_data = match self.gen_auth_response(&self.salt.clone()) {
            Err(err) => return Err(err),
            Ok(auth_data) => auth_data,
        };

        // length-encode-integer
        let mut auth_resp_length_integer = Vec::with_capacity(9);
        auth_resp_length_integer.put_lenc_int(auth_data.len() as u64);

        //append_length_encoded_integer1(&mut auth_resp_length_integer, auth_data.len() as u64);

        if auth_resp_length_integer.len() > 1 {
            capability |= CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA;
        }

        //packet length
        //capability 4
        //max-packet size 4
        //charset 1
        //reserved all[0] 23
        //username
        //auth
        //auth plugin name + null-terminated
        //default plugin name is caching_sha2_password, length 21

        if !self.db.is_empty() {
            capability |= CLIENT_CONNECT_WITH_DB;
        }

        // put packet header 4 bytes
        //data.put_slice(&[0; 4]);

        //capability [32 bit]
        data.put_u8(capability as u8);
        data.put_u8((capability >> 8) as u8);
        data.put_u8((capability >> 16) as u8);
        data.put_u8((capability >> 24) as u8);

        //MaxPacketSize [32 bit] (none)
        data.put_slice(&[0, 0, 0, 0x01]);
        //data[8] = 0x00;
        //data[9] = 0x00;
        //data[10] = 0x00;
        //data[11] = 0x00;

        //charset [1 byte]
        // data[12] = DEFAULT_COLLATION_ID as u8;
        data.put_u8(DEFAULT_COLLATION_ID);

        data.put_slice(&[0; 23]);

        if !self.user.is_empty() {
            data.put_slice(self.user.as_bytes());
        }

        data.put_u8(0x00);

        data.put_slice(&auth_resp_length_integer);
        data.put_slice(&auth_data);

        data.put_slice(self.auth_plugin_name.as_bytes());
        data.put_u8(0x00);

        Ok(self.tls_config.is_some())
    }

    // Parse handshake result
    fn decode_handshake_result(
        &mut self,
        data: &mut BytesMut,
    ) -> Result<Option<(String, BytesMut)>, ProtocolError> {
        let length = get_length(data) as usize;
        let _ = data.split_to(4);

        match data[0] {
            OK_HEADER => Ok(None),

            MORE_DATA_HEADER => Ok(Some(("".to_string(), data.split_to(length).split_off(1)))),

            EOF_HEADER => {
                if data.is_empty() {
                    return Err(ProtocolError::AuthPluginUnsupport(
                        "mysql_old_password".to_string(),
                    ));
                }

                let _ = data.split_to(1);
                let is_pos = data.iter().position(|&x| x == 0x00);
                match is_pos {
                    Some(pos) => {
                        let auth_plugin_name =
                            str::from_utf8(&data.split_to(pos)).unwrap().to_string();
                        let auth_data = data.split();
                        Ok(Some((auth_plugin_name, auth_data)))
                    }

                    None => Err(ProtocolError::InvalidPacket {
                        method: "decode_handshake_result".to_string(),
                        data: data.to_vec(),
                    }),
                }
            }

            ERR_HEADER => Err(ProtocolError::AuthFailed(data.to_vec())),

            _ => Err(ProtocolError::HandshakeError(data.to_vec())),
        }
    }

    // https://github.com/mysql/mysql-server/blob/8.0/sql-common/client_authentication.cc#L433
    fn handle_auth_data(
        &mut self,
        data: &BytesMut,
    ) -> Result<Option<(HandshakeState, BytesMut)>, ProtocolError> {
        match self.auth_plugin_name.as_str() {
            AUTH_CACHING_SHA2_PASSWORD => {
                if data.is_empty() {
                    return Ok(None);
                }

                match data[0] {
                    0x03 => Ok(None),

                    0x04 => {
                        if self.tls_config.is_some() {
                            let mut req = BytesMut::from(self.password.as_bytes());
                            req.put_u8(0x00);
                            Ok(Some((HandshakeState::WriteClearAuth, req)))
                        } else {
                            let mut req = BytesMut::with_capacity(1);
                            req.put_u8(0x02);
                            self.next_state = HandshakeState::RequestPublicKey;
                            Ok(Some((HandshakeState::RequestPublicKey, req)))
                        }
                    }

                    _ => Err(ProtocolError::InvalidPacket {
                        method: "handle_auth_data".to_string(),
                        data: data.to_vec(),
                    }),
                }
            }
            AUTH_SHA256_PASSWORD => {
                if data.is_empty() {
                    return Ok(None);
                }

                let pub_key =
                    RsaPublicKey::from_public_key_pem(str::from_utf8(data).unwrap()).unwrap();
                let enc_data =
                    encrypt_password(self.password.as_bytes().to_vec(), self.salt.clone(), pub_key);

                Ok(Some((HandshakeState::WriteAuthPassword, BytesMut::from(&enc_data[..]))))
            }

            x => Err(ProtocolError::AuthPluginUnsupport(x.to_string())),
        }
    }
}

/// To make the handshake decode status clearer.
#[derive(Debug)]
pub enum HandshakeDecoderReturn {
    InitialHandshake(BytesMut),
    WriteSslRequest(BytesMut),
    WriteClearAuth(BytesMut),
    AuthSwitch(Option<(String, BytesMut)>),
    AuthSuccess,
    RequestPublicKey(BytesMut),
    EncryptPassword(BytesMut),
    Other(BytesMut),
}

// Implements Decoder
impl Decoder for ClientAuth {
    type Item = HandshakeDecoderReturn;
    type Error = ProtocolError;

    fn decode(&mut self, data: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if data.is_empty() {
            return Ok(None);
        }

        self.seq = data[3];

        match self.next_state {
            HandshakeState::InitialHandshake => {
                let _ = data.split_to(4);
                let auth = self.read_initial_handshake(data);
                if let Err(e) = auth {
                    return Err(e);
                }

                let mut auth_handshake = BytesMut::with_capacity(255);

                let is_switch_ssl = self.write_auth_handshake(&mut auth_handshake)?;

                if is_switch_ssl {
                    self.next_state = HandshakeState::WriteSslRequest;
                    Ok(Some(HandshakeDecoderReturn::WriteSslRequest(auth_handshake)))
                } else {
                    self.next_state = HandshakeState::WriteHandshake;
                    Ok(Some(HandshakeDecoderReturn::InitialHandshake(auth_handshake)))
                }
            }

            HandshakeState::HandshakeAutoSwitch => {
                let res = self.decode_handshake_result(data)?;
                match res {
                    Some((auth_plugin_name, auth_data)) => {
                        let handle_res = self.handle_auth_data(&auth_data)?;

                        match handle_res {
                            Some((state, data)) => match state {
                                HandshakeState::RequestPublicKey => {
                                    Ok(Some(HandshakeDecoderReturn::RequestPublicKey(data)))
                                }

                                HandshakeState::WriteClearAuth => {
                                    self.next_state = HandshakeState::HandshakeResult;
                                    Ok(Some(HandshakeDecoderReturn::WriteClearAuth(data)))
                                }

                                HandshakeState::WriteAuthPassword => {
                                    self.next_state = HandshakeState::HandshakeResult;
                                    Ok(Some(HandshakeDecoderReturn::EncryptPassword(data)))
                                }

                                _ => unreachable!(),
                            },

                            None => {
                                self.next_state = HandshakeState::HandshakeResult;
                                Ok(Some(HandshakeDecoderReturn::AuthSwitch(Some((
                                    auth_plugin_name,
                                    auth_data,
                                )))))
                            }
                        }
                    }

                    None => {
                        self.next_state = HandshakeState::HandshakeResult;
                        Ok(Some(HandshakeDecoderReturn::AuthSwitch(None)))
                    }
                }
            }

            HandshakeState::GetPublicKey => {
                self.next_state = HandshakeState::WriteAuthPassword;
                let _ = data.split_to(5);
                let pub_key = RsaPublicKey::from_public_key_pem(str::from_utf8(data).unwrap())?;
                let _ = data.split();
                let enc_data =
                    encrypt_password(self.password.as_bytes().to_vec(), self.salt.clone(), pub_key);
                Ok(Some(HandshakeDecoderReturn::EncryptPassword(BytesMut::from(&enc_data[..]))))
            }

            HandshakeState::HandshakeResult => {
                let _ = data.split_to(4);
                if data[0] == OK_HEADER && data.len() <= 7 {
                    let _ = data.split();
                    return Ok(Some(HandshakeDecoderReturn::AuthSuccess));
                }

                Err(ProtocolError::AuthFailed(data.to_vec()))
            }

            _ => Ok(None),
        }
    }
}

fn encrypt_password(mut password: Vec<u8>, salt: Vec<u8>, pub_key: RsaPublicKey) -> Vec<u8> {
    password.push(0x00);
    let password_salt = password
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let j = i % salt.len();
            *v ^ salt[j]
        })
        .collect::<Vec<u8>>();

    let mut rng = OsRng;
    let padding = PaddingScheme::new_oaep::<Sha1>();
    pub_key.encrypt(&mut rng, padding, &password_salt).unwrap()
}

impl Encoder<BytesMut> for ClientAuth {
    type Error = ProtocolError;

    fn encode(&mut self, item: BytesMut, dst: &mut BytesMut) -> Result<(), ProtocolError> {
        let length = item.len();
        dst.put_u8(length as u8);
        dst.put_u8((length >> 8) as u8);
        dst.put_u8((length >> 16) as u8);

        self.seq += 1;

        match self.next_state {
            HandshakeState::WriteHandshake => {
                self.next_state = HandshakeState::HandshakeAutoSwitch;
                dst.put_u8(self.seq);
                dst.extend_from_slice(&item);
                Ok(())
            }

            HandshakeState::WriteSslRequest => {
                // truncate 4+4+1+23
                dst[0] = 32_u8;
                dst[1] = (32 >> 8) as u8;
                dst[2] = (32 >> 16) as u8;
                self.next_state = HandshakeState::WriteHandshake;
                dst.put_u8(self.seq);
                dst.extend_from_slice(&item[..32]);
                Ok(())
            }

            HandshakeState::RequestPublicKey => {
                self.next_state = HandshakeState::GetPublicKey;
                dst.put_u8(self.seq);
                dst.extend_from_slice(&item);

                Ok(())
            }

            HandshakeState::WriteAuthPassword => {
                self.next_state = HandshakeState::HandshakeResult;
                dst.put_u8(self.seq);
                dst.extend_from_slice(&item);
                Ok(())
            }

            _ => {
                dst.put_u8(self.seq);
                dst.extend_from_slice(&item);
                Ok(())
            }
        }
    }
}

pub async fn handshake(
    framed: ClientCodec,
) -> Result<(Framed<LocalStream, ClientAuth>, bool, Vec<u8>), ProtocolError> {
    let mut framed = match framed {
        ClientCodec::ClientAuth(data) => data,
        _ => unreachable!(),
    };

    loop {
        match framed.next().await {
            Some(Ok(HandshakeDecoderReturn::InitialHandshake(auth))) => framed.send(auth).await?,

            Some(Ok(HandshakeDecoderReturn::WriteSslRequest(req))) => {
                framed.send(req.clone()).await?;

                let mut parts = framed.into_parts();

                // Create tls connection
                parts.io.make_tls().await?;
                framed = Framed::new(parts.io, parts.codec);

                framed.send(req).await?
            }

            Some(Ok(HandshakeDecoderReturn::AuthSwitch(_data))) => {}

            Some(Ok(HandshakeDecoderReturn::RequestPublicKey(req))) => framed.send(req).await?,

            Some(Ok(HandshakeDecoderReturn::WriteClearAuth(item))) => framed.send(item).await?,

            Some(Ok(HandshakeDecoderReturn::EncryptPassword(passwd))) => {
                framed.send(passwd).await?
            }

            Some(Ok(HandshakeDecoderReturn::AuthSuccess)) => return Ok((framed, true, vec![])),

            Some(Err(e)) => return Err(e),

            _ => {}
        }
    }
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use tokio::net::TcpStream;
    use tokio_util::codec::Framed;

    use super::{handshake, ClientAuth};
    use crate::client::{
        codec::ClientCodec,
        stream::{LocalStream, StreamWrapper},
    };

    #[test]
    fn test_initial_handshake() {
        let initial_data = [
            0x0a, 0x38, 0x2e, 0x30, 0x2e, 0x32, 0x36, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x29, 0x35,
            0x3f, 0x0e, 0x58, 0x2f, 0x28, 0x50, 0x00, 0xff, 0xff, 0xff, 0x02, 0x00, 0xff, 0xcf,
            0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x79, 0x05,
            0x0f, 0x06, 0x45, 0x2d, 0x44, 0x2b, 0x14, 0x65, 0x59, 0x00, 0x63, 0x61, 0x63, 0x68,
            0x69, 0x6e, 0x67, 0x5f, 0x73, 0x68, 0x61, 0x32, 0x5f, 0x70, 0x61, 0x73, 0x73, 0x77,
            0x6f, 0x72, 0x64, 0x00,
        ];

        let mut c = ClientAuth::new();
        let mut bytes = BytesMut::from(&initial_data[..]);
        let _ = c.read_initial_handshake(&mut bytes);

        assert_eq!(c.capability, 3489660927);
        assert_eq!(c.salt[0], 0x29);
        assert_eq!(c.salt[c.salt.len() - 1], 0x59);
        assert_eq!(c.auth_plugin_name, "caching_sha2_password".to_string());
        assert_eq!(c.charset, 0xff);
    }

    // test auth success with mysql_native_password plugin
    #[tokio::test]
    async fn test_handshake_native_succ() {
        let mut auth_codec = ClientAuth::new();
        auth_codec.user = "root".to_string();
        auth_codec.password = "123456".to_string();
        assert_eq!(test_handshake_resp(auth_codec).await, true);
    }

    #[tokio::test]
    async fn test_handshake_native_ssl_succ() {
        let mut auth_codec = ClientAuth::new();
        auth_codec.user = "root".to_string();
        auth_codec.password = "123456".to_string();
        auth_codec.tls_config = Some(());
        assert_eq!(test_handshake_resp(auth_codec).await, true);
    }

    // test auth failed with mysql_native_password plugin
    #[tokio::test]
    async fn test_handshake_native_failed() {
        let mut auth_codec = ClientAuth::new();
        auth_codec.user = "root".to_string();
        auth_codec.password = "1234567".to_string();
        assert_eq!(test_handshake_resp(auth_codec).await, false);
    }

    #[tokio::test]
    async fn test_handshake_native_ssl_failed() {
        let mut auth_codec = ClientAuth::new();
        auth_codec.user = "root".to_string();
        auth_codec.password = "1234567".to_string();
        auth_codec.tls_config = Some(());
        assert_eq!(test_handshake_resp(auth_codec).await, false);
    }

    // test auth success with caching_sha2_password plugin
    #[tokio::test]
    async fn test_handshake_cache_succ() {
        let mut auth_codec = ClientAuth::new();
        auth_codec.user = "root".to_string();
        auth_codec.password = "123456".to_string();
        assert_eq!(test_handshake_resp(auth_codec).await, true);
    }

    // test auth failed with caching_sha2_password plugin
    #[tokio::test]
    async fn test_handshake_cache_failed() {
        let mut auth_codec = ClientAuth::new();
        auth_codec.user = "root".to_string();
        auth_codec.password = "1234567".to_string();
        assert_eq!(test_handshake_resp(auth_codec).await, false);
    }

    async fn test_handshake_resp(codec: ClientAuth) -> bool {
        let addr = "127.0.0.1:13306";
        let sock = TcpStream::connect(addr).await.unwrap();
        let stream_wrapper = StreamWrapper::from(sock);
        let local_stream = LocalStream::new(stream_wrapper);

        //let mut auth_codec = ClientAuth::new();

        let framed = Framed::new(local_stream, codec);

        !handshake(ClientCodec::ClientAuth(framed)).await.is_err()
    }
}
