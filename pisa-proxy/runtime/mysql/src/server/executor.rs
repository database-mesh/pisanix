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
    vec, ops::Div,
};

use bytes::{BytesMut, BufMut, Buf};
use conn_pool::{Pool, PoolConn};
use futures::{stream::FuturesOrdered, SinkExt, StreamExt};
use mysql_protocol::{
    client::{
        codec::{MergeResultsetState, MergeStream, ResultsetStream},
        conn::{ClientConn, SessionAttr},
        stmt::Stmt,
    },
    column::{Column, ColumnInfo, decode_column},
    err::ProtocolError,
    mysql_const::*,
    row::{RowData, RowDataBinary, RowDataText, RowDataTyp},
    server::codec::{make_eof_packet, CommonPacket, PacketSend},
    util::{is_eof, length_encode_int, BufMutExt},
};
use pisa_error::error::{Error, ErrorKind};
use rayon::prelude::*;
use serde::de::IntoDeserializer;
use strategy::sharding_rewrite::{DataSource, ShardingRewriteOutput, RewriteChange, meta::FieldWrapFunc, rewrite_const::{AVG_COUNT, AVG_SUM, AVG_FIELD}};
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
        attrs: Vec<SessionAttr>,
        is_get_conn: bool,
    ) -> Result<(), Error> {
        let mut curr_server_stmt_id: Option<u32> = None;
        let mut curr_cached_stmt_id = vec![];

        let conns = if is_get_conn {
            Self::get_shard_conns(&req.rewrite_outputs, req.pool.clone(), attrs).await?
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

        let mut conns = Self::shard_send_query(conns, &req.rewrite_outputs).await?;
        let shards_length = conns.len();
        let mut shard_streams = Vec::with_capacity(shards_length);

        for conn in conns.iter_mut() {
            let s = ResultsetStream::new(conn.framed.as_mut());
            shard_streams.push(s.fuse());
        }

        let mut merge_stream = MergeStream::new(shard_streams, shards_length);

        let sharding_column = req.rewrite_outputs[0].sharding_column.clone();
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
        let mut header = if let Some(header) = Self::get_shard_one_data(header)? {
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

        let ro = &req.rewrite_outputs[0];
        let avg_change = ro.changes.iter().find_map(|x| {
            if let RewriteChange::AvgChange(change) = x {
                Some(change)
            } else {
                None
            }
        });

        if let Some(_) = avg_change {
            header[4] = header[4] - 1;
        }
        
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

        // Save min or max row data
        let mut agg_buf = Vec::with_capacity(1024);

        while let Some(chunk) = stream.next().await {
            let mut chunk = chunk
                .into_par_iter().map(|x| x.unwrap()).collect::<Result<Vec<_>, _>>().map_err(ErrorKind::from)?;

            
            let ro = &req.rewrite_outputs[0];
            Self::handle_min_max(ro, &mut chunk, row_data.clone(), is_binary, &mut agg_buf)?;
            
            let mut avg_chunk = vec![];

            let avg_change = ro.changes.iter().find_map(|x| {
                if let RewriteChange::AvgChange(change) = x {
                    Some(change)
                } else {
                    None
                }
            });

            if let Some(avg) = avg_change {
                let count_field = avg.target.get(AVG_COUNT).unwrap();
                let sum_field = avg.target.get(AVG_SUM).unwrap();
                avg_chunk = chunk.par_iter().map(|x| {
                    let mut row_data = row_data.clone();
                    
                    row_data.with_buf(&x[4..]);
                    if is_binary {
                        let count = row_data.decode_with_name::<u64>(count_field).unwrap().unwrap();
                        let sum = row_data.decode_with_name::<u64>(sum_field).unwrap().unwrap();
                        (count, sum)
                    } else {
                        let count = row_data.decode_with_name::<String>(count_field).unwrap().unwrap();
                        let count = count.parse::<u64>().unwrap();

                        let sum = row_data.decode_with_name::<String>(sum_field).unwrap().unwrap();
                        let sum = sum.parse::<u64>().unwrap();
                        (count, sum)
                    }
                }).collect::<Vec<_>>();

                let count: u64 = avg_chunk.par_iter().map(|x| x.0).sum();
                let sum: u64 = avg_chunk.par_iter().map(|x| x.1).sum();

                let _ = chunk.par_iter_mut().map(|x| {
                    let mut row_data = row_data.clone();
                    let mut data = x.split_off(4);
                    row_data.with_buf(&data[..]);

                    let count_data = row_data.get_row_data_with_name(count_field).unwrap().unwrap();
                    let sum_data = row_data.get_row_data_with_name(sum_field).unwrap().unwrap();

                    let count_sum_data_length = count_data.part_data_length + sum_data.part_data_length;
                    let count_sum_length= count_data.part_encode_length + sum_data.part_encode_length + count_sum_data_length;
                    for _ in count_data.start_idx .. count_data.start_idx + count_sum_length {
                        data.get_u8();
                    }

                    x.extend_from_slice(&data[..]);

                    let mut buf = Vec::with_capacity(32);
                    let avg = format!("{:.4}", (sum as f64 / count as f64));
                    buf.put_lenc_int(avg.len() as u64, true);
                    buf.put_slice(avg.as_bytes());
                    x.put_slice(&buf);
                }).collect::<Vec<_>>();

                // When columns has count and sum field only, return directly.
                if col_info.len() == 2 {
                    let _ = req
                        .framed
                        .codec_mut()
                        .encode(PacketSend::EncodeOffset(chunk[0][4..].into(), buf.len()), buf);
                    return Ok(());
                }
            }

            if let Some(count_field) = &ro.count_field {
                let count_sum = chunk.par_iter().map(|x| {
                    let mut row_data = row_data.clone();
                    row_data.with_buf(&x[4..]);
                    if is_binary {
                        let count = row_data.decode_with_name::<u64>(&count_field.name).unwrap().unwrap();
                        count
                    } else {
                        let count = row_data.decode_with_name::<String>(&count_field.name).unwrap().unwrap();
                        let count = count.parse::<u64>().unwrap();
                        count
                    }
                }).sum::<u64>();

                let mut count_row_buf = vec![];
                
                if is_binary {
                    count_row_buf.put_u8(0);
                    for _ in 0..(col_info.len() + 7 + 2) >> 3 {
                        count_row_buf.put_u8(0)
                    }
                    count_row_buf.extend_from_slice(&count_sum.to_le_bytes()[..])
                } else {
                    let count_sum_str = count_sum.to_string();
                    count_row_buf.put_lenc_int(count_sum_str.len() as u64, false);
                    count_row_buf.extend_from_slice(count_sum_str.as_bytes());
                }

                let _ = req
                    .framed
                    .codec_mut()
                    .encode(PacketSend::EncodeOffset(count_row_buf[..].into(), buf.len()), buf);
                return Ok(());
            }
            
            if col_info.len() == ro.min_max_fields.len() {
                if is_binary {
                    agg_buf.insert(0, 0);
                    for  _ in 0..(col_info.len() + 7 + 2) >> 3 {
                        agg_buf.insert(1, 0);
                    }
                }
                let _ = req
                    .framed
                    .codec_mut()
                    .encode(PacketSend::EncodeOffset(agg_buf[..].into(), buf.len()), buf);
                return Ok(());
            }

            if let Some(name) = &sharding_column {
                if let Some(_) = col_info.iter().find(|col_info| col_info.column_name.eq(name)) {
                    chunk.par_sort_by_cached_key(|x| {
                        let mut row_data = row_data.clone();
                        row_data.with_buf(&x[4..]);
                        if is_binary {
                            row_data.decode_with_name::<u64>(&name).unwrap().unwrap()
                        } else {
                            let value = row_data.decode_with_name::<String>(&name).unwrap().unwrap();
                            value.parse::<u64>().unwrap()
                        }
                    })
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
    
    fn handle_min_max<B: BufMut>(ro: &ShardingRewriteOutput, chunk: &mut [BytesMut], row_data: RowDataTyp<&[u8]>, is_binary: bool, agg_buf: &mut B) -> Result<(), Error> {
        for (idx, mmf) in ro.min_max_fields.iter().enumerate() {
            match mmf.wrap_func {
                FieldWrapFunc::Max => {
                    chunk.par_sort_unstable_by(|a, b| {
                        let mut row_data = row_data.clone();
                        
                        row_data.with_buf(&a[4..]);
                        let a = if is_binary {
                            row_data.decode_with_name::<u64>(&mmf.name).unwrap().unwrap()
                        } else {
                            let value = row_data.decode_with_name::<String>(&mmf.name).unwrap().unwrap();
                            value.parse::<u64>().unwrap()
                        };

                        row_data.with_buf(&b[4..]);
                        let b = if is_binary {
                            row_data.decode_with_name::<u64>(&mmf.name).unwrap().unwrap()
                        } else {
                            let value = row_data.decode_with_name::<String>(&mmf.name).unwrap().unwrap();
                            value.parse::<u64>().unwrap()
                        };

                        b.cmp(&a)
                    });
                }
                FieldWrapFunc::Min => {
                    chunk.par_sort_unstable_by(|a, b| {
                        let mut row_data = row_data.clone();
                        
                        row_data.with_buf(&a[4..]);
                        let a = if is_binary {
                            row_data.decode_with_name::<u64>(&mmf.name).unwrap().unwrap()
                        } else {
                            let value = row_data.decode_with_name::<String>(&mmf.name).unwrap().unwrap();
                            value.parse::<u64>().unwrap()
                        };

                        row_data.with_buf(&b[4..]);
                        let b = if is_binary {
                            row_data.decode_with_name::<u64>(&mmf.name).unwrap().unwrap()
                        } else {
                            let value = row_data.decode_with_name::<String>(&mmf.name).unwrap().unwrap();
                            value.parse::<u64>().unwrap()
                        };

                        a.cmp(&b)
                    });

                }
                _ => break
            }

            let mut row_data = row_data.clone();
            row_data.with_buf(&chunk[0][4..]);
            let row = row_data.get_row_data_with_name(&mmf.name).map_err(|e| ErrorKind::Runtime(e))?;
            if let Some(row) = row {
                agg_buf.put_slice(&row.data);
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

        let ro = &req.rewrite_outputs[0];
        let avg_change = ro.changes.iter().find_map(|x| {
            if let RewriteChange::AvgChange(change) = x {
                Some(change)
            } else {
                None
            }
        });

        let mut avg_column_buf = Vec::with_capacity(128);
        if let Some(change) = avg_change {
            let avg_field = change.target.get(AVG_FIELD).unwrap();
            let avg_column = ColumnInfo {
                schema: None,
                table_name: None,
                column_name: avg_field.to_string(),
                charset: 0x3f,
                column_length: 0x46,
                column_type: ColumnType::MYSQL_TYPE_NEWDECIMAL,
                column_flag: 0x0080,
                decimals: 4,
            };

            avg_column.encode(&mut avg_column_buf);
        }
        
        //let column_infos = Vec::with_capacity(column_length as usize);
        let mut is_add_avg_column  = false;

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
            let column_info = decode_column(&data[4..]);
            if let Some(change) = avg_change {
                let avg_count = change.target.get(AVG_COUNT).unwrap();
                let avg_sum = change.target.get(AVG_SUM).unwrap();
                if &column_info.column_name == avg_count || &column_info.column_name == avg_sum {
                    if !is_add_avg_column {
                        let _ = req
                            .framed
                            .codec_mut()
                            .encode(PacketSend::EncodeOffset(avg_column_buf[..].into(), buf.len()), buf);
                    }

                    is_add_avg_column = true;
                    continue;
                }
            }

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
        attrs: Vec<SessionAttr>,
        _is_get_conn: bool,
    ) -> Result<(Vec<Stmt>, Vec<PoolConn<ClientConn>>), Error> {
        let conns = Self::get_shard_conns(&req.rewrite_outputs, req.pool.clone(), attrs).await?;
        let mut send_futs = FuturesOrdered::new();
        let mut sended_conns = Vec::with_capacity(conns.len());

        for (mut conn, ro) in conns.into_iter().zip(req.rewrite_outputs.iter()) {
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
