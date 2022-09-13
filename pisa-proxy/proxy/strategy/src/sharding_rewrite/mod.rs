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

mod meta;

use mysql_parser::ast::{SqlStmt, Visitor};

use crate::{config::Sharding, rewrite::ShardingRewriter};

use self::meta::RewriteMetaData;

pub struct ShardingRewrite<'a> {
    rule: &'a Sharding,
    // Raw sql
    raw_sql: &'a str,
}

impl<'a> ShardingRewrite<'a> {
    pub fn new(rule: &'a Sharding, raw_sql: &'a str) -> Self {
        ShardingRewrite { rule, raw_sql }
    }

    fn parse_rule(&self) {
        //self.rule.database_table_strategy.unwrap().

    }

    fn get_meta(&self, mut ast: SqlStmt) -> RewriteMetaData {
        let mut meta = RewriteMetaData::default();
        let _ = ast.visit(&mut meta);
        meta
    }
}

impl<'a> ShardingRewriter<SqlStmt> for ShardingRewrite<'a> {
    fn rewrite(&mut self, ast: SqlStmt) {
       let meta = self.get_meta(ast);

    }
}



