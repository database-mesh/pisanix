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

use lru::LruCache;
use mysql_parser::ast::SqlStmt;

pub struct ParserAstCache {
    pub ast_cache: LruCache<String, Vec<SqlStmt>>,
}

impl ParserAstCache {
    pub fn new() -> ParserAstCache {
        // LruCache default size is 512
        // TODO: make LruCache size as a parameter
        ParserAstCache { ast_cache: LruCache::new(512) }
    }

    pub fn set(&mut self, key: String, sql_stmt: Vec<SqlStmt>) {
        self.ast_cache.put(key, sql_stmt);
    }

    pub fn get(&mut self, key: String) -> Option<&Vec<SqlStmt>> {
        self.ast_cache.get(&key)
    }
}
