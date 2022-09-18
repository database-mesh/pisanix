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

use conn_pool::{Pool, PoolConn};
use indexmap::IndexMap;
use mysql_protocol::client::conn::ClientConn;
use tracing::Id;

type CacheValue = IndexMap<u32, PoolConn<ClientConn>>;
pub struct StmtCache {
    // key is generated id by pisa, value is returnd stmt id from client
    cache: IndexMap<u32, CacheValue>,
}

impl StmtCache {
    pub fn new() -> Self {
        Self { 
            cache: IndexMap::new() }
    }

    pub fn put(&mut self, server_stmt_id: u32, stmt_id: u32, conn: PoolConn<ClientConn>) {
        self.cache.entry(server_stmt_id).or_insert(CacheValue::new())
            .insert(stmt_id, conn);
    }

    pub fn get(&mut self, server_stmt_id: u32, stmt_id: u32) -> Option<PoolConn<ClientConn>> {
        let value = self.cache.get_mut(&server_stmt_id);
        let value = if let Some(value) = value {
            value
        } else {
            return None;
        };

        value.remove(&stmt_id)
    }

    pub fn remove(&mut self, server_stmt_id: u32) {
        self.cache.remove(&server_stmt_id);
    }
}
