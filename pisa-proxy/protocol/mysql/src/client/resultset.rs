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

use bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use super::{auth::ClientAuth, codec::*};
use crate::{
    err::ProtocolError,
    mysql_const::ERR_HEADER,
    util::{get_length, is_eof, is_ok, length_encode_int},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DecodeResultsetState {
    ColumnCount,
    ColumnInfo,
    Row,
    RowBinary,
    Complete,
}

impl DecodeResultsetState {
    pub fn is_complete(&self) -> bool {
        matches!(self, DecodeResultsetState::Complete)
    }
}

impl Default for DecodeResultsetState {
    fn default() -> Self {
        DecodeResultsetState::ColumnCount
    }
}

/// Parse `COM_QUERY` Response.
#[derive(Debug, Default)]
pub struct ResultsetCodec {
    pub next_state: DecodeResultsetState,
    pub col: u64,
    pub is_binary: bool,
    pub seq: u8,
    pub auth_info: Option<ClientAuth>,
}

impl ResultsetCodec {
    pub fn new() -> ResultsetCodec {
        ResultsetCodec {
            next_state: DecodeResultsetState::ColumnCount,
            col: 0,
            is_binary: false,
            seq: 0,
            auth_info: None,
        }
    }

    pub fn renew(&mut self) {
        let auth_info = self.auth_info.take();
        *self = ResultsetCodec::default();
        self.auth_info = auth_info;
    }

    pub fn with_auth_info(auth_info: Option<ClientAuth>) -> ResultsetCodec {
        let mut codec = ResultsetCodec::new();
        codec.auth_info = auth_info;

        codec
    }

    pub fn with_binary(&mut self, is_binary: bool) {
        self.is_binary = is_binary
    }

    fn decode_try_ok(&mut self, length: usize, data: &mut BytesMut) -> Option<BytesMut> {
        //let length = get_length(&*data) as usize;
        if is_ok(&*data) {
            Some(data.split_to(4 + length))
        } else {
            None
        }
    }

    fn decode_column_count(&mut self, length: usize, data: &mut BytesMut) -> (BytesMut, bool) {
        if data[4] == ERR_HEADER {
            //return Err(ProtocolError::PacketError(data.split().to_vec()))
            self.next_state = DecodeResultsetState::Complete;
            return (data.split(), true);
        }

        let is_ok = self.decode_try_ok(length, data);

        if let Some(data) = is_ok {
            self.next_state = DecodeResultsetState::Complete;
            return (data, true);
        }

        self.next_state = DecodeResultsetState::ColumnInfo;

        let payload = data.split_to(4 + length);

        let (col, is_null, _) = length_encode_int(&payload[4..]);
        self.col = col;

        (payload, is_null)
    }

    fn decode_column_info(&mut self, length: usize, data: &mut BytesMut) -> (BytesMut, bool) {
        let payload = data.split_to(4 + length);

        if is_eof(&payload) || is_ok(&payload) {
            if data.is_empty() {
                self.next_state = DecodeResultsetState::Row;
                return (payload, false);
            } else {
                self.decode_column_eof();
                return (payload, false);
            }
        } else {
            self.next_state = DecodeResultsetState::ColumnInfo;
        }

        (payload, false)
    }

    fn decode_column_eof(&mut self) {
        if self.is_binary {
            self.next_state = DecodeResultsetState::RowBinary
        } else {
            self.next_state = DecodeResultsetState::Row
        }
    }

    fn decode_row(&mut self, length: usize, data: &mut BytesMut) -> (BytesMut, bool) {
        let payload = data.split_to(4 + length);

        if is_eof(&payload) {
            self.next_state = DecodeResultsetState::Complete;
            return (payload, true);
        } else {
            self.next_state = DecodeResultsetState::Row;
        }

        (payload, false)
    }

    fn decode_row_binary(&mut self, length: usize, data: &mut BytesMut) -> (BytesMut, bool) {
        self.decode_row(length, data)
    }
}

/// Implements `Decoder` trait
impl Decoder for ResultsetCodec {
    type Item = (BytesMut, bool);
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() || src.len() < 3 {
            return Ok(None);
        }

        let length = get_length(&*src) as usize;
        if 4 + length > src.len() {
            return Ok(None);
        }

        self.seq = src[3];
        match self.next_state {
            DecodeResultsetState::ColumnCount => Ok(Some(self.decode_column_count(length, src))),

            DecodeResultsetState::ColumnInfo => Ok(Some(self.decode_column_info(length, src))),

            DecodeResultsetState::Row => {
                let res = self.decode_row(length, src);
                Ok(Some(res))
            }

            DecodeResultsetState::RowBinary => Ok(Some(self.decode_row_binary(length, src))),

            DecodeResultsetState::Complete => Ok(None),
        }
    }
}

pub enum ResultSendCommand<'a> {
    Plain((u8, &'a str)),
    Binary((u8, &'a [u8])),
}

impl<'a> Encoder<ResultSendCommand<'a>> for ResultsetCodec {
    type Error = ProtocolError;

    fn encode(
        &mut self,
        item: ResultSendCommand<'a>,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        self.next_state = DecodeResultsetState::ColumnCount;
        dst.reserve(1024);

        match item {
            ResultSendCommand::Plain(item) => write_command(item, dst),
            ResultSendCommand::Binary(item) => write_command_binary(item, dst),
        };
        Ok(())
    }
}

pub fn write_command_binary(item: (u8, &[u8]), dst: &mut BytesMut) {
    let length = item.1.len() + 1;
    dst.put_u8(length as u8);
    dst.put_u8((length >> 8) as u8);
    dst.put_u8((length >> 16) as u8);
    dst.put_u8(0);
    dst.put_u8(item.0);
    dst.extend_from_slice(item.1);
}

#[cfg(test)]
mod test {
    //use std::{time::{Duration, self}, thread};
    use futures::StreamExt;
    use tokio::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
        time,
        time::Duration,
    };
    use tokio_util::codec::Framed;

    use super::*;

    #[tokio::test]
    async fn test_decode() {
        let addr = "127.0.0.1:9999";
        let listener = TcpListener::bind(addr).await.unwrap();
        let accept = tokio::spawn(async move {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    println!("new client: {:?}", addr);

                    let mut framed = Framed::new(socket, ResultsetCodec::new());
                    let mut num = 0;
                    loop {
                        match framed.next().await {
                            Some(Ok(_data)) => {
                                num += 1;
                                if num == 6 {
                                    break;
                                }
                            }
                            Some(Err(_e)) => break,

                            None => break,
                        }
                    }

                    assert_eq!(num, 6)
                }
                _ => unreachable!(),
            }
        });

        let connect = TcpStream::connect(addr);

        let _ = tokio::select! {
            _ = accept => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            },
            v = connect => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let mut sock = v.unwrap();
                let data = [1,0,0,1,0xfe];
                sock.write(&data).await.unwrap();
                let data = [5,0,0,1, 1,1,1,1,1];
                sock.write(&data).await.unwrap();
                let data = [5,0,0,1, 1,1,1,1,2];
                sock.write(&data).await.unwrap();
                let data = [5,0,0,1, 1,1,1,1,3];
                sock.write(&data).await.unwrap();
                let data = [1,0,0,1,0xfe];
                sock.write(&data).await.unwrap();
                let data = [1,0,0,1, 2];
                sock.write(&data).await.unwrap();

                time::sleep(Duration::from_secs(3)).await;

                //let data = [1,0,0,1, 5];
                //sock.write(&data).await.unwrap();
                let data = [1,0,0,6,0xfe];
                sock.write(&data).await.unwrap();

                time::sleep(Duration::from_secs(3)).await;

                //let data = [1,0,0,1,0xfe];
                //sock.write(&data).await.unwrap();


            }
        };
    }
}
