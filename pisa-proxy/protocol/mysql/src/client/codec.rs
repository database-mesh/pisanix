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
use futures::{Stream, stream::Fuse};
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
    row::{RowData, RowDataTyp},
    util::{get_length, is_eof},
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

pub enum MergeResultsetState {
    Header,
    ColumnInfo,
    ColumnEof,
    Row
}

#[pin_project]
pub struct MergeStream<S> 
where 
    S: Stream + std::marker::Unpin,
{
    inner: Vec<Fuse<S>>,
    idx: usize,
    buf: Vec<Option<S::Item>>,
    base_length: usize,
    limit: usize,
    limit_idx: usize,
    state: MergeResultsetState,
}

impl<S> MergeStream<S> 
where 
    S: Stream + std::marker::Unpin,
{
   fn set_state(&mut self, state: MergeResultsetState)  {
        self.state = state;
   }
}

impl<S> MergeStream<S>
where 
    S: Stream + std::marker::Unpin,
{
   pub fn new(inner: Vec<Fuse<S>>, base_length: usize) -> Self {
        let limit = base_length * 100;
        
        MergeStream { 
            inner, 
            idx: 0,
            buf: Vec::with_capacity(base_length),
            base_length,
            limit_idx: 0, 
            limit,
            state: MergeResultsetState::Header,
        }
   } 
}

impl<S> Stream for MergeStream<S> 
where 
    S: Stream + std::marker::Unpin,
    <S as Stream>::Item: std::fmt::Debug
{
    type Item = Vec<Option<S::Item>>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.project();
        loop {
            let s = unsafe {
                Pin::new(me.inner.get_unchecked_mut(*me.idx))
            };

            match me.state {
                MergeResultsetState::Header | MergeResultsetState::ColumnInfo | MergeResultsetState::ColumnEof => {
                    match s.poll_next(cx) {
                        Poll::Ready(data) => {
                            *me.idx += 1;
                            me.buf.push(data);
                        },

                        Poll::Pending => return Poll::Pending,
                    };

                    if *me.idx == *me.base_length {
                        *me.idx = 0;
                        break
                    }
                },

                MergeResultsetState::Row => {
                    match s.poll_next(cx) {
                        Poll::Ready(data) => {
                            *me.idx += 1;
                            *me.limit_idx += 1;
                            me.buf.push(data);
                        },

                        Poll::Pending => return Poll::Pending,
                    };

                    if *me.idx == *me.base_length {
                        *me.idx = 0;
                    }

                    if *me.limit_idx == *me.limit {
                        *me.limit_idx = 0;
                        break;
                    }
                }
            }
        }

        if me.buf.is_empty() || me.buf.iter().all(|x| x.is_none()) {
            return Poll::Ready(None);
        } else {
            return  Poll::Ready(Some(std::mem::replace(me.buf, Vec::with_capacity(*me.limit))))
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



#[cfg(test)]
mod test {
    use tokio_stream::StreamExt;

    use crate::client::conn::ClientConn;

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



}
