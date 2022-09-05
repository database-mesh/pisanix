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
    convert::From,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Buf, BufMut, BytesMut};
use futures::Stream;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use pin_project::pin_project;
use protocol_codegen::mysql_codec_convert;
use tokio::io::Interest;
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
    row::{RowData, RowDataTyp},
    util::{get_length, is_eof, length_encoded_string, BufMutExt, BufExt},
};

pub type SendCommand<'a> = (u8, &'a str);

#[derive(Debug, mysql_codec_convert)]
pub enum ClientCodec {
    ClientAuth(Framed<LocalStream, ClientAuth>),
    Resultset(Framed<LocalStream, ResultsetCodec>),
    Stmt(Framed<LocalStream, Stmt>),
    Common(Framed<LocalStream, CommonCodec>),
}

// Access `AuthInfo` struct by dereferencing the `ClientCodec` struct.
impl Deref for ClientCodec {
    type Target = ClientAuth;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::ClientAuth(framed) => framed.codec(),
            Self::Resultset(framed) => framed.codec().auth_info.as_ref().unwrap(),
            Self::Stmt(framed) => framed.codec().auth_info.as_ref().unwrap(),
            Self::Common(framed) => framed.codec().auth_info.as_ref().unwrap(),
        }
    }
}

// Modify `AuthInfo` struct by dereferencing the `ClientCodec` struct.
impl DerefMut for ClientCodec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::ClientAuth(framed) => framed.codec_mut(),
            Self::Resultset(framed) => framed.codec_mut().auth_info.as_mut().unwrap(),
            Self::Stmt(framed) => framed.codec_mut().auth_info.as_mut().unwrap(),
            Self::Common(framed) => framed.codec_mut().auth_info.as_mut().unwrap(),
        }
    }
}

impl ClientCodec {
    pub async fn is_ready(&self) -> bool {
        let local_stream = match self {
            Self::ClientAuth(framed) => framed.get_ref(),
            Self::Resultset(framed) => framed.get_ref(),
            Self::Stmt(framed) => framed.get_ref(),
            Self::Common(framed) => framed.get_ref(),
        };

        let underly_io = local_stream.get_inner();

        let is_ready = underly_io.ready(Interest::READABLE | Interest::WRITABLE).await;
        match is_ready {
            Ok(ready) => {
                if ready.is_read_closed() || ready.is_write_closed() {
                    false
                } else {
                    true
                }
            }
            Err(_) => false,
        }
    }
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

    pub fn is_binary(&self) -> bool {
        self.framed.codec().is_binary
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

#[pin_project]
pub struct QueryResultStream<'a, T: AsRef<[u8]>> {
    #[pin]
    rs: ResultsetStream<'a>,
    row_data: RowDataTyp<T>,
}

impl<'a, T: AsRef<[u8]>> QueryResultStream<'a, T> {
    pub fn new(rs: ResultsetStream<'a>, row_data: RowDataTyp<T>) -> Self {
        QueryResultStream { rs, row_data }
    }
}

impl<'a, T: AsRef<[u8]> + Clone + From<bytes::BytesMut>> Stream for QueryResultStream<'a, T> {
    type Item = Result<RowDataTyp<T>, ProtocolError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.project();

        match me.rs.poll_next(cx) {
            Poll::Ready(Some(Ok(mut data))) => {
                if is_eof(&data) {
                    return Poll::Ready(None);
                }

                let mut row_data = me.row_data.clone();
                // Exclude header 4 bytes
                data.advance(4);
                row_data.with_buf(data.into());
                Poll::Ready(Some(Ok(row_data)))
            }

            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),

            Poll::Ready(None) => Poll::Ready(None),

            Poll::Pending => Poll::Pending,
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
        let (num, _, _) = payload.get_lenc_int();

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
    state_changes: Option<SessionState>,
}

impl ResultOkInfo {
    fn new() -> ResultOkInfo {
        ResultOkInfo {
            affected_rows: 0,
            last_insert_id: 0,
            status: None,
            warnings: None,
            info: None,
            state_changes: None,
        }
    }

    pub fn decode(auth_info: &ClientAuth, data: &mut BytesMut) -> ResultOkInfo {
        let mut ok_info = ResultOkInfo::new();

        let (affect_rows, _, _) = data.get_lenc_int();
        ok_info.affected_rows = affect_rows;

        let (last_inert_id, _, _) = data.get_lenc_int();
        ok_info.last_insert_id = last_inert_id;

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
                    let _ = data.get_lenc_int();
                    ok_info.state_changes = Some(SessionState::decode(data))
                }
            }
        } else {
            let payload = data.split();
            ok_info.info = Some(payload.to_vec())
        }

        ok_info
    }

    pub fn encode(&self, capability: u32) -> Box<[u8]> {
        let mut data = BytesMut::with_capacity(64);
        data.put_u8(0x00);

        data.put_lenc_int(self.affected_rows, true);
        data.put_lenc_int(self.last_insert_id, true);

        if capability & CLIENT_PROTOCOL_41 > 0 {
            if let Some(status) = &self.status {
                data.put_u16_le(*status);
            }

            if let Some(warnings) = &self.warnings {
                data.put_u16_le(*warnings);
            }

        } else if capability & CLIENT_TRANSACTIONS > 0 {
            if let Some(status) = &self.status {
                data.put_u16_le(*status);
            }
        }

        if let Some(info) = &self.info {
            data.put_lenc_int(info.len() as u64, false);
            data.put_slice(&info);
        }
        data[..].into()
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
        let driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "127.0.0.1:13306".to_string(),
        )
        .await;
        assert_eq!(driver.is_ok(), true)
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

        let driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "127.0.0.1:13306".to_string(),
        )
        .await
        .unwrap();

        let auth_info = driver.framed.as_ref().unwrap();
        let info = ResultOkInfo::decode(auth_info, &mut packet);

        if let Some(SessionState::Schema(schema)) = info.state_changes {
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

        let driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "127.0.0.1:13306".to_string(),
        )
        .await
        .unwrap();

        let auth_info = driver.framed.as_ref().unwrap();
        let info = ResultOkInfo::decode(auth_info, &mut packet);

        if let Some(SessionState::SystemVariables(vars)) = info.state_changes {
            assert_eq!(b"autocommit"[..], vars[0].0);
            assert_eq!(b"OFF"[..], vars[0].1);
        }
    }

    #[tokio::test]
    async fn test_decode_ok_packet_common() {
        let mut packet =
            BytesMut::from(&[0x07, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00][..]);

        let _ = packet.split_to(4 + 1);
        let driver = ClientConn::test_conn(
            "root".to_string(),
            "123456".to_string(),
            "127.0.0.1:13306".to_string(),
        )
        .await
        .unwrap();

        let auth_info = driver.framed.as_ref().unwrap();
        let info = ResultOkInfo::decode(auth_info, &mut packet);
        assert_eq!(info.state_changes.is_none(), true);
    } 

     #[tokio::test]
     async fn test_ok_packet_encode() {
         let driver = ClientConn::test_conn(
             "root".to_string(),
             "123456".to_string(),
             "127.0.0.1:13306".to_string(),
         )
         .await
         .unwrap();
         let auth_info = driver.framed.as_ref().unwrap();
         let ok_info = ResultOkInfo {
             affected_rows: 256,
             last_insert_id: 0,
             status: Some(0x0022),
             warnings: Some(0),
             info: Some(b"Records: 256  Duplicates: 0  Warnings: 0".to_vec()),
             state_changes: None,

         };

         let expected = [ 
             0x00, 0xfc, 0x00, 0x01, 0x00, 0x22, 0x00, 0x00, 0x00, 0x28, 0x52, 0x65,
             0x63, 0x6f, 0x72, 0x64, 0x73, 0x3a, 0x20, 0x32, 0x35, 0x36, 0x20, 0x20, 0x44, 0x75, 0x70, 0x6c,
             0x69, 0x63, 0x61, 0x74, 0x65, 0x73, 0x3a, 0x20, 0x30, 0x20, 0x20, 0x57, 0x61, 0x72, 0x6e, 0x69,
             0x6e, 0x67, 0x73, 0x3a, 0x20, 0x30,
         ];

         let res = ok_info.encode(auth_info.capability);

         assert_eq!(&expected[..], &res[..])
    }

}
