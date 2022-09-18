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

use std::marker::PhantomData;

use bytes::BytesMut;
use conn_pool::{Pool, PoolConn};
use futures::{executor, stream::FuturesOrdered, SinkExt, StreamExt};
use mysql_protocol::{
    client::{
        codec::{MergeStream, ResultsetStream},
        conn::{ClientConn, SessionAttr}, stmt::Stmt,
    },
    err::ProtocolError,
    mysql_const::*,
    server::codec::{make_eof_packet, CommonPacket, PacketSend},
    util::{length_encode_int, is_eof},
};
use pisa_error::error::{Error, ErrorKind};
use strategy::sharding_rewrite::{DataSource, ShardingRewriteOutput};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Encoder};

use crate::{mysql::ReqContext, transaction_fsm::check_get_conn};

#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    #[error("execute sql: {0:?} error")]
    DataSourceNotFound(String),
}

pub struct Executor<T, C> {
    _phant: PhantomData<(T, C)>,
}

impl<T, C> Executor<T, C>
where
    T: AsyncRead + AsyncWrite + Unpin + Send,
    C: Decoder<Item = BytesMut>
        + Encoder<PacketSend<Box<[u8]>>, Error = ProtocolError>
        + Send
        + CommonPacket,
{
    pub async fn sharding_query_executor(
        req: &mut ReqContext<T, C>,
        rewrite_outputs: Vec<ShardingRewriteOutput>,
        pool: Pool<ClientConn>,
        attrs: Vec<SessionAttr>,
    ) -> Result<(), Error> {
        let conns = Self::get_shard_conns(&rewrite_outputs, pool, attrs).await?;
        let mut conns = Self::shard_send_query(conns, &rewrite_outputs).await?;
        let shards_length = conns.len();
        let mut shard_streams = Vec::with_capacity(shards_length);
        for conn in conns.iter_mut() {
            shard_streams.push(ResultsetStream::new(conn.framed.as_mut()).fuse());
        }

        let mut merge_stream = MergeStream::new(shard_streams, shards_length);
        let header = merge_stream.next().await;
        let header = if let Some(header) = Self::get_shard_one_data(header)? {
            header.1
        } else {
            return Ok(());
        };

        let ok_or_err = header[4];

        if ok_or_err == OK_HEADER || ok_or_err == ERR_HEADER {
            req.framed
                .send(PacketSend::Encode(header[4..].into()))
                .await
                .map_err(ErrorKind::Protocol)?;
            return Ok(());
        }

        let (cols, ..) = length_encode_int(&header[4..]);

        let mut buf = BytesMut::with_capacity(1 << 16);

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(header[4..].into(), 0), &mut buf);

        Self::get_columns(req, &mut merge_stream, cols, &mut buf).await?;

        // read eof
        let _ = merge_stream.next().await;

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(make_eof_packet()[4..].into(), buf.len()), &mut buf);

        // get rows
        Self::get_rows(req, &mut merge_stream, &mut buf).await?;

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(make_eof_packet()[4..].into(), buf.len()), &mut buf);

        req.framed.send(PacketSend::Origin(buf[..].into())).await.map_err(ErrorKind::Protocol)?;
        Ok(())
    }

    async fn get_rows<'a>(
        req: &mut ReqContext<T, C>,
        stream: &mut MergeStream<ResultsetStream<'a>>,
        buf: &mut BytesMut,
    ) -> Result<(), Error> {
        while let Some(mut chunk) = stream.next().await {
            let _ = Self::check_single_chunk(&mut chunk)?;
            for row in chunk.into_iter() {
                if let Some(row) = row {
                    // We have ensured `c` is Ok(_) by above step
                    let row = row.unwrap();
                    if is_eof(&row) {
                        continue;
                    }

                    let _ = req
                        .framed
                        .codec_mut()
                        .encode(PacketSend::EncodeOffset(row[4..].into(), buf.len()), buf);
                }
            }
        }

        Ok(())
    }
    async fn get_columns<'a>(
        req: &mut ReqContext<T, C>,
        stream: &mut MergeStream<ResultsetStream<'a>>,
        column_length: u64,
        buf: &mut BytesMut,
    ) -> Result<(), Error> {
        let mut idx: Option<usize> = None;
        for _ in 0..column_length {
            let data = stream.next().await;
            let data = if let Some(idx) = idx {
                if let Some(mut data) = data {
                    let data = data.remove(idx);
                    if let Some(data) = data {
                        data.map_err(ErrorKind::Protocol)?
                    } else {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }
            } else {
                // find index from chunk
                let data = Self::get_shard_one_data(data)?;
                if let Some(data) = data {
                    idx = Some(data.0);
                    data.1
                } else {
                    return Ok(());
                }
            };

            let _ = req
                .framed
                .codec_mut()
                .encode(PacketSend::EncodeOffset(data[4..].into(), buf.len()), buf);
        }

        Ok(())
    }

    fn get_shard_one_data(
        data: Option<Vec<Option<Result<BytesMut, ProtocolError>>>>,
    ) -> Result<Option<(usize, BytesMut)>, Error> {
        match data {
            Some(data) => {
                // get one first is_some
                let data = data.into_iter().enumerate().find_map(|(idx, data)| {
                    if let Some(data) = data {
                        Some((idx, data))
                    } else {
                        None
                    }
                });
                match data {
                    Some((idx, data)) => Ok(Some((idx, data.map_err(ErrorKind::Protocol)?))),
                    None => return Ok(None),
                }
            }
            None => return Ok(None),
        }
    }

    fn check_single_chunk(
        chunk: &mut [Option<Result<BytesMut, ProtocolError>>],
    ) -> Result<(), Error> {
        for c in chunk.iter_mut() {
            if let Some(c) = c {
                match c {
                    Ok(_) => {}
                    Err(e) => {
                        let e = std::mem::replace(e, ProtocolError::Default);
                        return Err(Error::new(ErrorKind::Protocol(e)));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn shard_send_query(
        conns: Vec<PoolConn<ClientConn>>,
        rewrite_outputs: &[ShardingRewriteOutput],
    ) -> Result<Vec<PoolConn<ClientConn>>, Error> {
        let mut send_futs = FuturesOrdered::new();
        let mut sended_conns = Vec::with_capacity(conns.len());

        for (mut conn, ro) in conns.into_iter().zip(rewrite_outputs.iter()) {
            let sql = ro.target_sql.clone();
            let f = tokio::spawn(async move {
                let res = conn.send_query_without_stream(sql.as_bytes()).await;
                (conn, res)
            });
            send_futs.push(f);
        }

        while let Some(conn) = send_futs.next().await {
            let (conn, send_res) = conn.map_err(|e| ErrorKind::Runtime(e.into()))?;
            send_res.map_err(ErrorKind::Protocol)?;
            sended_conns.push(conn);
        }

        Ok(sended_conns)
    }

    pub async fn get_shard_conns(
        rewrite_outputs: &[ShardingRewriteOutput],
        pool: Pool<ClientConn>,
        attrs: Vec<SessionAttr>,
    ) -> Result<Vec<PoolConn<ClientConn>>, Error> {
        let endpoints = rewrite_outputs
            .iter()
            .map(|x| match &x.data_source {
                DataSource::Endpoint(ep) => Ok(ep.clone()),
                _ => Err(ExecuteError::DataSourceNotFound(x.target_sql.clone())),
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ErrorKind::Runtime(e.into()))?;

        let mut conn_futs = FuturesOrdered::new();
        for e in endpoints.iter() {
            let ep = e.clone();
            let mut pool = pool.clone();
            let attrs = attrs.clone();
            let f = tokio::spawn(async move {
                let factory = ClientConn::with_opts(ep.user, ep.password, ep.addr.clone());
                pool.set_factory(factory);
                check_get_conn(pool, &ep.addr, &attrs).await
            });
            conn_futs.push(f);
        }

        let mut conns = Vec::with_capacity(conn_futs.len());
        while let Some(conn) = conn_futs.next().await {
            let conn = conn.map_err(|e| ErrorKind::Runtime(e.into()))??;
            conns.push(conn);
        }

        Ok(conns)
    }

    pub async fn sharding_prepare_executor(
        req: &mut ReqContext<T, C>,
        rewrite_outputs: Vec<ShardingRewriteOutput>,
        attrs: Vec<SessionAttr>,
    ) -> Result<(Vec<Stmt>, Vec<PoolConn<ClientConn>>), Error> {
        let conns = Self::get_shard_conns(&rewrite_outputs, req.pool.clone(), attrs).await?;
        let mut send_futs = FuturesOrdered::new();
        let mut sended_conns = Vec::with_capacity(conns.len());

        for (mut conn, ro) in conns.into_iter().zip(rewrite_outputs.iter()) {
            let sql = ro.target_sql.clone();
            let f = tokio::spawn(async move {
                let res = conn.send_prepare(sql.as_bytes()).await;
                (conn, res)
            });
            send_futs.push(f);
        }

        let mut stmts = Vec::with_capacity(send_futs.len());
        while let Some(conn) = send_futs.next().await {
            let (conn, send_res) = conn.map_err(|e| ErrorKind::Runtime(e.into()))?;
            let stmt = send_res.map_err(ErrorKind::Protocol)?;
            sended_conns.push(conn);
            stmts.push(stmt);
        }

        Ok(( stmts, sended_conns ))
    }
}
