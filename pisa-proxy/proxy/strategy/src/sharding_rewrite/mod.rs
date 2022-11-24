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

mod generic_meta;
mod macros;
pub mod meta;
pub mod rewrite_const;

use std::vec;

use crc32fast::Hasher;
use endpoint::endpoint::Endpoint;
use indexmap::IndexMap;
use mysql_parser::ast::{SqlStmt, TableIdent, Visitor};
use paste::paste;

use self::{
    generic_meta::{ShardingMeta, ShardingMetaBaseInfo},
    meta::{
        FieldMeta, FieldMetaIdent, InsertValsMeta, RewriteMetaData, WhereMeta,
        WhereMetaRightDataType,
    },
};
use crate::{
    config::{NodeGroup, Sharding, ShardingAlgorithmName},
    get_meta_detail,
    rewrite::{ShardingRewriteInput, ShardingRewriter},
    sharding_rewrite::{
        meta::{AvgMeta, FieldWrapFunc, GroupMeta, OrderDirection, OrderMeta},
        rewrite_const::*,
    },
};

pub trait CalcShardingIdx<I> {
    fn calc(self, algo: &ShardingAlgorithmName, sharding_count: I) -> Option<u64>;
}

impl CalcShardingIdx<u64> for u64 {
    fn calc(self, algo: &ShardingAlgorithmName, sharding_count: u64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => Some(self.wrapping_rem(sharding_count)),
            ShardingAlgorithmName::CRC32Mod => {
                let mut hasher = Hasher::new();
                hasher.update(&self.to_be_bytes());
                let checksum = hasher.finalize();
                Some(checksum.wrapping_rem(sharding_count.try_into().unwrap()).into())
            }
        }
    }
}

impl CalcShardingIdx<i64> for i64 {
    fn calc(self, algo: &ShardingAlgorithmName, sharding_count: i64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => Some(self.wrapping_rem(sharding_count) as u64),
            ShardingAlgorithmName::CRC32Mod => {
                let mut hasher = Hasher::new();
                hasher.update(&self.to_be_bytes());
                let checksum = hasher.finalize();
                Some(checksum.wrapping_rem(sharding_count.try_into().unwrap()).into())
            }
        }
    }
}

impl CalcShardingIdx<f64> for f64 {
    fn calc(self, algo: &ShardingAlgorithmName, sharding_count: f64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => Some((self % sharding_count).round() as u64),
            ShardingAlgorithmName::CRC32Mod => {
                let mut hasher = Hasher::new();
                hasher.update(&self.to_be_bytes());
                let checksum = hasher.finalize();
                Some(((checksum as f64) % sharding_count).round() as u64)
            }
        }
    }
}

#[derive(Debug)]
pub enum RewriteChange {
    TableChange(TableChange),
    AvgChange(AvgChange),
    OrderChange(OrderChange),
    GroupChange(GroupChange),
}

#[derive(Debug, Clone)]
pub struct TableChange {
    pub span: mysql_parser::Span,
    pub table: Option<TableChangeDetail>,
    pub database: Option<TableChangeDetail>,
    pub rule: Sharding,
}

#[derive(Debug, Clone)]
pub struct TableChangeDetail {
    pub target: String,
    pub shard_idx: u64,
}

#[derive(Debug)]
pub struct AvgChange {
    // avg sql rewrite target
    // example: AVG(pbl): avg_count: PBL_AVG_DERIVED_COUNT_00000, avg_sum: PBL_AVG_DERIVED_SUM_00000
    pub target: IndexMap<String, String>,
}

#[derive(Debug)]
pub struct OrderChange {
    pub target: IndexMap<String, String>,
    pub direction: OrderDirection,
}

impl Default for OrderChange {
    fn default() -> OrderChange {
        OrderChange { target: IndexMap::new(), direction: OrderDirection::Asc }
    }
}

#[derive(Debug)]
pub struct GroupChange {
    pub target: IndexMap<String, String>,
}

impl Default for GroupChange {
    fn default() -> GroupChange {
        GroupChange { target: IndexMap::new() }
    }
}

#[derive(Debug)]
pub struct ShardingRewriteOutput {
    pub changes: Vec<RewriteChange>,
    pub target_sql: String,
    pub data_source: DataSource,
    pub sharding_column: Option<String>,
    pub min_max_fields: Vec<FieldMetaIdent>,
    pub count_field: Option<FieldMetaIdent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataSource {
    Endpoint(Endpoint),
    NodeGroup(String),
    None,
}

#[derive(Debug, Clone)]
pub struct ShardingRewrite {
    rules: Vec<Sharding>,
    // Raw sql
    raw_sql: String,

    // Endpoints
    endpoints: Vec<Endpoint>,
    // Whether has readwritesplitting
    has_rw: bool,

    // Node Group Config
    node_group_config: Option<NodeGroup>,

    // Default database from client
    default_db: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ShardingRewriteError {
    #[error("sharding column not found")]
    ShardingColumnNotFound,

    #[error("parse str to u64 error {0:?}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("parse str to u64 error {0:?}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    #[error("calc sharding idx error")]
    CalcShardingIdxError,

    #[error("enpoint not found when using actual_datanodes")]
    EndpointNotFound,

    #[error("fields is empty")]
    FieldsIsEmpty,

    #[error("database is not found")]
    DatabaseNotFound,
}

struct ChangeInsertMeta {
    sharding_value: InsertShardingValue,
    value_span: mysql_parser::Span,
}

struct InsertShardingValue {
    database: Option<String>,
    table: Option<String>,
}

#[derive(Debug)]
struct ShardingIdx {
    database: Option<u64>,
    table: Option<u64>,
    span: mysql_parser::Span,
}

#[derive(Debug)]
pub enum StrategyTyp {
    DatabaseTable,
    Database,
    Table,
}

enum DatabaseTableStrategyPart {
    Database(usize),
    Table(usize),
    All,
}

pub enum OrderGroupChange {
    Order(OrderMeta),
    Group(GroupMeta),
}

impl ShardingRewrite {
    pub fn new(
        rules: Vec<Sharding>,
        endpoints: Vec<Endpoint>,
        node_group_config: Option<NodeGroup>,
        has_rw: bool,
    ) -> Self {
        ShardingRewrite {
            rules,
            raw_sql: "".to_string(),
            endpoints,
            node_group_config,
            has_rw,
            default_db: None,
        }
    }

    pub fn get_endpoints(&self) -> &Vec<Endpoint> {
        &self.endpoints
    }

    pub fn set_raw_sql(&mut self, raw_sql: String) {
        self.raw_sql = raw_sql;
    }

    pub fn set_default_db(&mut self, db: Option<String>) {
        self.default_db = db;
    }

    fn database_table_strategy(
        &self,
        meta: RewriteMetaData,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        get_meta_detail!(meta, wheres, inserts, fields, avgs, orders, groups);
        if !inserts.is_empty() {
            if fields.is_empty() {
                return Err(ShardingRewriteError::FieldsIsEmpty);
            }
            return self.change_insert_sql(try_tables, fields, inserts);
        }

        if wheres.is_empty() {
            return Ok(self.database_table_strategy_iproduct(
                DatabaseTableStrategyPart::All,
                try_tables,
                avgs,
                fields,
                orders,
                groups,
            ));
        }

        let wheres = Self::find_try_where(StrategyTyp::DatabaseTable, &try_tables, wheres)?
            .into_iter()
            .filter_map(|x| (!x.1.is_empty()).then(|| x))
            .collect::<Vec<_>>();

        if wheres.is_empty() {
            return Ok(self.database_table_strategy_iproduct(
                DatabaseTableStrategyPart::All,
                try_tables,
                avgs,
                fields,
                orders,
                groups,
            ));
        }

        // We don't consider subquery for now.
        // The table sharding exists only.
        if wheres[0].1[0].is_none() {
            let table_idx = wheres[0].1[1].unwrap() as usize;
            return Ok(self.database_table_strategy_iproduct(
                DatabaseTableStrategyPart::Table(table_idx),
                try_tables,
                avgs,
                fields,
                orders,
                groups,
            ));
        }

        // The database sharding exists only.
        if wheres[0].1[1].is_none() {
            let db_idx = wheres[0].1[0].unwrap() as usize;
            return Ok(self.database_table_strategy_iproduct(
                DatabaseTableStrategyPart::Database(db_idx),
                try_tables,
                avgs,
                fields,
                orders,
                groups,
            ));
        }

        let db_fn = |rule: &Sharding, shard_idx: u64| {
            let node = &rule.actual_datanodes[shard_idx as usize];
            let ep = self.endpoints.iter().find(|x| x.name.eq(node)).unwrap();
            Some(ep.db.as_str())
        };

        let would_changes =
            self.get_table_change_plan(StrategyTyp::DatabaseTable, &try_tables, &wheres, db_fn);

        let mut target_sql = self.raw_sql.to_string();

        let changes =
            Self::table_change_apply(&mut target_sql, StrategyTyp::DatabaseTable, &would_changes);

        let shard_idx = would_changes[0].1.database.as_ref().unwrap().shard_idx;
        let sharding_rule = &would_changes[0].1.rule;

        let data_source = self.gen_data_source(sharding_rule, shard_idx as usize)?;

        let sharding_column = sharding_rule.get_sharding_column().0.unwrap().to_string();
        let min_max_fields =
            changes.iter().map(|x| self.find_min_max_fields(&x.0, fields)).flatten().collect();

        let mut changes = changes.into_iter().map(|x| x.1).collect::<Vec<_>>();

        let _ = self.change_order_group(
            &mut target_sql,
            &mut changes,
            orders,
            groups,
            fields,
            shard_idx,
        );
        self.change_avg(&mut target_sql, &mut changes, avgs, shard_idx, 0);

        Ok(vec![ShardingRewriteOutput {
            changes,
            target_sql,
            data_source,
            sharding_column: Some(sharding_column),
            min_max_fields,
            count_field: Self::get_count_field(fields),
        }])
    }

    fn database_strategy(
        &self,
        meta: RewriteMetaData,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        get_meta_detail!(meta, wheres, inserts, fields, avgs, orders, groups);
        if !inserts.is_empty() {
            if fields.is_empty() {
                return Err(ShardingRewriteError::FieldsIsEmpty);
            }

            return self.change_insert_sql(try_tables, fields, inserts);
        }

        if wheres.is_empty() {
            return Ok(self.database_strategy_iproduct(try_tables, avgs, fields, orders, groups));
        }

        let wheres = Self::find_try_where(StrategyTyp::Database, &try_tables, wheres)?;
        if wheres.is_empty() {
            return Ok(self.database_strategy_iproduct(try_tables, avgs, fields, orders, groups));
        }

        let expect_sum = wheres[0].1[0].unwrap() as usize * wheres.len();
        let sum: usize = wheres.iter().map(|x| x.1[0].unwrap()).sum::<u64>() as usize;

        if expect_sum != sum {
            return Ok(self.database_strategy_iproduct(try_tables, avgs, fields, orders, groups));
        }

        let db_fn = |rule: &Sharding, shard_idx: u64| {
            let node = &rule.actual_datanodes[shard_idx as usize];
            let ep = self.endpoints.iter().find(|x| x.name.eq(node)).unwrap();
            Some(ep.db.as_str())
        };

        let mut target_sql = self.raw_sql.to_string();
        let would_changes =
            self.get_table_change_plan(StrategyTyp::Database, &try_tables, &wheres, db_fn);
        let changes =
            Self::table_change_apply(&mut target_sql, StrategyTyp::Database, &would_changes);

        // Currently, we do not consider the endpoint corresponding to the subquery when the subquery exists.
        let shard_idx = would_changes[0].1.database.as_ref().unwrap().shard_idx;
        let sharding_rule = &would_changes[0].1.rule;

        let data_source = self.gen_data_source(sharding_rule, shard_idx as usize)?;

        let sharding_column = sharding_rule.get_sharding_column().0.unwrap().to_string();
        let min_max_fields =
            changes.iter().map(|x| self.find_min_max_fields(&x.0, fields)).flatten().collect();

        let mut changes = changes.into_iter().map(|x| x.1).collect::<Vec<_>>();

        let _ = self.change_order_group(
            &mut target_sql,
            &mut changes,
            orders,
            groups,
            fields,
            shard_idx,
        );
        self.change_avg(&mut target_sql, &mut changes, avgs, shard_idx, 0);

        Ok(vec![ShardingRewriteOutput {
            changes,
            target_sql,
            data_source,
            sharding_column: Some(sharding_column),
            min_max_fields,
            count_field: Self::get_count_field(fields),
        }])
    }

    fn table_strategy(
        &self,
        meta: RewriteMetaData,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        get_meta_detail!(meta, wheres, inserts, fields, avgs, orders, groups);

        if !inserts.is_empty() {
            if fields.is_empty() {
                return Err(ShardingRewriteError::FieldsIsEmpty);
            }

            return self.change_insert_sql(try_tables, fields, inserts);
        }

        if wheres.is_empty() {
            return Ok(self.table_strategy_iproduct(try_tables, avgs, fields, orders, groups));
        }

        let wheres = Self::find_try_where(StrategyTyp::Table, &try_tables, wheres)?;
        if wheres.is_empty() {
            return Ok(self.table_strategy_iproduct(try_tables, avgs, fields, orders, groups));
        }

        let expect_sum = wheres[0].1[0].unwrap() as usize * wheres.len();
        let sum: usize = wheres.iter().map(|x| x.1[0].unwrap()).sum::<u64>() as usize;

        if expect_sum != sum {
            return Ok(self.table_strategy_iproduct(try_tables, avgs, fields, orders, groups));
        }

        let db_fn = |_rule: &Sharding, _shard_idx: u64| None;
        let would_changes =
            self.get_table_change_plan(StrategyTyp::Table, &try_tables, &wheres, db_fn);

        let mut target_sql = self.raw_sql.clone();
        let sharding_rule = &would_changes[0].1.rule;
        let shard_idx: u64 = would_changes[0].1.table.as_ref().unwrap().shard_idx;

        let data_source = self.gen_data_source(sharding_rule, 0)?;

        let changes = Self::table_change_apply(&mut target_sql, StrategyTyp::Table, &would_changes);

        let sharding_column = sharding_rule.get_sharding_column().1.unwrap();

        let min_max_fields =
            changes.iter().map(|x| self.find_min_max_fields(&x.0, fields)).flatten().collect();
        let mut changes = changes.into_iter().map(|x| x.1).collect::<Vec<_>>();

        let _ = self.change_order_group(
            &mut target_sql,
            &mut changes,
            orders,
            groups,
            fields,
            shard_idx,
        );
        self.change_avg(&mut target_sql, &mut changes, avgs, shard_idx, 0);

        Ok(vec![ShardingRewriteOutput {
            changes,
            target_sql: target_sql.to_string(),
            data_source,
            sharding_column: Some(sharding_column.to_string()),
            min_max_fields,
            count_field: Self::get_count_field(fields),
        }])
    }

    fn get_table_change_plan<'a, F>(
        &self,
        strategy_typ: StrategyTyp,
        try_tables: &[(u8, Sharding, &TableIdent)],
        wheres: &[(u8, Vec<Option<u64>>)],
        db_fn: F,
    ) -> Vec<(u8, TableChange)>
    where
        F: Fn(&Sharding, u64) -> Option<&'a str>,
    {
        try_tables
            .iter()
            .filter_map(|x| {
                let w = wheres.iter().find(|w| w.0 == x.0);
                if let Some(w) = w {
                    //let actual_db = db_fn(&x.1, w.1).unwrap_or_else(|| "");
                    match strategy_typ {
                        StrategyTyp::Database => {
                            let actual_db = db_fn(&x.1, w.1[0].unwrap()).unwrap_or_else(|| "");
                            let target = self.change_table(x.2, actual_db, w.1[0].unwrap());
                            Some((
                                x.0,
                                TableChange {
                                    span: x.2.span,
                                    rule: x.1.clone(),
                                    database: Some(TableChangeDetail {
                                        target,
                                        shard_idx: w.1[0].unwrap(),
                                    }),
                                    table: None,
                                },
                            ))
                        }
                        StrategyTyp::Table => {
                            let actual_db = db_fn(&x.1, w.1[0].unwrap()).unwrap_or_else(|| "");
                            let target = self.change_table(x.2, actual_db, w.1[0].unwrap());
                            Some((
                                x.0,
                                TableChange {
                                    span: x.2.span,
                                    rule: x.1.clone(),
                                    table: Some(TableChangeDetail {
                                        target,
                                        shard_idx: w.1[0].unwrap(),
                                    }),
                                    database: None,
                                },
                            ))
                        }
                        StrategyTyp::DatabaseTable => {
                            let db_idx = w.1[0].unwrap();
                            let actual_db = db_fn(&x.1, db_idx).unwrap_or_else(|| "");
                            let _ = self.change_table(x.2, actual_db, w.1[0].unwrap());

                            let table = TableIdent {
                                span: x.2.span.clone(),
                                schema: Some(actual_db.to_string()),
                                name: x.2.name.clone(),
                            };

                            let table_idx = w.1[1].unwrap();
                            let target = self.change_table(&table, "", table_idx);
                            Some((
                                x.0,
                                TableChange {
                                    span: x.2.span,
                                    rule: x.1.clone(),
                                    database: Some(TableChangeDetail {
                                        target: target.clone(),
                                        shard_idx: db_idx,
                                    }),
                                    table: Some(TableChangeDetail { target, shard_idx: table_idx }),
                                },
                            ))
                        }
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    fn table_change_apply(
        target_sql: &mut String,
        strategy_typ: StrategyTyp,
        would_changes: &[(u8, TableChange)],
    ) -> Vec<(u8, RewriteChange)> {
        let mut offset = 0;

        would_changes
            .into_iter()
            .map(|x| {
                match strategy_typ {
                    StrategyTyp::Database => {
                        Self::change_sql(
                            target_sql,
                            x.1.span,
                            &x.1.database.as_ref().unwrap().target,
                            offset,
                        );
                        offset = x.1.database.as_ref().unwrap().target.len() - x.1.span.len();
                    }
                    StrategyTyp::Table => {
                        Self::change_sql(
                            target_sql,
                            x.1.span,
                            &x.1.table.as_ref().unwrap().target,
                            offset,
                        );
                        offset = x.1.table.as_ref().unwrap().target.len() - x.1.span.len();
                    }
                    StrategyTyp::DatabaseTable => {
                        Self::change_sql(
                            target_sql,
                            x.1.span,
                            &x.1.database.as_ref().unwrap().target,
                            offset,
                        );
                        offset = x.1.database.as_ref().unwrap().target.len() - x.1.span.len();
                    }
                };

                (x.0, RewriteChange::TableChange(x.1.clone()))
            })
            .collect::<Vec<_>>()
    }

    fn find_table_rule<'a>(
        &self,
        tables: &'a IndexMap<u8, Vec<TableIdent>>,
    ) -> Vec<(u8, Sharding, &'a TableIdent)> {
        let default_db = self.default_db.as_ref();
        let mut endpoints = self.endpoints.iter();

        Self::find_table(tables, |idx, meta| {
            let rule = self.rules.iter().find(|x| {
                let name = if meta.name.contains("`") {
                    meta.name.replace("`", "")
                } else {
                    meta.name.to_string()
                };
                x.table_name == name
            });
            if let Some(rule) = rule {
                let has_default_db = if let Some(default_db) = default_db {
                    if self.has_rw {
                        let node_group = self.node_group_config.as_ref().unwrap();
                        rule.actual_datanodes
                            .iter()
                            .find(|x| node_group.members.iter().find(|g| &g.name == *x).is_some())
                            .is_some()
                    } else {
                        rule.actual_datanodes
                            .iter()
                            .find(|x| {
                                endpoints
                                    .find(|ep| &ep.name == *x && default_db == &ep.db)
                                    .is_some()
                            })
                            .is_some()
                    }
                } else {
                    false
                };

                if meta.schema.is_some() || has_default_db {
                    (idx, Some(rule.clone()), true)
                } else {
                    (idx, None, false)
                }
            } else {
                (idx, None, false)
            }
        })
    }

    fn find_table<F>(
        tables: &IndexMap<u8, Vec<TableIdent>>,
        mut calc_fn: F,
    ) -> Vec<(u8, Sharding, &TableIdent)>
    where
        F: FnMut(u8, &TableIdent) -> (u8, Option<Sharding>, bool),
    {
        tables
            .iter()
            .filter_map(|(k, v)| {
                let res = v
                    .iter()
                    .filter_map(|x| {
                        let res = calc_fn(*k, x);
                        if res.2 {
                            Some((res.0, res.1.unwrap(), x))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                if res.is_empty() {
                    None
                } else {
                    Some(res)
                }
            })
            .flatten()
            .collect::<Vec<_>>()
    }

    fn find_try_where<'a>(
        strategy_typ: StrategyTyp,
        try_tables: &[(u8, Sharding, &TableIdent)],
        wheres: &'a IndexMap<u8, Vec<WhereMeta>>,
    ) -> Result<Vec<(u8, Vec<Option<u64>>)>, ShardingRewriteError> {
        let mut res = vec![];

        match strategy_typ {
            StrategyTyp::Database => {
                for (query_id, metas) in wheres.iter() {
                    let rule = try_tables.iter().find(|x| x.0 == *query_id).unwrap();

                    let sharding_column = rule.1.get_sharding_column().0;
                    let sharding_count = rule.1.get_sharding_count().0;
                    let algo = rule.1.get_algo().0;

                    let db_right = Self::get_where_match_right(metas, sharding_column.unwrap());

                    if let Some(right) = db_right {
                        let idx = Self::parse_where(right, sharding_count.unwrap(), algo.unwrap())?;
                        res.push((*query_id, vec![idx]));
                    }
                }
            }

            StrategyTyp::Table => {
                for (query_id, metas) in wheres.iter() {
                    let rule = try_tables.iter().find(|x| x.0 == *query_id).unwrap();

                    let sharding_column = rule.1.get_sharding_column().1;
                    let sharding_count = rule.1.get_sharding_count().1;
                    let algo = rule.1.get_algo().1;

                    let table_right = Self::get_where_match_right(metas, sharding_column.unwrap());

                    if let Some(right) = table_right {
                        let idx = Self::parse_where(right, sharding_count.unwrap(), algo.unwrap())?;
                        res.push((*query_id, vec![idx]));
                    }
                }
            }

            StrategyTyp::DatabaseTable => {
                for (query_id, metas) in wheres.iter() {
                    let mut shards = vec![None, None];
                    let rule = try_tables.iter().find(|x| x.0 == *query_id).unwrap();

                    let sharding_column = rule.1.get_sharding_column();
                    let sharding_count = rule.1.get_sharding_count();
                    let algo = rule.1.get_algo();

                    let db_right = Self::get_where_match_right(metas, sharding_column.0.unwrap());
                    let table_right =
                        Self::get_where_match_right(metas, sharding_column.1.unwrap());

                    if let Some(right) = db_right {
                        let idx =
                            Self::parse_where(right, sharding_count.0.unwrap(), algo.0.unwrap())?;
                        shards[0] = idx;
                    }

                    if let Some(right) = table_right {
                        let idx =
                            Self::parse_where(right, sharding_count.1.unwrap(), algo.1.unwrap())?;
                        shards[1] = idx;
                    }

                    res.push((*query_id, shards));
                }
            }
        };

        Ok(res)
    }

    fn get_where_match_right<'b>(
        metas: &'b [WhereMeta],
        sharding_column: &'b str,
    ) -> Option<&'b WhereMetaRightDataType> {
        metas.iter().find_map(|x| {
            let WhereMeta::BinaryExpr { left, right } = x;
            let left = left.replace("`", "");
            left.eq(sharding_column).then(|| right)
        })
    }

    fn parse_where(
        right: &WhereMetaRightDataType,
        sharding_count: u32,
        sharding_algo: &ShardingAlgorithmName,
    ) -> Result<Option<u64>, ShardingRewriteError> {
        let num = match right {
            WhereMetaRightDataType::Num(val) => {
                let val = val.parse::<u64>()?;
                val.calc(sharding_algo, sharding_count as u64)
            }

            WhereMetaRightDataType::SignedNum(val) => {
                let val = val.parse::<i64>()?;
                val.calc(sharding_algo, sharding_count as i64)
            }

            WhereMetaRightDataType::FloatNum(val) => {
                let val = val.parse::<f64>()?;
                val.calc(sharding_algo, sharding_count as f64)
            }
            _ => return Ok(None),
        };

        if let Some(num) = num {
            return Ok(Some(num));
        }

        Ok(None)
    }

    fn change_order_group(&self, target_sql: &mut String, changes: &mut Vec<RewriteChange>, orders: &IndexMap<u8, Vec<OrderMeta>>, groups: &IndexMap<u8, Vec<GroupMeta>>, fields: &IndexMap<u8, Vec<FieldMeta>>,
        idx: u64,
    ) -> Result<usize, ()> {
        // Currently, we don't consider the case that query_id not equal 1.
        let fields = fields.get(&1).ok_or_else(|| ())?;

        let last_span = match fields.last() {
            Some(FieldMeta::Ident(field)) => field.span,
            _ => return Ok(0),
        };
        
        let default_orders = Vec::<OrderMeta>::new();
        let orders = orders.get(&1).map_or_else(|| &default_orders, |v| v);
        // Get the order field that does not exist in the fields.
        let not_exist_order_fields = orders.iter().filter_map(|x| {
            let exist_field = fields.iter().find(|field| {
                if let FieldMeta::Ident(field_ident) = field {
                    if field_ident.name.replace("`", "") == x.name.replace("`", "") {
                        return true;
                    }
                }
                false
            });

            exist_field.is_none().then(|| x)
        }).collect::<Vec<_>>();

        let default_groups = Vec::<GroupMeta>::new();
        let groups = groups.get(&1).map_or_else(|| &default_groups, |v| v);

        // Get the group field that does not exist in the fields.
        let not_exist_group_fields = groups.iter().filter_map(|x| {
            let exist_field = fields.iter().find(|field| {
                if let FieldMeta::Ident(field_ident) = field {
                    if field_ident.name.replace("`", "") == x.name.replace("`", "") {
                        return true;
                    }
                }
                false
            });

            exist_field.is_none().then(|| x)
        }).collect::<Vec<_>>();

        let mut target_fields = vec![];

        for field in not_exist_order_fields {
            let order_as_name = field.name.replace("`", "");
            let target_field = format!(
                "{} {} {}_{}_{:05}",
                field.name,
                AS,
                order_as_name.to_ascii_uppercase(),
                ORDER_BY_DERIVED,
                idx
            );
            target_fields.push(target_field.clone());   

            let mut change_target = IndexMap::new();
            change_target.insert(ORDER_TARGET.to_string(), target_field);
            change_target.insert(ORDER_FIELD.to_string(), field.name.clone());
            let order_change = OrderChange {
                target: change_target,
                direction: field.direction.clone(),
            };
            changes.push(RewriteChange::OrderChange(order_change));
        }

        for field in not_exist_group_fields {
            let group_as_name = field.name.replace("`", "");
            let target_field = format!(
                "{} {} {}_{}_{:05}",
                field.name,
                AS,
                group_as_name.to_ascii_uppercase(),
                GROUP_BY_DERIVED,
                idx
            );
            target_fields.push(target_field.clone());
            
            let mut change_target = IndexMap::new();
            change_target.insert(GROUP_TARGET.to_string(), target_field);
            change_target.insert(GROUP_FIELD.to_string(), field.name.clone());
            let group_change = GroupChange {
                target: change_target,
            };
            changes.push(RewriteChange::GroupChange(group_change));
        }

        if target_fields.is_empty() {
            return Ok(0);
        }

        let mut target_fields_string = target_fields.join(", ");
        target_fields_string.insert_str(0, ", ");
        target_sql.insert_str(last_span.end(), &target_fields_string);

        return Ok(target_fields_string.len());
    }

    fn change_avg(
        &self,
        target_sql: &mut String,
        changes: &mut Vec<RewriteChange>,
        avgs: &IndexMap<u8, Vec<AvgMeta>>,
        idx: u64,
        offset: usize,
    ) {
        if avgs.is_empty() {
            return;
        }

        let mut res = IndexMap::new();
        let mut target = String::with_capacity(AVG_DERIVED_COUNT.len() + AVG_DERIVED_SUM.len());

        for (_, avg) in avgs.iter() {
            let last_span = avg.last().unwrap().span;
            let first_span = avg.first().unwrap().span;
            let len = last_span.start() + last_span.len() - first_span.start();

            for _ in 0..len {
                target_sql.remove(first_span.start() + offset);
            }

            for avg_meta in avg {
                res.insert(AVG_FIELD.to_string(), avg_meta.avg_field_name.clone());
                let target_count = &format!("{}({}) {} ", COUNT, avg_meta.field_name, AS);
                let target_count_as = &format!(
                    "{}_{}_{:05}",
                    avg_meta.field_name.to_ascii_uppercase(),
                    AVG_DERIVED_COUNT,
                    idx
                );
                res.insert(AVG_COUNT.to_string(), target_count_as.to_string());

                let target_sum = &format!("{}({}) {} ", SUM, avg_meta.field_name, AS);
                let target_sum_as = &format!(
                    "{}_{}_{:05}",
                    avg_meta.field_name.to_ascii_uppercase(),
                    AVG_DERIVED_SUM,
                    idx
                );
                res.insert(AVG_SUM.to_string(), target_sum_as.to_string());

                target += &format!(
                    "{}{}, {}{}",
                    target_count, target_count_as, target_sum, target_sum_as
                );
                if avg_meta.span != last_span {
                    target.push(',');
                    target.push(' ');
                }
            }

            target_sql.insert_str(first_span.start() + offset, &target);
        }
        changes.push(RewriteChange::AvgChange(AvgChange { target: res }));
    }

    fn change_insert_sql(
        &self,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
        inserts: &IndexMap<u8, Vec<InsertValsMeta>>,
    ) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        let outputs = try_tables
            .into_iter()
            .map(|(query_id, rule, table)| {
                let meta_base_info = if rule.table_strategy.is_some() {
                    ShardingMetaBaseInfo {
                        column: (rule.get_sharding_column().1, None),
                        count: (rule.get_sharding_count().1, None),
                        algo: (rule.get_algo().1, None),
                    }
                } else if rule.database_strategy.is_some() {
                    ShardingMetaBaseInfo {
                        column: (rule.get_sharding_column().0, None),
                        count: (rule.get_sharding_count().0, None),
                        algo: (rule.get_algo().0, None),
                    }
                } else {
                    ShardingMetaBaseInfo {
                        column: rule.get_sharding_column(),
                        count: rule.get_sharding_count(),
                        algo: rule.get_algo(),
                    }
                };

                self.change_insert_sql_inner(
                    &rule,
                    &table,
                    &inserts.get(&query_id).unwrap(),
                    &fields.get(&query_id).unwrap(),
                    meta_base_info,
                    Self::get_count_field(fields),
                )
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        Ok(outputs)
    }

    fn change_insert_sql_inner<'b>(
        &self,
        rule: &Sharding,
        table: &TableIdent,
        inserts: &[InsertValsMeta],
        fields: &[FieldMeta],
        meta_base_info: ShardingMetaBaseInfo<'b>,
        count_field: Option<FieldMetaIdent>,
    ) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        let strategy_typ = rule.get_strategy_typ();
        let changes = Self::change_insert(&strategy_typ, inserts, fields, meta_base_info)?;
        let row_start_idx = changes[0].span.start();
        let sql_prefix_text = &self.raw_sql[0..row_start_idx];

        let mut sqls = IndexMap::<(Option<u64>, Option<u64>), String>::new();
        match strategy_typ {
            StrategyTyp::Database => {
                for change in changes.iter() {
                    let db_idx = change.database.unwrap();
                    let actual_db = rule
                        .get_actual_schema(&self.endpoints, Some(db_idx as usize))
                        .unwrap_or_else(|| "");
                    let target = self.change_table(table, actual_db, db_idx);
                    let mut target_sql_prefix_text = sql_prefix_text.to_string();
                    let row_value_text = self.get_change_insert_row(
                        &mut target_sql_prefix_text,
                        &target,
                        table.span,
                        change,
                    );
                    sqls.entry((Some(db_idx), None))
                        .or_insert(target_sql_prefix_text)
                        .push_str(&row_value_text);
                }
            }

            StrategyTyp::Table => {
                for change in changes.iter() {
                    let db_idx = change.table.unwrap();
                    let target = self.change_table(table, "", db_idx);
                    let mut target_sql_prefix_text = sql_prefix_text.to_string();
                    let row_value_text = self.get_change_insert_row(
                        &mut target_sql_prefix_text,
                        &target,
                        table.span,
                        change,
                    );
                    sqls.entry((Some(db_idx), None))
                        .or_insert(target_sql_prefix_text)
                        .push_str(&row_value_text);
                }
            }

            StrategyTyp::DatabaseTable => {
                for change in changes.iter() {
                    let db_idx = change.database.unwrap();
                    let actual_db = rule
                        .get_actual_schema(&self.endpoints, Some(db_idx as usize))
                        .unwrap_or_else(|| "");
                    let _ = self.change_table(table, actual_db, db_idx);
                    let table = TableIdent {
                        span: table.span.clone(),
                        schema: Some(actual_db.to_string()),
                        name: table.name.clone(),
                    };

                    let table_idx = change.table.unwrap();
                    let target = self.change_table(&table, "", table_idx);

                    let mut target_sql_prefix_text = sql_prefix_text.to_string();
                    let row_value_text = self.get_change_insert_row(
                        &mut target_sql_prefix_text,
                        &target,
                        table.span,
                        change,
                    );
                    sqls.entry((Some(db_idx), Some(table_idx)))
                        .or_insert(target_sql_prefix_text)
                        .push_str(&row_value_text);
                }
            }
        }
        let outputs = sqls
            .into_iter()
            .map(|((idx, _), v)| -> Result<ShardingRewriteOutput, ShardingRewriteError> {
                let data_source = self.gen_data_source(rule, idx.unwrap() as usize)?;
                let sharding_column = rule.get_sharding_column().0.map(|x| x.to_string());

                Ok(ShardingRewriteOutput {
                    changes: vec![],
                    target_sql: v.trim_end_matches(", ").to_string(),
                    data_source: data_source.clone(),
                    sharding_column,
                    min_max_fields: vec![],
                    count_field: count_field.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(outputs)
    }

    fn get_change_insert_row(
        &self,
        prefix_text: &mut String,
        target: &str,
        span: mysql_parser::Span,
        sharding_idx: &ShardingIdx,
    ) -> String {
        Self::change_sql(prefix_text, span, target, 0);
        let mut row_value_text =
            self.raw_sql[sharding_idx.span.start()..sharding_idx.span.end()].to_string();
        row_value_text.push_str(", ");
        row_value_text
    }

    fn change_insert<'b>(
        strategy_typ: &StrategyTyp,
        inserts: &[InsertValsMeta],
        fields: &[FieldMeta],
        meta_base_info: ShardingMetaBaseInfo<'b>,
    ) -> Result<Vec<ShardingIdx>, ShardingRewriteError> {
        let insert_values = Self::find_inserts(&strategy_typ, inserts, fields, &meta_base_info)?;
        let mut changes = vec![];

        match strategy_typ {
            StrategyTyp::Database => {
                for value in insert_values.iter() {
                    let idx = Self::calc_database_or_table_sharding_idx(
                        value.sharding_value.database.as_ref(),
                        &meta_base_info,
                    )?;
                    changes.push(ShardingIdx {
                        database: Some(idx),
                        table: None,
                        span: value.value_span,
                    })
                }
            }

            StrategyTyp::Table => {
                for value in insert_values.iter() {
                    let idx = Self::calc_database_or_table_sharding_idx(
                        value.sharding_value.table.as_ref(),
                        &meta_base_info,
                    )?;

                    changes.push(ShardingIdx {
                        database: None,
                        table: Some(idx),
                        span: value.value_span,
                    })
                }
            }

            StrategyTyp::DatabaseTable => {
                for value in insert_values.iter() {
                    let (db_idx, table_idx) = Self::calc_database_and_table_sharding_idx(
                        value.sharding_value.database.as_ref(),
                        value.sharding_value.table.as_ref(),
                        &meta_base_info,
                    )?;

                    changes.push(ShardingIdx {
                        database: Some(db_idx),
                        table: Some(table_idx),
                        span: value.value_span,
                    })
                }
            }
        };

        Ok(changes)
    }

    fn calc_database_or_table_sharding_idx<'b>(
        value: Option<&String>,
        meta_base_info: &'b ShardingMetaBaseInfo<'b>,
    ) -> Result<u64, ShardingRewriteError> {
        let algo = meta_base_info.algo.0.unwrap();
        let sharding_count = meta_base_info.count.0.unwrap() as u64;
        let value = value
            .ok_or_else(|| ShardingRewriteError::CalcShardingIdxError)?
            .parse::<u64>()
            .map_err(ShardingRewriteError::from)?;
        value.calc(algo, sharding_count).ok_or_else(|| ShardingRewriteError::CalcShardingIdxError)
    }

    fn calc_database_and_table_sharding_idx<'b>(
        db_value: Option<&String>,
        table_value: Option<&String>,
        meta_base_info: &ShardingMetaBaseInfo<'b>,
    ) -> Result<(u64, u64), ShardingRewriteError> {
        let database_sharding_algo = meta_base_info.algo.0.unwrap();
        let table_sharding_algo = meta_base_info.algo.1.unwrap();
        let database_sharding_count = meta_base_info.count.0.unwrap() as u64;
        let table_sharding_count = meta_base_info.count.1.unwrap() as u64;
        
        let db_value = db_value
            .ok_or_else(|| ShardingRewriteError::CalcShardingIdxError)?
            .parse::<u64>()
            .map_err(ShardingRewriteError::from)?;
        let db_idx = db_value
            .calc(database_sharding_algo, database_sharding_count)
            .ok_or_else(|| ShardingRewriteError::CalcShardingIdxError)?;

        let table_value = table_value
            .ok_or_else(|| ShardingRewriteError::CalcShardingIdxError)?
            .parse::<u64>()
            .map_err(ShardingRewriteError::from)?;
        let table_idx = table_value
            .calc(table_sharding_algo, table_sharding_count)
            .ok_or_else(|| ShardingRewriteError::CalcShardingIdxError)?;

        Ok((db_idx, table_idx))
    }

    fn find_inserts<'b>(
        strategy_typ: &StrategyTyp,
        inserts: &[InsertValsMeta],
        fields: &[FieldMeta],
        meta_base_info: &ShardingMetaBaseInfo<'b>,
    ) -> Result<Vec<ChangeInsertMeta>, ShardingRewriteError> {
        let (db_idx, table_idx) = match strategy_typ {
            StrategyTyp::Database => {
                let idx = Self::find_insert_field_idx(fields, |idx, field| {
                    (field.name == meta_base_info.column.0.unwrap()).then(|| idx)
                })?;
                (Some(idx), None)
            }
            StrategyTyp::Table => {
                let idx = Self::find_insert_field_idx(fields, |idx, field| {
                    (field.name == meta_base_info.column.0.unwrap()).then(|| idx)
                })?;
                (None, Some(idx))
            }
            StrategyTyp::DatabaseTable => {
                let db_idx = Self::find_insert_field_idx(fields, |idx, field| {
                    (field.name == meta_base_info.column.0.unwrap()).then(|| idx)
                })?;

                let table_idx = Self::find_insert_field_idx(fields, |idx, field| {
                    (field.name == meta_base_info.column.1.unwrap()).then(|| idx)
                })?;
                (Some(db_idx), Some(table_idx))
            }
        };

        if db_idx.is_some() && table_idx.is_some() {
            let changes = inserts
                .iter()
                .map(|x| ChangeInsertMeta {
                    sharding_value: InsertShardingValue {
                        database: Some(x.values[db_idx.unwrap()].value.clone()),
                        table: Some(x.values[table_idx.unwrap()].value.clone()),
                    },
                    value_span: x.span,
                })
                .collect::<Vec<_>>();

            return Ok(changes);
        }

        if let Some(db_idx) = db_idx {
            let db_changes = inserts
                .iter()
                .map(|x| ChangeInsertMeta {
                    sharding_value: InsertShardingValue {
                        database: Some(x.values[db_idx].value.clone()),
                        table: None,
                    },
                    value_span: x.span,
                })
                .collect::<Vec<_>>();

            return Ok(db_changes);
        }

        if let Some(table_idx) = table_idx {
            let table_changes = inserts
                .iter()
                .map(|x| ChangeInsertMeta {
                    sharding_value: InsertShardingValue {
                        database: None,
                        table: Some(x.values[table_idx].value.clone()),
                    },
                    value_span: x.span,
                })
                .collect::<Vec<_>>();

            return Ok(table_changes);
        }

        Ok(vec![])
    }

    fn find_insert_field_idx<F>(
        fields: &[FieldMeta],
        find_f: F,
    ) -> Result<usize, ShardingRewriteError>
    where
        F: Fn(usize, &FieldMetaIdent) -> Option<usize>,
    {
        fields
            .iter()
            .enumerate()
            .find_map(|x| if let FieldMeta::Ident(field) = x.1 { find_f(x.0, field) } else { None })
            .ok_or_else(|| ShardingRewriteError::ShardingColumnNotFound)
    }

    fn database_table_strategy_iproduct(
        &self,
        part: DatabaseTableStrategyPart,
        tables: Vec<(u8, Sharding, &TableIdent)>,
        avgs: &IndexMap<u8, Vec<AvgMeta>>,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
        orders: &IndexMap<u8, Vec<OrderMeta>>,
        groups: &IndexMap<u8, Vec<GroupMeta>>,
    ) -> Vec<ShardingRewriteOutput> {
        let mut outputs = vec![];

        for t in tables.iter() {
            let db_sharding_column = t.1.get_sharding_column().0.map(|x| x.to_string());
            let sharding_count = t.1.get_sharding_count().1.unwrap() as u64;

            let actual_nodes = if let DatabaseTableStrategyPart::Database(idx) = part {
                vec![t.1.actual_datanodes[idx].clone()]
            } else {
                t.1.actual_datanodes.clone()
            };

            for (idx, node) in actual_nodes.iter().enumerate() {
                let ep = self.endpoints.iter().find(|e| e.name.eq(node)).unwrap().clone();
                let _ = self.change_table(t.2, &ep.db, 0);
                let table =
                    TableIdent { span: t.2.span, schema: Some(ep.db), name: t.2.name.clone() };
                let mut group_changes = IndexMap::<usize, (u8, Vec<TableChange>)>::new();
                let data_source = self.gen_data_source(&t.1, idx).unwrap();

                if let DatabaseTableStrategyPart::Table(idx) = part {
                    let target = self.change_table(&table, "", idx as u64);
                    let change = TableChange {
                        span: t.2.span,
                        table: Some(TableChangeDetail { target, shard_idx: idx as u64 }),
                        database: None,
                        rule: t.1.clone(),
                    };
                    group_changes.entry(idx as usize).or_insert((t.0, vec![])).1.push(change);
                } else {
                    for table_idx in 0..sharding_count {
                        let target = self.change_table(&table, "", table_idx);
                        let change = TableChange {
                            span: t.2.span,
                            table: Some(TableChangeDetail { target, shard_idx: table_idx as u64 }),
                            database: None,
                            rule: t.1.clone(),
                        };
                        group_changes
                            .entry(table_idx as usize)
                            .or_insert((t.0, vec![]))
                            .1
                            .push(change);
                    }
                }

                for (_group, changes) in group_changes.into_iter() {
                    let mut offset = 0;
                    let mut target_sql = self.raw_sql.to_string();

                    for change in changes.1.iter() {
                        Self::change_sql(
                            &mut target_sql,
                            change.span,
                            &change.table.as_ref().unwrap().target,
                            offset,
                        );
                        offset = change.table.as_ref().unwrap().target.len() - change.span.len();
                    }

                    let min_max_fields = self.find_min_max_fields(&changes.0, fields);
                    let mut rewrite_changes: Vec<RewriteChange> =
                        changes.1.iter().map(|x| RewriteChange::TableChange(x.clone())).collect();

                    let _ = self.change_order_group(
                        &mut target_sql,
                        &mut rewrite_changes,
                        orders,
                        groups,
                        fields,
                        changes.1[0].table.as_ref().unwrap().shard_idx,
                    );

                    if !avgs.is_empty() {
                        if changes.1[0].span.start() > avgs[0][0].span.start() {
                            offset = 0;
                        }
                        self.change_avg(
                            &mut target_sql,
                            &mut rewrite_changes,
                            avgs,
                            changes.1[0].table.as_ref().unwrap().shard_idx,
                            offset,
                        );
                    }

                    outputs.push(ShardingRewriteOutput {
                        changes: rewrite_changes,
                        target_sql,
                        data_source: data_source.clone(),
                        sharding_column: db_sharding_column.clone(),
                        min_max_fields,
                        count_field: Self::get_count_field(fields),
                    })
                }
            }
        }

        outputs
    }

    fn database_strategy_iproduct(
        &self,
        tables: Vec<(u8, Sharding, &TableIdent)>,
        avgs: &IndexMap<u8, Vec<AvgMeta>>,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
        orders: &IndexMap<u8, Vec<OrderMeta>>,
        groups: &IndexMap<u8, Vec<GroupMeta>>,
    ) -> Vec<ShardingRewriteOutput> {
        let mut output = vec![];

        let mut sharding_column = None;
        let mut group_changes = IndexMap::<usize, (u8, Vec<TableChange>)>::new();

        for t in tables.iter() {
            sharding_column = Some(t.1.get_sharding_column().0.unwrap().to_string());

            for (idx, node) in t.1.actual_datanodes.iter().enumerate() {
                let ep = self.endpoints.iter().find(|e| e.name.eq(node)).unwrap().clone();
                let target = self.change_table(t.2, &ep.db, 0);

                let change = TableChange {
                    span: t.2.span,
                    database: Some(TableChangeDetail { target, shard_idx: idx as u64 }),
                    table: None,
                    rule: t.1.clone(),
                };

                group_changes.entry(idx).or_insert((t.0, vec![])).1.push(change);
            }
        }

        for (group, changes) in group_changes.into_iter() {
            let mut offset = 0;
            let mut target_sql = self.raw_sql.to_string();
            let ep = self.endpoints[group].clone();

            for change in changes.1.iter() {
                Self::change_sql(
                    &mut target_sql,
                    change.span,
                    &change.database.as_ref().unwrap().target,
                    offset,
                );
                offset = change.database.as_ref().unwrap().target.len() - change.span.len();
            }

            let min_max_fields = self.find_min_max_fields(&changes.0, fields);
            let mut rewrite_changes: Vec<RewriteChange> =
                changes.1.iter().map(|x| RewriteChange::TableChange(x.clone())).collect();

            let _ = self.change_order_group(
                &mut target_sql,
                &mut rewrite_changes,
                orders,
                groups,
                fields,
                changes.1[0].database.as_ref().unwrap().shard_idx,
            );

            if !avgs.is_empty() {
                if changes.1[0].span.start() > avgs[0][0].span.start() {
                    offset = 0;
                }
                self.change_avg(
                    &mut target_sql,
                    &mut rewrite_changes,
                    avgs,
                    changes.1[0].database.as_ref().unwrap().shard_idx,
                    offset,
                );
            }

            output.push(ShardingRewriteOutput {
                changes: rewrite_changes,
                target_sql,
                data_source: DataSource::Endpoint(ep),
                sharding_column: sharding_column.clone(),
                min_max_fields,
                count_field: Self::get_count_field(fields),
            })
        }
        output
    }

    fn table_strategy_iproduct(
        &self,
        tables: Vec<(u8, Sharding, &TableIdent)>,
        avgs: &IndexMap<u8, Vec<AvgMeta>>,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
        orders: &IndexMap<u8, Vec<OrderMeta>>,
        groups: &IndexMap<u8, Vec<GroupMeta>>,
    ) -> Vec<ShardingRewriteOutput> {
        let mut output = vec![];
        let mut group_changes = IndexMap::<usize, (u8, Vec<TableChange>)>::new();
        let mut sharding_column = None;
        let data_source = self.gen_data_source(&tables[0].1, 0).unwrap();

        for t in tables.iter() {
            sharding_column = Some(t.1.get_sharding_column().1.unwrap().to_string());
            let sharding_count = t.1.get_sharding_count().1.unwrap();
            for idx in 0..sharding_count as u64 {
                let target = self.change_table(t.2, "", idx);

                let change = TableChange {
                    span: t.2.span,
                    table: Some(TableChangeDetail { target, shard_idx: idx as u64 }),
                    database: None,
                    rule: t.1.clone(),
                };

                group_changes.entry(idx as usize).or_insert((t.0, vec![])).1.push(change);
            }
        }

        for (_, changes) in group_changes.into_iter() {
            let mut offset = 0;
            let mut target_sql = self.raw_sql.clone();

            for change in changes.1.iter() {
                Self::change_sql(
                    &mut target_sql,
                    change.span,
                    &change.table.as_ref().unwrap().target,
                    offset,
                );
                offset = change.table.as_ref().unwrap().target.len() - change.span.len();
            }

            let min_max_fields = self.find_min_max_fields(&changes.0, fields);

            let mut rewrite_changes: Vec<RewriteChange> =
                changes.1.iter().map(|x| RewriteChange::TableChange(x.clone())).collect();
            let _ = self.change_order_group(
                &mut target_sql,
                &mut rewrite_changes,
                orders,
                groups,
                fields,
                changes.1[0].table.as_ref().unwrap().shard_idx,
            );

            if !avgs.is_empty() {
                if changes.1[0].span.start() > avgs[0][0].span.start() {
                    offset = 0;
                }
                self.change_avg(
                    &mut target_sql,
                    &mut rewrite_changes,
                    avgs,
                    changes.1[0].table.as_ref().unwrap().shard_idx,
                    offset,
                );
            }

            output.push(ShardingRewriteOutput {
                changes: rewrite_changes,
                target_sql: target_sql.to_string(),
                data_source: data_source.clone(),
                sharding_column: sharding_column.clone(),
                min_max_fields,
                count_field: Self::get_count_field(fields),
            })
        }

        output
    }

    fn change_table(&self, table: &TableIdent, actual_db: &str, table_idx: u64) -> String {
        let db = table.schema.as_ref().unwrap_or_else(|| self.default_db.as_ref().unwrap());

        let mut target = String::with_capacity(db.len());
        if actual_db.is_empty() {
            target.push('`');
            target.push_str(db);
            target.push('`');
            target.push('.');
            if table.name.contains("`") {
                target.push('`');
                let name = table.name.replace("`", "");
                target.push_str(&format!("{}_{:05}", &name, table_idx));
                target.push('`');
            } else {
                target.push_str(&format!("{}_{:05}", &table.name, table_idx));
            }
        } else {
            if db.contains("`") {
                target.push('`');
                target.push_str(actual_db);
                target.push('`');
            } else {
                target.push_str(actual_db);
            }

            target.push_str(".");
            target.push_str(&table.name);
        }
        target
    }

    fn change_sql(target_sql: &mut String, span: mysql_parser::Span, target: &str, offset: usize) {
        for _ in 0..span.len() {
            target_sql.remove(span.start() + offset);
        }
        target_sql.insert_str(span.start() + offset, target);
    }

    fn get_meta(&self, ast: &mut SqlStmt) -> RewriteMetaData {
        let mut meta = RewriteMetaData::new(self.raw_sql.clone());
        let _ = ast.visit(&mut meta);
        meta
    }

    fn find_min_max_fields(
        &self,
        idx: &u8,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
    ) -> Vec<FieldMetaIdent> {
        match fields.get(idx) {
            Some(fields) => {
                let mut min_max_fields = vec![];
                for f in fields.into_iter() {
                    match f {
                        FieldMeta::Ident(field) => match field.wrap_func {
                            FieldWrapFunc::Min | FieldWrapFunc::Max => {
                                min_max_fields.push(FieldMetaIdent {
                                    span: field.span,
                                    name: field.name.clone(),
                                    wrap_func: field.wrap_func.clone(),
                                })
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
                min_max_fields
            }

            None => vec![],
        }
    }

    fn get_count_field(fields: &IndexMap<u8, Vec<FieldMeta>>) -> Option<FieldMetaIdent> {
        fields.values().find_map(|f| {
            f.iter().find_map(|x| {
                if let FieldMeta::Ident(meta) = x {
                    if meta.wrap_func == FieldWrapFunc::Count {
                        return Some(meta.clone());
                    }
                }
                None
            })
        })
    }

    fn gen_data_source(
        &self,
        rule: &Sharding,
        shard_idx: usize,
    ) -> Result<DataSource, ShardingRewriteError> {
        if self.has_rw {
            Ok(DataSource::NodeGroup(rule.actual_datanodes[0].clone()))
        } else {
            let ep = rule
                .get_endpoint(&self.endpoints, Some(shard_idx))
                .ok_or_else(|| ShardingRewriteError::EndpointNotFound)?;
            Ok(DataSource::Endpoint(ep.clone()))
        }
    }
}

impl ShardingRewriter<ShardingRewriteInput> for ShardingRewrite {
    type Output = Result<Vec<ShardingRewriteOutput>, ShardingRewriteError>;
    fn rewrite(&mut self, mut input: ShardingRewriteInput) -> Self::Output {
        self.set_raw_sql(input.raw_sql);
        self.set_default_db(input.default_db);

        let meta = self.get_meta(&mut input.ast);
        let tables = meta.get_tables().clone();
        let try_tables = self.find_table_rule(&tables);

        if try_tables.is_empty() {
            return Ok(vec![]);
        }

        // Strategy according to first element of `try_tables`.
        let rule = &try_tables[0].1;

        if rule.database_strategy.is_some() {
            return self.database_strategy(meta, try_tables);
        } else if rule.table_strategy.is_some() {
            return self.table_strategy(meta, try_tables);
        } else if rule.database_table_strategy.is_some() {
            return self.database_table_strategy(meta, try_tables);
        }

        return Ok(vec![]);
    }
}

#[cfg(test)]
mod test {
    use endpoint::endpoint::Endpoint;
    use mysql_parser::parser::Parser;

    use super::ShardingRewrite;
    use crate::{
        config::{
            DatabaseStrategyConfig, DatabaseTableStrategyConfig, Sharding, ShardingAlgorithmName,
            StrategyType,
        },
        rewrite::{ShardingRewriteInput, ShardingRewriter},
        sharding_rewrite::DataSource,
    };

    fn get_database_table_sharding_config() -> (Vec<Sharding>, Vec<Endpoint>) {
        (
            vec![Sharding {
                table_name: "tshard".to_string(),
                actual_datanodes: vec!["ds0".to_string(), "ds1".to_string()],
                binding_tables: None,
                broadcast_tables: None,
                database_strategy: None,
                table_strategy: None,
                database_table_strategy: Some(StrategyType::DatabaseTableStrategyConfig(
                    DatabaseTableStrategyConfig {
                        database_sharding_algorithm_name: ShardingAlgorithmName::Mod,
                        database_sharding_column: "didx".to_string(),
                        table_sharding_algorithm_name: ShardingAlgorithmName::Mod,
                        table_sharding_column: "idx".to_string(),
                        shading_count: 4,
                    },
                )),
            }],
            vec![
                Endpoint {
                    weight: 1,
                    name: String::from("ds0"),
                    db: String::from("db0"),
                    user: String::from("user"),
                    password: String::from("password"),
                    addr: String::from("127.0.0.1"),
                },
                Endpoint {
                    weight: 1,
                    name: String::from("ds1"),
                    db: String::from("db1"),
                    user: String::from("user"),
                    password: String::from("password"),
                    addr: String::from("127.0.0.2"),
                },
            ],
        )
    }

    fn get_database_sharding_config() -> (Vec<Sharding>, Vec<Endpoint>) {
        (
            vec![Sharding {
                table_name: "tshard".to_string(),
                actual_datanodes: vec!["ds0".to_string(), "ds1".to_string()],
                binding_tables: None,
                broadcast_tables: None,
                database_strategy: Some(StrategyType::DatabaseStrategyConfig(
                    DatabaseStrategyConfig {
                        database_sharding_algorithm_name: ShardingAlgorithmName::Mod,
                        database_sharding_column: "idx".to_string(),
                    },
                )),
                table_strategy: None,
                database_table_strategy: None,
            }],
            vec![
                Endpoint {
                    weight: 1,
                    name: String::from("ds0"),
                    db: String::from("db0"),
                    user: String::from("user"),
                    password: String::from("password"),
                    addr: String::from("127.0.0.1"),
                },
                Endpoint {
                    weight: 1,
                    name: String::from("ds1"),
                    db: String::from("db1"),
                    user: String::from("user"),
                    password: String::from("password"),
                    addr: String::from("127.0.0.2"),
                },
            ],
        )
    }

    fn get_table_sharding_config() -> (Vec<Sharding>, Vec<Endpoint>) {
        (
            vec![Sharding {
                table_name: "tshard".to_string(),
                actual_datanodes: vec!["ds001".to_string()],
                binding_tables: None,
                broadcast_tables: None,
                table_strategy: Some(StrategyType::TableStrategyConfig(
                    crate::config::TableStrategyConfig {
                        table_sharding_algorithm_name: ShardingAlgorithmName::Mod,
                        table_sharding_column: "idx".to_string(),
                        sharding_count: 4,
                    },
                )),
                database_strategy: None,
                database_table_strategy: None,
            }],
            vec![Endpoint {
                weight: 1,
                name: String::from("ds001"),
                db: String::from("db"),
                user: String::from("user"),
                password: String::from("password"),
                addr: String::from("127.0.0.1:3306"),
            }],
        )
    }

    #[test]
    fn test_database_table_sharding_strategy() {
        let config = get_database_table_sharding_config();
        let raw_sql = "SELECT * FROM db.tshard where idx > 3";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: Some("db".to_string()),
        };
        let res = sr.rewrite(input).unwrap();
        let sqls = res.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();
        assert_eq!(sqls[0], "SELECT * FROM `db0`.tshard_00000 where idx > 3");
        assert_eq!(sqls[1], "SELECT * FROM `db0`.tshard_00001 where idx > 3");
        assert_eq!(sqls[2], "SELECT * FROM `db0`.tshard_00002 where idx > 3");
        assert_eq!(sqls[3], "SELECT * FROM `db0`.tshard_00003 where idx > 3");
        assert_eq!(sqls[4], "SELECT * FROM `db1`.tshard_00000 where idx > 3");
        assert_eq!(sqls[5], "SELECT * FROM `db1`.tshard_00001 where idx > 3");
        assert_eq!(sqls[6], "SELECT * FROM `db1`.tshard_00002 where idx > 3");
        assert_eq!(sqls[7], "SELECT * FROM `db1`.tshard_00003 where idx > 3");
    }

    #[test]
    fn test_database_table_sharding_strategy_where() {
        let config = get_database_table_sharding_config();
        let raw_sql = "SELECT * FROM db.tshard where idx = 3";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        let sqls = res.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();

        assert_eq!(sqls[0], "SELECT * FROM `db0`.tshard_00003 where idx = 3");
        assert_eq!(sqls[1], "SELECT * FROM `db1`.tshard_00003 where idx = 3");

        let raw_sql = "SELECT * FROM db.tshard where didx = 5";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };

        let res = sr.rewrite(input).unwrap();
        let sqls = res.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();
        assert_eq!(sqls[0], "SELECT * FROM `db1`.tshard_00000 where didx = 5");
        assert_eq!(sqls[1], "SELECT * FROM `db1`.tshard_00001 where didx = 5");
        assert_eq!(sqls[2], "SELECT * FROM `db1`.tshard_00002 where didx = 5");
        assert_eq!(sqls[3], "SELECT * FROM `db1`.tshard_00003 where didx = 5");

        let raw_sql = "SELECT * FROM db.tshard where didx = 6 and idx = 3";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };

        let res = sr.rewrite(input).unwrap();
        let sqls = res.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();

        assert_eq!(sqls[0], "SELECT * FROM `db0`.tshard_00003 where didx = 6 and idx = 3");
    }

    #[test]
    fn test_database_table_sharding_strategy_insert() {
        let config = get_database_table_sharding_config();
        let raw_sql = "INSERT INTO db.tshard(didx, idx) VALUES (12, 22), (13, 57), (19, 37)";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        let sqls = res.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();
        assert_eq!(sqls[0], "INSERT INTO `db0`.tshard_00000(didx, idx) VALUES (12, 22)");
        assert_eq!(sqls[1], "INSERT INTO `db1`.tshard_00001(didx, idx) VALUES (13, 57), (19, 37)");
    }

    #[test]
    fn test_database_sharding_strategy() {
        let config = get_database_sharding_config();
        let raw_sql = "SELECT idx from `db`.tshard where idx = 3";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from `db1`.tshard where idx = 3");

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 3)";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from db1.tshard where idx = 3 and idx = (SELECT idx from db1.tshard where idx = 3)");

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 4)";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "SELECT idx from db0.tshard where idx = 3 and idx = (SELECT idx from db0.tshard where idx = 4)",
                "SELECT idx from db1.tshard where idx = 3 and idx = (SELECT idx from db1.tshard where idx = 4)",
            ],
        );

        let raw_sql = "INSERT INTO db.tshard(idx, tt) VALUES (12, 22), (13, 55), (16, 77)";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "INSERT INTO db0.tshard(idx, tt) VALUES (12, 22), (16, 77)", 
                "INSERT INTO db1.tshard(idx, tt) VALUES (13, 55)"
            ]
        );
    }

    #[test]
    fn test_table_sharding_strategy() {
        let config = get_table_sharding_config();
        let parser = Parser::new();
        let raw_sql = "SELECT idx from db.tshard where idx > 3".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "SELECT idx from `db`.tshard_00000 where idx > 3",
                "SELECT idx from `db`.tshard_00001 where idx > 3",
                "SELECT idx from `db`.tshard_00002 where idx > 3",
                "SELECT idx from `db`.tshard_00003 where idx > 3",
            ],
        );

        let raw_sql = "SELECT idx from db.tshard where idx = 4".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res[0].target_sql,
            "SELECT idx from `db`.tshard_00000 where idx = 4".to_string()
        );

        let raw_sql = "SELECT idx from db.`tshard` where idx = 3 and idx = (SELECT idx from db.tshard where idx = 3)".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from `db`.`tshard_00003` where idx = 3 and idx = (SELECT idx from `db`.tshard_00003 where idx = 3)".to_string());

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 4)".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "SELECT idx from `db`.tshard_00000 where idx = 3 and idx = (SELECT idx from `db`.tshard_00000 where idx = 4)",
                "SELECT idx from `db`.tshard_00001 where idx = 3 and idx = (SELECT idx from `db`.tshard_00001 where idx = 4)",
                "SELECT idx from `db`.tshard_00002 where idx = 3 and idx = (SELECT idx from `db`.tshard_00002 where idx = 4)",
                "SELECT idx from `db`.tshard_00003 where idx = 3 and idx = (SELECT idx from `db`.tshard_00003 where idx = 4)",
            ],
        );
    }

    #[test]
    fn test_table_sharding_strategy_insert() {
        let config = get_table_sharding_config();
        let raw_sql = "INSERT INTO db.tshard(idx) VALUES (12), (13), (16)";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();

        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "INSERT INTO `db`.tshard_00000(idx) VALUES (12), (16)",
                "INSERT INTO `db`.tshard_00001(idx) VALUES (13)",
            ],
        );
    }

    #[test]
    fn test_table_sharding_strategy_avg() {
        let config = get_table_sharding_config();
        let parser = Parser::new();
        let raw_sql = "SELECT AVG(price) FROM db.tshard WHERE idx > 3".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.clone(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT COUNT(price) AS PRICE_AVG_DERIVED_COUNT_00000, SUM(price) AS PRICE_AVG_DERIVED_SUM_00000 FROM `db`.tshard_00000 WHERE idx > 3");

        let raw_sql =
            "SELECT AVG(pbl), AVG(znl), AVG(ngl) FROM db.tshard WHERE idx > 3".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.clone(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT COUNT(pbl) AS PBL_AVG_DERIVED_COUNT_00000, SUM(pbl) AS PBL_AVG_DERIVED_SUM_00000, COUNT(znl) AS ZNL_AVG_DERIVED_COUNT_00000, SUM(znl) AS ZNL_AVG_DERIVED_SUM_00000, COUNT(ngl) AS NGL_AVG_DERIVED_COUNT_00000, SUM(ngl) AS NGL_AVG_DERIVED_SUM_00000 FROM `db`.tshard_00000 WHERE idx > 3");

        let raw_sql = "SELECT AVG(pbl) FROM db.tshard WHERE idx = 3".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.clone(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT COUNT(pbl) AS PBL_AVG_DERIVED_COUNT_00003, SUM(pbl) AS PBL_AVG_DERIVED_SUM_00003 FROM `db`.tshard_00003 WHERE idx = 3");

        let raw_sql = "SELECT AVG(znl) FROM db.tshard";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT COUNT(znl) AS ZNL_AVG_DERIVED_COUNT_00000, SUM(znl) AS ZNL_AVG_DERIVED_SUM_00000 FROM `db`.tshard_00000");

        let raw_sql =
            "SELECT * from db.tshard where znl > (SELECT AVG(znl) from db.tshard)".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.clone(),
            ast: ast[0].clone(),
            default_db: None,
        };

        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT * from `db`.tshard_00000 where znl > (SELECT COUNT(znl) AS ZNL_AVG_DERIVED_COUNT_00000, SUM(znl) AS ZNL_AVG_DERIVED_SUM_00000 from `db`.tshard_00000)")
    }

    #[test]
    fn test_table_sharding_strategy_update_delete() {
        let config = get_table_sharding_config();
        let raw_sql = "UPDATE db.tshard set a=1 where idx = 2";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec!["UPDATE `db`.tshard_00002 set a=1 where idx = 2"],
        );

        let raw_sql = "DELETE FROM db.tshard where idx = 1";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec!["DELETE FROM `db`.tshard_00001 where idx = 1"],
        );
    }

    #[test]
    fn test_default_db() {
        let config = get_table_sharding_config();
        let raw_sql = "UPDATE tshard set a=1 where idx = 2";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: Some("db".to_string()),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec!["UPDATE `db`.tshard_00002 set a=1 where idx = 2"],
        );
    }

    #[test]
    fn test_table_sharding_strategy_order_group_by() {
        let config = get_table_sharding_config();
        let parser = Parser::new();
        let raw_sql = "SELECT order_id, order_item_id FROM db.tshard ORDER BY user_id";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            default_db: None,
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res[0].target_sql,
            "SELECT order_id, order_item_id, user_id AS USER_ID_ORDER_BY_DERIVED_00000 FROM `db`.tshard_00000 ORDER BY user_id"
        );

        let raw_sql = "SELECT order_id, order_item_id FROM db.tshard GROUP BY user_id";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            default_db: None,
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res[0].target_sql,
            "SELECT order_id, order_item_id, user_id AS USER_ID_GROUP_BY_DERIVED_00000 FROM `db`.tshard_00000 GROUP BY user_id"
        );

        let raw_sql =
            "SELECT order_id, order_item_id from db.tshard where user_id > 3 ORDER BY user_id";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            default_db: None,
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res[0].target_sql,
            "SELECT order_id, order_item_id, user_id AS USER_ID_ORDER_BY_DERIVED_00000 from `db`.tshard_00000 where user_id > 3 ORDER BY user_id"
        );

        let raw_sql = "SELECT order_id, order_item_id FROM db.tshard WHERE id in (SELECT s_id, ngl, znl from db.tshard) ORDER BY user_id";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            default_db: None,
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res[0].target_sql,
            "SELECT order_id, order_item_id, user_id AS USER_ID_ORDER_BY_DERIVED_00000 FROM `db`.tshard_00000 WHERE id in (SELECT s_id, ngl, znl from `db`.tshard_00000) ORDER BY user_id"
        );

        let raw_sql =
            "SELECT order_id, order_item_id from db.tshard where idx = 3 ORDER BY `user_id`";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            default_db: None,
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res[0].target_sql,
            "SELECT order_id, order_item_id, `user_id` AS USER_ID_ORDER_BY_DERIVED_00003 from `db`.tshard_00003 where idx = 3 ORDER BY `user_id`"
        );

        let raw_sql = "SELECT id FROM db.tshard ORDER BY `id`";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            default_db: None,
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT id FROM `db`.tshard_00000 ORDER BY `id`");

        let raw_sql = "SELECT id FROM db.tshard ORDER BY idx DESC";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            default_db: None,
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res[0].target_sql,
            "SELECT id, idx AS IDX_ORDER_BY_DERIVED_00000 FROM `db`.tshard_00000 ORDER BY idx DESC"
        );
    }

    #[test]
    fn test_table_sharding_strategy_rw() {
        let config = get_table_sharding_config();
        let parser = Parser::new();
        let raw_sql = "SELECT id FROM db.tshard where idx = 10";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, true);
        let input = ShardingRewriteInput {
            default_db: None,
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].data_source, DataSource::NodeGroup("ds001".to_string()));
    }
}
