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
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Buf, BufMut, BytesMut};
use futures::Stream;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use pin_project::pin_project;
use protocol_codegen::mysql_codec_convert;
use tokio_util::codec::{Decoder, Encoder, Framed};

use super::{
    auth::ClientAuth,
    resultset::{write_command_binary, ResultsetCodec},
    stmt::Stmt,
    stream::LocalStream,
};
use crate::{
    err::ProtocolError,
    mysql_const::{
        CLIENT_PROTOCOL_41, CLIENT_SESSION_TRACK, CLIENT_TRANSACTIONS, SERVER_SESSION_STATE_CHANGED,
    },
    util::{get_length, length_encode_int, length_encoded_string},
};

pub type SendCommand<'a> = (u8, &'a str);

#[derive(Debug, mysql_codec_convert)]
pub enum ClientCodec {
    ClientAuth(Framed<LocalStream, ClientAuth>),
    Resultset(Framed<LocalStream, ResultsetCodec>),
    Stmt(Framed<LocalStream, Stmt>),
    Common(Framed<LocalStream, CommonCodec>),
}

#[derive(Debug)]
#[pin_project]
pub struct ResultsetStream<'a> {
    framed: &'a mut Framed<LocalStream, ResultsetCodec>,
}

impl<'a> ResultsetStream<'a> {
    pub fn new(framed: Option<&'a mut Box<ClientCodec>>) -> ResultsetStream {
        let framed = match framed.unwrap().as_mut() {
            ClientCodec::Resultset(data) => data,
            _ => unreachable! {},
        };

        ResultsetStream { framed }
    }
}

impl<'a> Stream for ResultsetStream<'a> {
    type Item = Result<BytesMut, ProtocolError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.project();
        let codec = Pin::new(me.framed);
        let is_complete = codec.codec().next_state.is_complete();

        match codec.poll_next(cx) {
            Poll::Ready(Some(Ok(data))) => Poll::Ready(Some(Ok(data.0))),

            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),

            Poll::Ready(None) => Poll::Ready(None),

            Poll::Pending => {
                if is_complete {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct CommonCodec {
    seq: u8,
    pub auth_info: Option<ClientAuth>,
    is_complete: bool,
}

impl CommonCodec {
    pub fn with_auth_info(auth_info: Option<ClientAuth>) -> CommonCodec {
        CommonCodec { seq: 0, auth_info, is_complete: false }
    }
}

impl Decoder for CommonCodec {
    type Item = (BytesMut, bool);
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        self.seq = src[3];
        let length = get_length(src) as usize;

        if src.len() < 4 + length {
            return Ok(None);
        }
        let payload = src.split_to(4 + length);

        let is_empty = src.is_empty();
        if is_empty {
            self.is_complete = true
        }

        Ok(Some((payload, is_empty)))
    }
}

impl<'a> Encoder<(u8, &'a [u8])> for CommonCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: (u8, &'a [u8]), dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(1024);
        write_command_binary(item, dst);
        Ok(())
    }
}

#[pin_project]
pub struct CommonStream<'a> {
    #[pin]
    framed: &'a mut Framed<LocalStream, CommonCodec>,
}

impl<'a> CommonStream<'a> {
    pub fn new(framed: Option<&'a mut Box<ClientCodec>>) -> CommonStream {
        let framed = match framed.unwrap().as_mut() {
            ClientCodec::Common(data) => data,
            _ => unreachable! {},
        };

        CommonStream { framed }
    }
}

impl<'a> Stream for CommonStream<'a> {
    type Item = Result<BytesMut, ProtocolError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.project();
        let is_complete = me.framed.codec().is_complete;

        match me.framed.poll_next(cx) {
            Poll::Ready(Some(Ok(data))) => Poll::Ready(Some(Ok(data.0))),

            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),

            Poll::Ready(None) => Poll::Ready(None),

            Poll::Pending => {
                if is_complete {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

pub fn write_command<'a>(item: SendCommand<'a>, dst: &'a mut BytesMut) {
    let length = item.1.len() + 1;
    dst.put_u8(length as u8);
    dst.put_u8((length >> 8) as u8);
    dst.put_u8((length >> 16) as u8);
    dst.put_u8(0);
    dst.put_u8(item.0);
    dst.put(item.1.as_bytes());
}

#[derive(Debug, FromPrimitive)]
#[repr(u8)]
pub enum SessionStateType {
    SystemVariables,            // Session system variables
    Schema,                     // Current schema
    StateChange,                // Session state changes
    Gtids,                      // GTIDs
    TransactionCharacteristics, // Transaction characteristics
    TransactionState,           // Transaction state
}

#[derive(Debug)]
pub enum SessionState {
    SystemVariables(Vec<(Vec<u8>, Vec<u8>)>),
    Schema(Vec<u8>),
    StateChange(bool),
    Gtids(Vec<u8>),
    TransactionCharacteristics(Vec<u8>),
    TransactionState(Vec<u8>),
    Unknown(Vec<u8>),
}

impl SessionState {
    pub fn decode(data: &mut BytesMut) -> SessionState {
        let mut payload = data.split_off(1);
        let (num, _, pos) = length_encode_int(&payload);
        let _ = payload.split_to(pos as usize);
        let mut payload = payload.split_to(num as usize);

        match FromPrimitive::from_u8(data[0]) {
            Some(SessionStateType::SystemVariables) => {
                let mut pairs = Vec::new();
                while !payload.is_empty() {
                    let (name, _) = length_encoded_string(&mut payload);
                    let (value, _) = length_encoded_string(&mut payload);
                    pairs.push((name, value))
                }

                SessionState::SystemVariables(pairs)
            }
            Some(SessionStateType::Schema) => {
                let (schema, _) = length_encoded_string(&mut payload);
                SessionState::Schema(schema)
            }
            Some(SessionStateType::StateChange) => {
                let (is_tracked, _) = length_encoded_string(&mut payload);
                SessionState::StateChange(is_tracked == b"1")
            }
            Some(SessionStateType::Gtids) => {
                let (gtids, _) = length_encoded_string(&mut payload);
                SessionState::Gtids(gtids)
            }
            Some(SessionStateType::TransactionCharacteristics) => {
                let (char, _) = length_encoded_string(&mut payload);
                SessionState::TransactionCharacteristics(char)
            }
            Some(SessionStateType::TransactionState) => {
                let (state, _) = length_encoded_string(&mut payload);
                SessionState::TransactionState(state)
            }
            None => {
                data.unsplit(payload);
                SessionState::Unknown(data.to_vec())
            }
        }
    }
}

#[derive(Debug)]
pub struct ResultOkInfo {
    affected_rows: u64,
    last_insert_id: u64,
    status: Option<u16>,
    warnings: Option<u16>,
    info: Option<Vec<u8>>,
    state_info: Option<SessionState>,
}

impl ResultOkInfo {
    fn new() -> ResultOkInfo {
        ResultOkInfo {
            affected_rows: 0,
            last_insert_id: 0,
            status: None,
            warnings: None,
            info: None,
            state_info: None,
        }
    }

    pub fn decode(auth_info: &ClientAuth, data: &mut BytesMut) -> ResultOkInfo {
        let mut ok_info = ResultOkInfo::new();

        let (affect_rows, _, pos) = length_encode_int(data);
        ok_info.affected_rows = affect_rows;
        let _ = data.split_to(pos as usize);

        let (last_inert_id, _, pos) = length_encode_int(data);
        ok_info.last_insert_id = last_inert_id;
        let _ = data.split_to(pos as usize);

        if auth_info.capability & CLIENT_PROTOCOL_41 > 0 {
            ok_info.status = Some(data.get_u16_le());
            ok_info.warnings = Some(data.get_u16_le());
        } else if auth_info.capability & CLIENT_TRANSACTIONS > 0 {
            ok_info.status = Some(data.get_u16_le());
        }

        if data.is_empty() {
            return ok_info;
        }

        if auth_info.capability & CLIENT_SESSION_TRACK > 0 {
            let (info, _) = length_encoded_string(data);
            ok_info.info = Some(info);

            if let Some(status) = ok_info.status {
                if status & SERVER_SESSION_STATE_CHANGED > 0 {
                    let (_, _, pos) = length_encode_int(data);
                    let _ = data.split_to(pos as usize);
                    ok_info.state_info = Some(SessionState::decode(data))
                }
            }
        } else {
            let payload = data.split();
            ok_info.info = Some(payload.to_vec())
        }

        ok_info
    }
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use tokio_stream::StreamExt;

    use super::ResultOkInfo;
    use crate::client::{codec::SessionState, conn::ClientConn};

    #[tokio::test]
    async fn test_handshake() {
        let mut driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "127.0.0.1:13306".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(!driver.handshake().await.is_err(), true)
    }

    #[tokio::test]
    async fn test_query() {
        let mut driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "127.0.0.1:13307".to_string(),
        )
        .await
        .unwrap();
        let _ = driver.handshake().await;

        let query = "select user from mysql.user".as_bytes();
        let mut stream = driver.send_query(query).await.unwrap();

        while let Some(data) = stream.next().await {
            assert_eq!(data.is_ok(), true);
        }

        let query = "show databases;".as_bytes();

        let s = driver.send_common_command(3, query).await;

        let mut stream = s.unwrap();

        loop {
            match stream.next().await {
                Some(data) => {
                    println!("recv {:?}", &data);
                    assert_eq!(data.is_ok(), true);
                }
                None => break,
            }
        }
    }

    #[tokio::test]
    async fn test_decode_ok_packet_schema() {
        let mut packet = BytesMut::from(
            &[
                0x13, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x40, 0x00, 0x00, 0x00, 0x0a, 0x01,
                0x05, 0x04, 0x74, 0x65, 0x73, 0x74, 0x02, 0x01, 0x31,
            ][..],
        );

        let _ = packet.split_to(4 + 1);

        let mut driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "192.168.5.144:13306".to_string(),
        )
        .await
        .unwrap();
        let _ = driver.handshake().await;

        let auth_info = driver.auth_info.take().unwrap();
        let info = ResultOkInfo::decode(&auth_info, &mut packet);

        if let Some(SessionState::Schema(schema)) = info.state_info {
            assert_eq!(b"test"[..], schema)
        }
    }

    #[tokio::test]
    async fn test_decode_ok_packet_vars() {
        let mut packet = BytesMut::from(
            &[
                0x1d, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x14, 0x00,
                0x0f, 0x0a, 0x61, 0x75, 0x74, 0x6f, 0x63, 0x6f, 0x6d, 0x6d, 0x69, 0x74, 0x03, 0x4f,
                0x46, 0x46, 0x02, 0x01, 0x31,
            ][..],
        );

        let _ = packet.split_to(4 + 1);

        let mut driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "192.168.5.144:13306".to_string(),
        )
        .await
        .unwrap();
        let _ = driver.handshake().await;

        let auth_info = driver.auth_info.take().unwrap();
        let info = ResultOkInfo::decode(&auth_info, &mut packet);

        if let Some(SessionState::SystemVariables(vars)) = info.state_info {
            assert_eq!(b"autocommit"[..], vars[0].0);
            assert_eq!(b"OFF"[..], vars[0].1);
        }
    }

    #[tokio::test]
    async fn test_decode_ok_packet_common() {
        let mut packet =
            BytesMut::from(&[0x07, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00][..]);

        let _ = packet.split_to(4 + 1);
        let mut driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "192.168.5.144:13306".to_string(),
        )
        .await
        .unwrap();
        let _ = driver.handshake().await;

        let auth_info = driver.auth_info.take().unwrap();
        let info = ResultOkInfo::decode(&auth_info, &mut packet);
        assert_eq!(info.state_info.is_none(), true);
    }
}
