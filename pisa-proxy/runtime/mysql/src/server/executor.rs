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

use byteorder::{ByteOrder, LittleEndian};
use bytes::BytesMut;
use conn_pool::{Pool, PoolConn};
use futures::{stream::FuturesOrdered, SinkExt, StreamExt};
use mysql_protocol::{
    client::{
        codec::{MergeResultsetState, MergeStream, ResultsetStream},
        conn::{ClientConn, SessionAttr},
        stmt::Stmt,
    },
    column::{decode_column, ColumnInfo},
    err::ProtocolError,
    mysql_const::*,
    row::{decode_with_name, RowData, RowDataBinary, RowDataText, RowDataTyp, RowPartData},
    server::codec::{make_eof_packet, CommonPacket, PacketSend},
    util::{length_encode_int, BufMutExt},
};
use pisa_error::error::{Error, ErrorKind};
use rayon::prelude::*;
use strategy::sharding_rewrite::{
    meta::FieldWrapFunc, ChangeTargetMeta, DataSource, ShardingColumn, ShardingRewriteOutput,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Encoder};

use super::util::{filter_avg_column, get_avg_change};
use crate::{mysql::ReqContext, transaction_fsm::check_get_conn};

#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    #[error("execute sql: {0:?} error")]
    DataSourceNotFound(String),
}

pub struct Executor<T, C> {
    _phant: PhantomData<(T, C)>,
}

struct ColumnUpdate {
    ori_columns: Arc<[ColumnInfo]>,
    new_columns: Arc<[ColumnInfo]>,
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

        let sharding_column = req.rewrite_outputs.results[0].ds_idx.column.clone();
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
        sharding_column: ShardingColumn,
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

        let ro = &req.rewrite_outputs.results[0];
        let avg_change = ro.changes.iter().find_map(|x| {
            x.1.iter().find_map(|meta| {
                if let ChangeTargetMeta::Avg { .. } = meta {
                    Some(meta.clone())
                } else {
                    None
                }
            })
        });

        if let Some(_) = avg_change {
            header[4] = header[4] - 1;
        }

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(header[4..].into(), 0), &mut buf);

        let column_update = Self::get_columns(req, merge_stream, cols, &mut buf).await?;

        // read eof
        let _ = merge_stream.next().await;

        let _ = req
            .framed
            .codec_mut()
            .encode(PacketSend::EncodeOffset(make_eof_packet()[4..].into(), buf.len()), &mut buf);

        merge_stream.set_state(MergeResultsetState::Row);
        // get rows
        Self::get_rows(req, merge_stream, &mut buf, sharding_column, column_update, is_binary)
            .await?;

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
        sharding_column: ShardingColumn,
        column_update: ColumnUpdate,
        is_binary: bool,
    ) -> Result<(), Error> {
        let mut row_data = match is_binary {
            false => {
                let row_data_text = RowDataText::new(column_update.ori_columns.clone(), &[][..]);
                RowDataTyp::Text(row_data_text)
            }
            true => {
                let row_data_binary =
                    RowDataBinary::new(column_update.ori_columns.clone(), &[][..]);
                RowDataTyp::Binary(row_data_binary)
            }
        };

        while let Some(chunk) = stream.next().await {
            let mut chunk = chunk
                .into_par_iter()
                .map(|x| x.unwrap())
                .collect::<Result<Vec<_>, _>>()
                .map_err(ErrorKind::from)?;

            let ro = &req.rewrite_outputs;
            Self::handle_min_max(ro, &mut chunk, row_data.clone(), is_binary)?;

            let avg_change = get_avg_change(&ro.results[0].changes);

            if let Some(avg) = avg_change {
                let (count_data, sum_data): (Vec<_>, Vec<_>) = chunk
                    .par_iter()
                    .map(|x| -> Result<(u64, u64), Error> {
                        let mut row_data = row_data.clone();
                        row_data.with_buf(&x[4..]);
                        let count = decode_with_name::<&[u8], u64>(
                            &mut row_data,
                            &avg.count_field,
                            is_binary,
                        )
                        .map_err(|e| ErrorKind::Runtime(e))?
                        .unwrap_or_else(|| 0);
                        // Sum type is `MYSQL_TYPE_NEWDECIMAL` in binary, so need to convet to `String` type.
                        let sum = if is_binary {
                            let sum = decode_with_name::<&[u8], String>(
                                &mut row_data,
                                &avg.sum_field,
                                is_binary,
                            )
                            .map_err(|e| ErrorKind::from(e))?;
                            if let Some(sum) = sum {
                                sum.parse::<u64>().map_err(|e| ErrorKind::Runtime(e.into()))?
                            } else {
                                0
                            }
                        } else {
                            decode_with_name::<&[u8], u64>(&mut row_data, &avg.sum_field, is_binary)
                                .map_err(|e| ErrorKind::Runtime(e))?
                                .unwrap_or_else(|| 0)
                        };

                        Ok((count, sum))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .par_iter()
                    .cloned()
                    .unzip();

                let count: u64 = count_data.par_iter().sum();
                let sum: u64 = sum_data.par_iter().sum();

                chunk.par_iter_mut().for_each(|x| {
                    let mut row_data = row_data.clone();
                    row_data.with_buf(&x[4..]);
                    let count_data =
                        row_data.get_row_data_with_name(&avg.count_field).unwrap().unwrap();
                    let sum_data =
                        row_data.get_row_data_with_name(&avg.sum_field).unwrap().unwrap();
                    let part_data = RowPartData {
                        data: vec![].into(),
                        start_idx: count_data.start_idx,
                        part_encode_length: count_data.part_encode_length
                            + sum_data.part_encode_length,
                        part_data_length: count_data.part_data_length + sum_data.part_data_length,
                    };

                    let avg = format!("{:.4}", (sum as f64 / count as f64));

                    row_data_cut_merge(x, &part_data, |data: &mut BytesMut| {
                        data.put_lenc_int(avg.len() as u64, false);
                        data.extend_from_slice(avg.as_bytes());
                    });
                });

                // When columns has count and sum field only, return directly.
                if column_update.ori_columns.len() == 2 {
                    let _ = req
                        .framed
                        .codec_mut()
                        .encode(PacketSend::EncodeOffset(chunk[0][4..].into(), buf.len()), buf);
                    return Ok(());
                }

                row_data = match is_binary {
                    false => {
                        let row_data_text =
                            RowDataText::new(column_update.new_columns.clone(), &[][..]);
                        RowDataTyp::Text(row_data_text)
                    }
                    true => {
                        let row_data_binary =
                            RowDataBinary::new(column_update.new_columns.clone(), &[][..]);
                        RowDataTyp::Binary(row_data_binary)
                    }
                };
            }

            let count_field = ro.agg_fields.iter().find_map(|x| {
                x.1.iter().find_map(|meta| {
                    if meta.wrap_func == FieldWrapFunc::Count {
                        Some(meta)
                    } else {
                        None
                    }
                })
            });

            if let Some(count_field) = count_field {
                let count_sum = chunk
                    .par_iter()
                    .map(|x| {
                        let mut row_data = row_data.clone();
                        row_data.with_buf(&x[4..]);
                        decode_with_name::<&[u8], u64>(&mut row_data, &count_field.name, is_binary)
                            .unwrap()
                            .unwrap()
                    })
                    .sum::<u64>();

                let chunk_data = &chunk[0];
                let mut row_data = row_data.clone();
                row_data.with_buf(&chunk_data[4..]);
                let row_part_data = row_data
                    .get_row_data_with_name(&count_field.name)
                    .map_err(|e| ErrorKind::Runtime(e))?
                    .unwrap();
                chunk.par_iter_mut().for_each(|x| {
                    row_data_cut_merge(x, &row_part_data, |data: &mut BytesMut| {
                        if is_binary {
                            data.extend_from_slice(&count_sum.to_le_bytes()[..])
                        } else {
                            let count_sum_str = count_sum.to_string();
                            data.put_lenc_int(count_sum_str.len() as u64, false);
                            data.extend_from_slice(count_sum_str.as_bytes());
                        }
                    });
                });
            }

            if let Some(name) = &sharding_column.table {
                if let Some(_) =
                    column_update.ori_columns.iter().find(|col_info| col_info.column_name.eq(name))
                {
                    chunk.par_sort_by_cached_key(|x| {
                        let mut row_data = row_data.clone();
                        row_data.with_buf(&x[4..]);
                        if is_binary {
                            row_data.decode_with_name::<u64>(&name).unwrap().unwrap()
                        } else {
                            let value =
                                row_data.decode_with_name::<String>(&name).unwrap().unwrap();
                            value.parse::<u64>().unwrap()
                        }
                    })
                }
            }

            if chunk.par_iter().map(|x| &x[4..]).min() == chunk.par_iter().map(|x| &x[4..]).max() {
                let _ = req
                    .framed
                    .codec_mut()
                    .encode(PacketSend::EncodeOffset(chunk[0][4..].into(), buf.len()), buf);
                return Ok(());
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

    fn handle_min_max(
        ro: &ShardingRewriteOutput,
        chunk: &mut [BytesMut],
        row_data: RowDataTyp<&[u8]>,
        is_binary: bool,
    ) -> Result<(), Error> {
        let default_agg_fields = vec![];
        let agg_fields = ro.agg_fields.get(&1).unwrap_or_else(|| &default_agg_fields);
        for agg in agg_fields.iter() {
            match agg.wrap_func {
                FieldWrapFunc::Max => {
                    chunk.par_sort_unstable_by(|a, b| {
                        let mut row_data = row_data.clone();
                        let (a, b) = get_min_max_value(&mut row_data, is_binary, &agg.name, a, b);
                        b.cmp(&a)
                    });
                }
                FieldWrapFunc::Min => {
                    chunk.par_sort_unstable_by(|a, b| {
                        let mut row_data = row_data.clone();
                        let (a, b) = get_min_max_value(&mut row_data, is_binary, &agg.name, a, b);
                        a.cmp(&b)
                    });
                }
                _ => {}
            }

            let chunk_data = &chunk[0];
            let ori_row_data = row_data.clone();
            let mut row_data = row_data.clone();
            row_data.with_buf(&chunk_data[4..]);
            let row_part_data = row_data.get_row_data_with_name(&agg.name).map_err(|e| ErrorKind::Runtime(e))?.unwrap();
            chunk.par_iter_mut().for_each(|x| {
                let mut row_data = ori_row_data.clone();
                row_data.with_buf(&x[4..]);
                let ori_row_part_data = row_data.get_row_data_with_name(&agg.name).unwrap().unwrap();

                row_data_cut_merge(x, &ori_row_part_data, |data: &mut BytesMut| {
                    if is_binary {
                        data.extend_from_slice(&row_part_data.data);
                    } else {
                        data.put_lenc_int(row_part_data.part_data_length as u64, false);
                        data.extend_from_slice(&row_part_data.data);
                    }
                })
            });
        }

        Ok(())
    }

    async fn get_columns<'a>(
        req: &mut ReqContext<T, C>,
        stream: &mut MergeStream<ResultsetStream<'a>>,
        column_length: u64,
        buf: &mut BytesMut,
    ) -> Result<ColumnUpdate, Error> {
        let mut ori_columns = Vec::with_capacity(32);
        let mut new_columns = Vec::with_capacity(32);
        let mut idx: Option<usize> = None;

        let ro = &req.rewrite_outputs.results[0];
        let avg_change = ro.changes.iter().find_map(|x| {
            x.1.iter().find_map(|meta| {
                if let ChangeTargetMeta::Avg(change) = meta {
                    Some(change)
                } else {
                    None
                }
            })
        });

        //let mut avg_column_buf = Vec::with_capacity(128);
        let mut is_added_avg_column = false;

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

            let column_info = decode_column(&data[4..]);
            ori_columns.push(column_info.clone());

            if let Some(change) = avg_change {
                let filter_res = filter_avg_column(change, &column_info, is_added_avg_column);
                if let Some(avg_column) = filter_res.1 {
                    if !filter_res.0.is_empty() {
                        let _ = req
                            .framed
                            .codec_mut()
                            .encode(PacketSend::EncodeOffset(filter_res.0.into(), buf.len()), buf);
                        new_columns.push(avg_column)
                    }

                    is_added_avg_column = true;
                    continue;
                }
            }

            new_columns.push(column_info);

            let _ = req
                .framed
                .codec_mut()
                .encode(PacketSend::EncodeOffset(data[4..].into(), buf.len()), buf);
        }

        Ok(ColumnUpdate { ori_columns: ori_columns.into(), new_columns: new_columns.into() })
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
        rewrite_outputs: &ShardingRewriteOutput,
    ) -> Result<Vec<PoolConn<ClientConn>>, Error> {
        let mut send_futs = FuturesOrdered::new();
        let mut sended_conns = Vec::with_capacity(conns.len());

        for (mut conn, ro) in conns.into_iter().zip(rewrite_outputs.results.iter()) {
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
        rewrite_outputs: &ShardingRewriteOutput,
        pool: Pool<ClientConn>,
        attrs: Vec<SessionAttr>,
    ) -> Result<Vec<PoolConn<ClientConn>>, Error> {
        let endpoints = rewrite_outputs
            .results
            .iter()
            .map(|x| match &x.ds_idx.ds {
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

        for (mut conn, ro) in conns.into_iter().zip(req.rewrite_outputs.results.iter()) {
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

fn row_data_cut_merge<F>(ori_data: &mut BytesMut, row_part_data: &RowPartData, f: F)
where
    F: FnOnce(&mut BytesMut),
{
    let mut data = ori_data.split_off(4);
    let mut data_remain = data.split_off(row_part_data.start_idx);

    f(&mut data);

    let _ = data_remain.split_to(row_part_data.part_encode_length + row_part_data.part_data_length);
    data.extend_from_slice(&data_remain);
    ori_data.extend_from_slice(&data);
}

fn get_min_max_value<'a>(
    row_data: &mut RowDataTyp<&'a [u8]>,
    is_binary: bool,
    name: &'a str,
    a: &'a BytesMut,
    b: &'a BytesMut,
) -> (u64, u64) {
    row_data.with_buf(&a[4..]);
    let a = decode_with_name::<&[u8], u64>(row_data, name, is_binary).unwrap().unwrap();

    row_data.with_buf(&b[4..]);
    let b = decode_with_name::<&[u8], u64>(row_data, name, is_binary).unwrap().unwrap();
    (a, b)
}
