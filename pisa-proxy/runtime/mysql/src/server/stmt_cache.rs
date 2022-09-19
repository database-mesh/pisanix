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

use conn_pool::PoolConn;
use indexmap::IndexMap;
use mysql_protocol::client::{conn::ClientConn, stmt};

struct Entry {
    id: u32,
    conn: PoolConn<ClientConn>,
}

pub struct StmtCache {
    // key is generated id by pisa, value is returnd stmt id from client
    cache: IndexMap<u32, Vec<Entry>>,
}

impl StmtCache {
    pub fn new() -> Self {
        Self { 
            cache: IndexMap::new() }
    }

    pub fn put(&mut self, server_stmt_id: u32, stmt_id: u32, conn: PoolConn<ClientConn>) {
        let entry = Entry {
            id: stmt_id,
            conn,
        };

        let value = self.cache.entry(server_stmt_id).or_insert(vec![]);
        let idx = value.iter().position(|x| x.id == stmt_id);
        if let Some(idx) = idx {
            value[idx] = entry;
        } else {
            value.push(entry);
        }
    }

    pub fn get(&mut self, server_stmt_id: u32, stmt_id: u32) -> Option<PoolConn<ClientConn>> {
        let value = self.cache.get_mut(&server_stmt_id);
        let value = if let Some(value) = value {
            value
        } else {
            return None;
        };

        let idx = value.iter().position(|x| x.id == stmt_id);
        if let Some(idx) = idx {
            Some(value.remove(idx).conn)
        } else {
            None
        }
    }

    pub fn get_all(&mut self, server_stmt_id: u32) -> Vec<(u32, PoolConn<ClientConn>)> {
        let value = self.cache.remove(&server_stmt_id);
        let mut value = if let Some(value) = value {
            value
        } else {
            return vec![]
        };

        value.drain(..).map(|x| (x.id, x.conn)).collect::<Vec<_>>()
    }

    pub fn put_all(&mut self, server_stmt_id: u32, conns: Vec<(u32, PoolConn<ClientConn>)>) {
        let value = self.cache.entry(server_stmt_id).or_insert(vec![]);
        for pair in conns.into_iter() {
            let entry = Entry {
                id: pair.0 ,
                conn: pair.1,               
            };

            let idx = value.iter().position(|x| x.id == pair.0);
            if let Some(idx) = idx {
                value[idx] = entry;
            } else {
                value.push(entry)
            }
        }
    }

    pub fn remove(&mut self, server_stmt_id: u32) {
        self.cache.remove(&server_stmt_id);
    }
}
