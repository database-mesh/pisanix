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

use std::collections::HashMap;

use mysql_parser::ast::SqlStmt;

#[derive(Clone)]
pub struct ParserAstCache {
    pub ast_cache: HashMap<String, Vec<SqlStmt>>,
}

impl ParserAstCache {
    pub fn new() -> ParserAstCache {
        ParserAstCache { ast_cache: HashMap::new() }
    }

    pub fn set(&mut self, key: String, sql_stmt: Vec<SqlStmt>) {
        self.ast_cache.insert(key, sql_stmt);
    }

    pub fn get(&self, key: String) -> Option<&Vec<SqlStmt>> {
        self.ast_cache.get(&key)
    }

    //TODO: local storage
}
