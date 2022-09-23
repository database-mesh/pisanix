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
    marker::PhantomData,
    sync::{atomic::Ordering, Arc},
    vec,
};

use bytes::BytesMut;
use conn_pool::{Pool, PoolConn};
use futures::{stream::FuturesOrdered, SinkExt, StreamExt};
use mysql_protocol::{
    client::{
        codec::{MergeResultsetState, MergeStream, ResultsetStream},
        conn::{ClientConn, SessionAttr},
        stmt::Stmt,
    },
    column::{Column, ColumnInfo},
    err::ProtocolError,
    mysql_const::*,
    row::{RowData, RowDataBinary, RowDataText, RowDataTyp},
    server::codec::{make_eof_packet, CommonPacket, PacketSend},
    util::{is_eof, length_encode_int},
};
use pisa_error::error::{Error, ErrorKind};
use rayon::prelude::*;
use strategy::sharding_rewrite::{DataSource, ShardingRewriteOutput};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    mysql::ReqContext,
    transaction_fsm::check_get_conn,
};

use byteorder::{ByteOrder, LittleEndian};

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
    pub async fn shard_query_executor(
        req: &mut ReqContext<T, C>,
        rewrite_outputs: Vec<ShardingRewriteOutput>,
        attrs: Vec<SessionAttr>,
        is_get_conn: bool,
    ) -> Result<(), Error> {
        let mut curr_server_stmt_id: Option<u32> = None;
        let mut curr_cached_stmt_id = vec![];

        let conns = if is_get_conn {
            Self::get_shard_conns(&rewrite_outputs, req.pool.clone(), attrs).await?
        } else {
            let mut cached_conn = req.fsm.get_shard_conn();
            if cached_conn.is_empty() {
                let server_stmt_id = req.stmt_id.load(Ordering::Relaxed);
                let cached_stmt_conn = req.stmt_cache.get_all(server_stmt_id);
                curr_cached_stmt_id = cached_stmt_conn.iter().map(|x| x.0).collect();
                cached_conn = cached_stmt_conn.into_iter().map(|x| x.1).collect();
                curr_server_stmt_id = Some(server_stmt_id);
            }

            cached_conn
        };

        let mut conns = Self::shard_send_query(conns, &rewrite_outputs).await?;
        let shards_length = conns.len();
        let mut shard_streams = Vec::with_capacity(shards_length);

        for conn in conns.iter_mut() {
            let s = ResultsetStream::new(conn.framed.as_mut());
            shard_streams.push(s.fuse());
        }

        let mut merge_stream = MergeStream::new(shard_streams, shards_length);

        let sharding_column = rewrite_outputs[0].sharding_column.clone();
        Self::handle_shard_resultset(req, &mut merge_stream, sharding_column, false).await?;

        if let Some(id) = curr_server_stmt_id {
            let stmt_conns = curr_cached_stmt_id.into_iter().zip(conns.into_iter()).collect();
            req.stmt_cache.put_all(id, stmt_conns)
        } else {
            // Put shard conn to fsm
            req.fsm.put_shard_conn(conns);
        }
        Ok(())
    }

    async fn handle_shard_resultset<'a>(
        req: &mut ReqContext<T, C>,
        merge_stream: &mut MergeStream<ResultsetStream<'a>>,
        sharding_column: Option<String>,
        is_binary: bool,
    ) -> Result<(), Error> {
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

        let col_info = Self::get_columns(req, merge_stream, cols, &mut buf).await?;

        // read eof
        let _ = merge_stream.next().await;

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(make_eof_packet()[4..].into(), buf.len()), &mut buf);

        merge_stream.set_state(MergeResultsetState::Row);
        // get rows
        Self::get_rows(req, merge_stream, &mut buf, sharding_column, col_info, is_binary).await?;

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
        sharding_column: Option<String>,
        col_info: Arc<[ColumnInfo]>,
        is_binary: bool,
    ) -> Result<(), Error> {
        let row_data = match is_binary {
            false => {
                let row_data_text = RowDataText::new(col_info.clone(), &[][..]);
                RowDataTyp::Text(row_data_text)
            }
            true => {
                let row_data_binary = RowDataBinary::new(col_info.clone(), &[][..]);
                RowDataTyp::Binary(row_data_binary)
            }
        };

        while let Some(chunk) = stream.next().await {
            let mut chunk = chunk
                .into_par_iter().map(|x| x.unwrap()).collect::<Result<Vec<_>, _>>().map_err(ErrorKind::from)?;

            //println!("chunk {:?}", &chunk[..]);
            if is_binary {
                if let Some(name) = &sharding_column {
                    chunk.par_sort_by_cached_key(|x| {
                        let mut row_data = row_data.clone();
                        row_data.with_buf(&x[4..]);
                        row_data.decode_with_name::<u64>(&name).unwrap()
                    })
                }
            } else {
                if let Some(name) = &sharding_column {
                    if let Some(_) = col_info.iter().find(|col_info| col_info.column_name.eq(name)) {
                        chunk.par_sort_by_cached_key(|x| {
                            let mut row_data = row_data.clone();
                            row_data.with_buf(&x[4..]);
                            let value = row_data.decode_with_name::<String>(&name).unwrap().unwrap();
                            value.parse::<u64>().unwrap()
                        })
                    }
                }
            }

            for row in chunk.iter() {
                let _ = req
                    .framed
                    .codec_mut()
                    .encode(PacketSend::EncodeOffset(row[4..].into(), buf.len()), buf);
            }
        }

        Ok(())
    }
    async fn get_columns<'a>(
        req: &mut ReqContext<T, C>,
        stream: &mut MergeStream<ResultsetStream<'a>>,
        column_length: u64,
        buf: &mut BytesMut,
    ) -> Result<Arc<[ColumnInfo]>, Error> {
        let mut col_buf = Vec::with_capacity(100);
        let mut idx: Option<usize> = None;

        for _ in 0..column_length {
            let data = stream.next().await;
            let data = if let Some(idx) = idx {
                if let Some(mut data) = data {
                    let data = data.remove(idx);
                    if let Some(data) = data {
                        data.map_err(ErrorKind::Protocol)?
                    } else {
                        unreachable!()
                    }
                } else {
                    unreachable!()
                }
            } else {
                // find index from chunk
                let data = Self::get_shard_one_data(data)?;
                if let Some(data) = data {
                    idx = Some(data.0);
                    data.1
                } else {
                    unreachable!()
                }
            };

            col_buf.extend_from_slice(&data[..]);
            let _ = req
                .framed
                .codec_mut()
                .encode(PacketSend::EncodeOffset(data[4..].into(), buf.len()), buf);
        }

        let col_info = col_buf.as_slice().decode_columns();
        let arc_col_info: Arc<[ColumnInfo]> = col_info.into_boxed_slice().into();

        Ok(arc_col_info)
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

    async fn shard_send_query(
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

    pub async fn shard_prepare_executor(
        req: &mut ReqContext<T, C>,
        rewrite_outputs: Vec<ShardingRewriteOutput>,
        attrs: Vec<SessionAttr>,
        _is_get_conn: bool,
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

        Ok((stmts, sended_conns))
    }

    pub async fn shard_execute_executor(
        req: &mut ReqContext<T, C>,
        payload: &[u8],
    ) -> Result<(), Error> {
        let stmt_id = LittleEndian::read_u32(payload);
        let mut conns = Self::shard_send_execute(req, stmt_id, payload).await?;
        let shard_length = conns.len();
        let mut shard_streams = Vec::with_capacity(shard_length);
        for conn in conns.iter_mut() {
            shard_streams.push(ResultsetStream::new(conn.1.framed.as_mut()).fuse());
        }

        let sharding_column = req.stmt_cache.get_sharding_column(stmt_id);
        let mut merge_stream = MergeStream::new(shard_streams, shard_length);
        Self::handle_shard_resultset(req, &mut merge_stream, sharding_column, true).await?;

        req.stmt_cache.put_all(stmt_id, conns);

        Ok(())
    }

    async fn shard_send_execute(
        req: &mut ReqContext<T, C>,
        stmt_id: u32,
        payload: &[u8],
    ) -> Result<Vec<(u32, PoolConn<ClientConn>)>, Error> {
        let mut send_futs = FuturesOrdered::new();
        let stmt_cache = req.stmt_cache.get_all(stmt_id);
        let mut sended_conns = Vec::with_capacity(stmt_cache.len());

        for (id, mut conn) in stmt_cache.into_iter() {
            let mut payload = payload.to_vec();
            let payload_ptr = payload.as_mut_ptr();
            let id_bytes = id.to_le_bytes().as_ptr();
            unsafe {
                std::ptr::copy_nonoverlapping(id_bytes, payload_ptr, 4);
            }
            let payload = payload.clone();
            let f = tokio::spawn(async move {
                let res = conn.send_execute_without_stream(&payload).await;
                (conn, id, res)
            });
            send_futs.push(f);
        }

        while let Some(res) = send_futs.next().await {
            let (conn, id, send_res) = res.map_err(|e| ErrorKind::Runtime(e.into()))?;
            let _ = send_res.map_err(ErrorKind::from)?;
            sended_conns.push((id, conn));
        }

        Ok(sended_conns)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        assert_eq!(1, 1);
    }
}
