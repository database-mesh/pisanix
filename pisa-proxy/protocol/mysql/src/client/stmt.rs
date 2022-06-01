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

use byteorder::{ByteOrder, LittleEndian};
use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use super::{auth::ClientAuth, resultset::write_command_binary};
use crate::{
    err::ProtocolError,
    mysql_const::{EOF_HEADER, ERR_HEADER},
    util::get_length,
};

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
enum DecodeStmtState {
    PrepareFirst,
    PrepareCols,
    PrepareParams,
    PrepareComplete,
}

impl Default for DecodeStmtState {
    fn default() -> Self {
        DecodeStmtState::PrepareFirst
    }
}

/// Parse `COM_STMT_PREPARE`
#[derive(Debug, Default, Clone)]
pub struct Stmt {
    pub ori_stmt_id: Vec<u8>,
    pub ori_cols_count: Vec<u8>,
    pub ori_params_count: Vec<u8>,

    pub stmt_id: u32,
    pub cols_count: u16,
    pub params_count: u16,

    pub params_data: Vec<Vec<u8>>,
    pub cols_data: Vec<Vec<u8>>,

    next_state: DecodeStmtState,
    cols_idx: u16,
    params_idx: u16,
    seq: u8,
    pub auth_info: Option<ClientAuth>,
}

impl Stmt {
    pub fn new() -> Stmt {
        Stmt {
            ori_stmt_id: vec![0; 4],
            ori_cols_count: vec![0; 2],
            ori_params_count: vec![0; 2],

            stmt_id: 0,
            cols_count: 0,
            params_count: 0,

            params_data: vec![vec![0; 0]; 0],
            cols_data: vec![vec![0; 0]; 0],

            next_state: DecodeStmtState::PrepareFirst,
            cols_idx: 0,
            params_idx: 0,
            seq: 0,
            auth_info: None,
        }
    }

    pub fn renew(&mut self) {
        let auth_info = self.auth_info.take();
        *self = Stmt::new();
        self.auth_info = auth_info;
    }

    pub fn with_auth_info(auth_info: Option<ClientAuth>) -> Stmt {
        let mut codec = Stmt::new();
        codec.auth_info = auth_info;

        codec
    }

    fn decode_prepare_return(&mut self, length: usize, src: &mut BytesMut) -> Option<BytesMut> {
        if length + 4 > src.len() {
            return None;
        }

        let mut data = src.split_to(length + 4);

        if data[4] == ERR_HEADER {
            return Some(data);
        }

        let _ = data.split_to(5);

        let ori_stmt_id = data.split_to(4);
        self.stmt_id = LittleEndian::read_u32(&ori_stmt_id);
        self.ori_stmt_id = ori_stmt_id.to_vec();

        let ori_cols_count = data.split_to(2);
        self.cols_count = LittleEndian::read_u16(&ori_cols_count);
        self.ori_cols_count = ori_cols_count.to_vec();

        let ori_params_count = data.split_to(2);
        self.params_count = LittleEndian::read_u16(&ori_params_count);
        self.ori_params_count = ori_params_count.to_vec();

        //reserved_1 (1) -- [00] filler
        let _ = data.split_to(1);

        //warning_count (2) -- number of warnings
        let _ = data.split_to(2);
        if !src.is_empty() {
            self.next_state = DecodeStmtState::PrepareParams;
            for _ in 0..self.params_count {
                self.decode_prepare_params(get_length(&*src) as usize, src);
                if src.is_empty() {
                    break;
                }
            }

            if src.len() < 4 {
                return None;
            }

            if !src.is_empty() && src[4] == EOF_HEADER {
                let length = get_length(&*src) as usize;
                let _ = src.split_to(length + 4);
            }
        }

        if !src.is_empty() {
            self.next_state = DecodeStmtState::PrepareCols;
            for _ in 0..self.cols_count {
                self.decode_prepare_cols(get_length(&*src) as usize, src);
                if src.is_empty() {
                    break;
                }
            }

            if !src.is_empty() && src[4] == EOF_HEADER {
                let length = get_length(&*src) as usize;
                let _ = src.split_to(length + 4);
            }
        } else if self.params_count > 0 {
            self.next_state = DecodeStmtState::PrepareParams
        }

        None
    }

    fn decode_prepare_params(&mut self, length: usize, src: &mut BytesMut) -> bool {
        if length + 4 > src.len() {
            return false;
        }

        let data = src.split_to(length + 4);
        if data[4] == EOF_HEADER || self.params_idx == self.params_count {
            if self.cols_count > 0 {
                self.next_state = DecodeStmtState::PrepareCols
            }
        } else {
            self.params_idx += 1;
            self.params_data.push(data.to_vec());
        }

        self.params_idx == self.params_count
    }

    fn decode_prepare_cols(&mut self, length: usize, src: &mut BytesMut) -> bool {
        if length + 4 > src.len() {
            return false;
        }
        let data = src.split_to(length + 4);
        if data[4] == EOF_HEADER {
            self.next_state = DecodeStmtState::PrepareComplete
        } else {
            self.cols_idx += 1;
            self.cols_data.push(data.to_vec());
        }

        self.cols_idx == self.cols_count
    }
}

/// Implements `Decoder` trait
impl Decoder for Stmt {
    type Item = Option<BytesMut>;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() || src.len() <= 3 {
            return Ok(None);
        }

        let length = get_length(&*src) as usize;

        self.seq = src[3];
        match self.next_state {
            // Return Ok(Some(data)) only when prepare return error, otherwise return Ok(Some(None)).
            DecodeStmtState::PrepareFirst => Ok(Some(self.decode_prepare_return(length, src))),

            DecodeStmtState::PrepareParams => {
                let res = self.decode_prepare_params(length, src);
                if res {
                    Ok(Some(None))
                } else {
                    Ok(Some(Some(BytesMut::new())))
                }
            }

            DecodeStmtState::PrepareCols => {
                let res = self.decode_prepare_cols(length, src);
                if res {
                    Ok(Some(None))
                } else {
                    Ok(Some(Some(BytesMut::new())))
                }
            }

            DecodeStmtState::PrepareComplete => Ok(Some(None)),
        }
    }
}

impl<'a> Encoder<(u8, &'a [u8])> for Stmt {
    type Error = ProtocolError;

    fn encode(&mut self, item: (u8, &[u8]), dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(1024);
        write_command_binary(item, dst);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use futures::StreamExt;
    use tokio::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
        time,
    };
    use tracing::trace;

    use crate::client::stmt::Stmt;

    #[tokio::test]
    async fn test_decode_prepare() {
        let addr = "127.0.0.1:9999";
        let listener = TcpListener::bind(addr).await.unwrap();
        let accept = tokio::spawn(async move {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    println!("new client: {:?}", addr);

                    let mut framed = tokio_util::codec::Framed::new(socket, Stmt::new());
                    loop {
                        match framed.next().await {
                            Some(Ok(c)) => {
                                if c.is_none() {
                                    break;
                                }
                            }
                            Some(Err(_e)) => break,

                            None => {}
                        }
                    }

                    trace!("codec: {:?}", framed.codec());
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
                let data = [0x0c,0x00,0x00,0x01,0x00,0x02,0x00,0x00,0x00,0x02,0x00,0x01,0x00,0x00,0x00,0x00];
                sock.write(&data).await.unwrap();
                let data = [0x17,0x00,0x00,0x02,0x03,0x64,0x65,0x66,0x00,0x00,0x00,0x01,0x3f,0x00,0x0c,0x3f,
                            0x00,0x00,0x00,0x00,0x00,0xfd,0x80,0x00,0x00,0x00,0x00];
                sock.write(&data).await.unwrap();
                let data = [0x2b,0x00,0x00,0x03,0x03,0x64,0x65,0x66,0x05,0x6d,0x79,0x73,0x71,0x6c,0x04,0x75,
                            0x73,0x65,0x72,0x04,0x75,0x73,0x65,0x72,0x04,0x75,0x73,0x65,0x72,0x04,0x55,0x73,
                            0x65,0x72,0x0c,0x2d,0x00,0x80,0x00,0x00,0x00,0xfe,0x83,0x40,0x00,0x00,0x00];
                sock.write(&data).await.unwrap();
                let data = [0x2b,0x00,0x00,0x04,0x03,0x64,0x65,0x66,0x05,0x6d,0x79,0x73,0x71,0x6c,0x04,0x75,
                            0x73,0x65,0x72,0x04,0x75,0x73,0x65,0x72,0x04,0x68,0x6f,0x73,0x74,0x04,0x48,0x6f,
                            0x73,0x74,0x0c,0x2d,0x00,0xf0,0x00,0x00,0x00,0xfe,0x83,0x40,0x00,0x00,0x00];
                //time::sleep(Duration::from_secs(3)).await;
                sock.write(&data).await.unwrap();

                time::sleep(Duration::from_secs(5)).await;
            }
        };
    }
}
