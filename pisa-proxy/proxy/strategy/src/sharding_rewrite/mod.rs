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

use crc32fast::hash;
use endpoint::endpoint::Endpoint;
use indexmap::IndexMap;
use mysql_parser::ast::{SqlStmt, TableIdent, Visitor};
use paste::paste;

use self::{
    generic_meta::{ShardingMeta, ShardingMetaBaseInfo},
    meta::*,
};
use crate::{
    config::{NodeGroup, Sharding, ShardingAlgorithmName},
    get_meta_detail,
    rewrite::{ShardingRewriteInput, ShardingRewriter},
    sharding_rewrite::{
        meta::{FieldAggFunc, GroupMeta, OrderMeta},
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
                let checksum = hash(&self.to_be_bytes()) as u64;
                Some(checksum.wrapping_rem(sharding_count as u64))
            }
        }
    }
}

impl CalcShardingIdx<i64> for i64 {
    fn calc(self, algo: &ShardingAlgorithmName, sharding_count: i64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => Some(self.wrapping_rem(sharding_count) as u64),
            ShardingAlgorithmName::CRC32Mod => {
                let checksum = hash(&self.to_be_bytes()) as i64;
                Some(checksum.wrapping_rem(sharding_count) as u64)
            }
        }
    }
}

impl CalcShardingIdx<f64> for f64 {
    fn calc(self, algo: &ShardingAlgorithmName, sharding_count: f64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => Some((self % sharding_count).round() as u64),
            ShardingAlgorithmName::CRC32Mod => {
                let checksum = hash(&self.to_be_bytes()) as u64;
                Some(checksum.wrapping_rem(sharding_count.round() as u64))
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ShardingRewriteOutput {
    pub results: Vec<ShardingRewriteResult>,
    pub agg_fields: IndexMap<u8, Vec<FieldMetaIdent>>,
}

#[derive(Debug, Clone)]
pub struct ShardingRewriteResult {
    pub ds_idx: DataSourceShardingIdx,
    pub changes: IndexMap<u8, Vec<ChangeTargetMeta>>,
    pub target_sql: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum DataSource {
    Endpoint(Endpoint),
    NodeGroup(String),
    None,
}

#[derive(Debug, Clone)]
struct ChangePlan {
    query_id: u8,
    target: String,
    span: mysql_parser::Span,
    action: ChangePlanAction,
    typ: ChangePlanTyp,
    target_meta: ChangeTargetMeta,
}

#[derive(Debug, Clone, PartialEq)]
enum ChangePlanAction {
    // The Add means that add field, The Span.end() should be used.
    Add,
    // The Replace means that replace field, eg, avg function, The Span.start() should be used.
    Replace,
}

#[derive(Debug, Clone, PartialEq)]
enum ChangePlanTyp {
    // The Field
    Field,

    // The Table
    Table,
}

#[derive(Debug, Clone)]
pub enum ChangeTargetMeta {
    Table(String),
    Avg(AvgChange),
    OrderGroup { order_fields: Vec<String>, group_fields: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct AvgChange {
    pub count_field: String,
    pub sum_field: String,
    pub ori_field: String,
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct ShardingColumn {
    pub database: Option<String>,
    pub table: Option<String>,
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

    query_metas: IndexMap<u8, QueryMeta>,
    field_block_metas: IndexMap<u8, FieldBlockMeta>,

    // The key is a tuple means that database index, table index.
    change_plans: IndexMap<DataSourceShardingIdx, Vec<ChangePlan>>,
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
struct InsertRowValueIdx {
    sharding_idx: ShardingIdx,
    span: mysql_parser::Span,
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct ShardingIdx {
    pub database: Option<u64>,
    pub table: Option<u64>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DataSourceShardingIdx {
    pub ds: DataSource,
    pub idx: ShardingIdx,
    pub column: ShardingColumn,
}

#[derive(Debug)]
pub enum StrategyTyp {
    DatabaseTable,
    Database,
    Table,
}

#[derive(Debug)]
enum DatabaseTableStrategyPart {
    Database(usize),
    Table(usize),
    DatabaseTable(usize, usize),
    All,
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
            query_metas: IndexMap::new(),
            field_block_metas: IndexMap::new(),
            change_plans: IndexMap::new(),
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
        &mut self,
        meta: RewriteMetaData,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Result<ShardingRewriteOutput, ShardingRewriteError> {
        get_meta_detail!(meta, wheres, inserts, fields, orders, groups);
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
                fields,
                orders,
                groups,
            ));
        }

        Ok(self.database_table_strategy_iproduct(DatabaseTableStrategyPart::DatabaseTable(wheres[0].1[0].unwrap() as usize, wheres[0].1[1].unwrap() as usize), try_tables,fields, orders, groups))
    }

    fn database_strategy(
        &mut self,
        meta: RewriteMetaData,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Result<ShardingRewriteOutput, ShardingRewriteError> {
        get_meta_detail!(meta, wheres, inserts, fields, orders, groups);
        if !inserts.is_empty() {
            if fields.is_empty() {
                return Err(ShardingRewriteError::FieldsIsEmpty);
            }

            return self.change_insert_sql(try_tables, fields, inserts);
        }

        if wheres.is_empty() {
            return Ok(self.database_strategy_iproduct(DatabaseTableStrategyPart::All, try_tables, fields, orders, groups));
        }

        let wheres = Self::find_try_where(StrategyTyp::Database, &try_tables, wheres)?;
        if wheres.is_empty() {
            return Ok(self.database_strategy_iproduct(DatabaseTableStrategyPart::All, try_tables, fields, orders, groups));
        }

        let expect_sum = wheres[0].1[0].unwrap() as usize * wheres.len();
        let sum: usize = wheres.iter().map(|x| x.1[0].unwrap()).sum::<u64>() as usize;

        if expect_sum != sum {
            return Ok(self.database_strategy_iproduct(DatabaseTableStrategyPart::All, try_tables, fields, orders, groups));
        }

        let output = self.database_strategy_iproduct(DatabaseTableStrategyPart::Database(wheres[0].1[0].unwrap() as usize), try_tables, fields, orders, groups);
        Ok(output)
    }

    fn table_strategy(
        &mut self,
        meta: RewriteMetaData,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Result<ShardingRewriteOutput, ShardingRewriteError> {
        get_meta_detail!(meta, wheres, inserts, fields, orders, groups);

        if !inserts.is_empty() {
            if fields.is_empty() {
                return Err(ShardingRewriteError::FieldsIsEmpty);
            }

            return self.change_insert_sql(try_tables, fields, inserts);
        }

        if wheres.is_empty() {
            return Ok(self.table_strategy_iproduct(DatabaseTableStrategyPart::All, try_tables, fields, orders, groups));
        }

        let wheres = Self::find_try_where(StrategyTyp::Table, &try_tables, wheres)?;
        if wheres.is_empty() {
            return Ok(self.table_strategy_iproduct(DatabaseTableStrategyPart::All, try_tables, fields, orders, groups));
        }

        let expect_sum = wheres[0].1[0].unwrap() as usize * wheres.len();
        let sum: usize = wheres.iter().map(|x| x.1[0].unwrap()).sum::<u64>() as usize;

        if expect_sum != sum {
            return Ok(self.table_strategy_iproduct(DatabaseTableStrategyPart::All, try_tables, fields, orders, groups));
        }

        let output = self.table_strategy_iproduct(DatabaseTableStrategyPart::Table(wheres[0].1[0].unwrap() as usize), try_tables, fields, orders, groups);
        Ok(output)
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

    fn change_order_group(
        &mut self,
        query_id: u8,
        orders: Option<&Vec<OrderMeta>>,
        groups: Option<&Vec<GroupMeta>>,
        fields: Option<&Vec<FieldMeta>>,
        ds_sharding_idx: &DataSourceShardingIdx,
    ) -> Result<usize, ()> {
        if orders.is_none() && groups.is_none() {
            return Ok(0);
        }

        let default_fields = Vec::<FieldMeta>::new();
        let fields = fields.map_or_else(|| &default_fields, |v| v);

        // Currently, we don't consider the case that query_id not equal 1.

        let field_block_span = self.field_block_metas.get(&query_id).unwrap().span;

        let default_orders = Vec::<OrderMeta>::new();
        let orders = orders.map_or_else(|| &default_orders, |v| v);
        // Get the order field that does not exist in the fields.
        let not_exist_order_fields = orders
            .iter()
            .filter_map(|x| {
                let exist_field = fields.iter().find(|field| {
                    if let FieldMeta::Ident(field_ident) = field {
                        if field_ident.name.replace("`", "") == x.name.replace("`", "") {
                            return true;
                        }
                    }
                    false
                });

                exist_field.is_none().then(|| x)
            })
            .collect::<Vec<_>>();

        let default_groups = Vec::<GroupMeta>::new();
        let groups = groups.map_or_else(|| &default_groups, |v| v);

        // Get the group field that does not exist in the fields.
        let not_exist_group_fields = groups
            .iter()
            .filter_map(|x| {
                let exist_field = fields.iter().find(|field| {
                    if let FieldMeta::Ident(field_ident) = field {
                        if field_ident.name.replace("`", "") == x.name.replace("`", "") {
                            return true;
                        }
                    }
                    false
                });

                exist_field.is_none().then(|| x)
            })
            .collect::<Vec<_>>();

        let mut target_fields = vec![];

        let (mut added_order_fields, mut added_group_fields) = (vec![], vec![]);

        for field in not_exist_order_fields {
            let order_as_name = field.name.replace("`", "").to_ascii_uppercase();
            let order_as_name =
                format!("{}_{}_{:05}", order_as_name, ORDER_BY_DERIVED, ds_sharding_idx.idx.table.unwrap());
            let target_field = format!("{} {} {}", field.name, AS, &order_as_name,);
            target_fields.push(target_field.clone());

            let mut change_target = IndexMap::new();
            change_target.insert(ORDER_TARGET.to_string(), target_field);
            change_target.insert(ORDER_FIELD.to_string(), field.name.clone());
            added_order_fields.push(order_as_name);
        }

        for field in not_exist_group_fields {
            let group_as_name = field.name.replace("`", "").to_ascii_uppercase();
            let group_as_name =
                format!("{}_{}_{:05}", group_as_name, GROUP_BY_DERIVED, ds_sharding_idx.idx.table.unwrap());
            let target_field = format!("{} {} {}", field.name, AS, &group_as_name,);
            target_fields.push(target_field.clone());

            let mut change_target = IndexMap::new();
            change_target.insert(GROUP_TARGET.to_string(), target_field);
            change_target.insert(GROUP_FIELD.to_string(), field.name.clone());
            added_group_fields.push(group_as_name);
        }

        if target_fields.is_empty() {
            return Ok(0);
        }

        let mut target_fields_string = target_fields.join(", ");
        target_fields_string.insert_str(0, ", ");
        let length = target_fields_string.len();
        let change_plan = ChangePlan {
            target: target_fields_string.clone(),
            span: field_block_span,
            action: ChangePlanAction::Add,
            typ: ChangePlanTyp::Field,
            query_id,
            target_meta: ChangeTargetMeta::OrderGroup {
                order_fields: added_order_fields,
                group_fields: added_group_fields,
            },
        };

        self.change_plans.entry(ds_sharding_idx.clone()).or_insert(vec![]).push(change_plan);

        return Ok(length);
    }

    fn change_avg(
        &mut self,
        query_id: u8,
        avgs: Option<&Vec<FieldMeta>>,
        ds_sharding_idx: &DataSourceShardingIdx,
    ) {
        if avgs.is_none() {
            return;
        }

        let avgs = avgs.unwrap();

        let mut res = IndexMap::new();

        for meta in avgs {
            if let FieldMeta::Ident(meta) = meta {
                if let FieldAggFunc::Avg { avg_field_name, field_name, span } = &meta.agg_func {
                    res.insert(AVG_FIELD.to_string(), avg_field_name.clone());
                    let target_count = format!("{}({}) {} ", COUNT, field_name, AS);
                    let target_count_as = format!(
                        "{}_{}_{:05}",
                        field_name.to_ascii_uppercase(),
                        AVG_DERIVED_COUNT,
                        ds_sharding_idx.idx.table.as_ref().unwrap(),
                    );
                    res.insert(AVG_COUNT.to_string(), target_count_as.to_string());

                    let target_sum = format!("{}({}) {} ", SUM, field_name, AS);
                    let target_sum_as = format!(
                        "{}_{}_{:05}",
                        field_name.to_ascii_uppercase(),
                        AVG_DERIVED_SUM,
                        ds_sharding_idx.idx.table.as_ref().unwrap(),
                    );
                    res.insert(AVG_SUM.to_string(), target_sum_as.to_string());

                    let target =
                        format!("{}{}, {}{}", target_count, &target_count_as, target_sum, &target_sum_as);

                    let change_plan = ChangePlan {
                        query_id,
                        target,
                        span: *span,
                        action: ChangePlanAction::Replace,
                        typ: ChangePlanTyp::Field,
                        target_meta: ChangeTargetMeta::Avg( AvgChange {
                            count_field: target_count_as,
                            sum_field: target_sum_as,
                            ori_field: avg_field_name.clone(),
                        }),
                    };

                    self.change_plans.entry(ds_sharding_idx.clone()).or_insert(vec![]).push(change_plan);
                }
            }
        }
    }

    fn change_insert_sql(
        &self,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
        inserts: &IndexMap<u8, Vec<InsertValsMeta>>,
    ) -> Result<ShardingRewriteOutput, ShardingRewriteError> {
        let results = try_tables
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
                )
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        
        Ok(ShardingRewriteOutput { results, agg_fields: IndexMap::new() })

    }

    fn change_insert_sql_inner<'b>(
        &self,
        rule: &Sharding,
        table: &TableIdent,
        inserts: &[InsertValsMeta],
        fields: &[FieldMeta],
        meta_base_info: ShardingMetaBaseInfo<'b>,
    ) -> Result<Vec<ShardingRewriteResult>, ShardingRewriteError> {
        let strategy_typ = rule.get_strategy_typ();
        let changes = Self::change_insert(&strategy_typ, inserts, fields, &meta_base_info)?;
        let row_start_idx = changes[0].span.start();
        let sql_prefix_text = &self.raw_sql[0..row_start_idx];

        let mut sqls = IndexMap::<(Option<u64>, Option<u64>), String>::new();
        match strategy_typ {
            StrategyTyp::Database => {
                for change in changes.iter() {
                    let db_idx = change.sharding_idx.database.unwrap();
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
                    let table_idx = change.sharding_idx.table.unwrap();
                    let target = self.change_table(table, "", table_idx);
                    let mut target_sql_prefix_text = sql_prefix_text.to_string();
                    let row_value_text = self.get_change_insert_row(
                        &mut target_sql_prefix_text,
                        &target,
                        table.span,
                        change,
                    );
                    sqls.entry((None, Some(table_idx)))
                        .or_insert(target_sql_prefix_text)
                        .push_str(&row_value_text);
                }
            }

            StrategyTyp::DatabaseTable => {
                for change in changes.iter() {
                    let db_idx = change.sharding_idx.database.unwrap();
                    let actual_db = rule
                        .get_actual_schema(&self.endpoints, Some(db_idx as usize))
                        .unwrap_or_else(|| "");
                    let _ = self.change_table(table, actual_db, db_idx);
                    let table = TableIdent {
                        span: table.span.clone(),
                        schema: Some(actual_db.to_string()),
                        name: table.name.clone(),
                    };

                    let table_idx = change.sharding_idx.table.unwrap();
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
        let results = sqls
            .into_iter()
            .map(|((db_idx, table_idx), v)| -> Result<ShardingRewriteResult, ShardingRewriteError> {
                let idx = db_idx.unwrap_or_else(|| 0) as usize;
                let data_source = self.gen_data_source(rule, idx)?;

                Ok(ShardingRewriteResult {
                    ds_idx: DataSourceShardingIdx { 
                        ds: data_source, 
                        idx: ShardingIdx { database: db_idx, table: table_idx },
                        column: ShardingColumn { 
                            database: meta_base_info.column.0.map(|x| x.to_string()), 
                            table: meta_base_info.column.1.map(|x| x.to_string()),
                        },
                    },
                    changes: IndexMap::new(),
                    target_sql: v.trim_end_matches(", ").to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    fn get_change_insert_row(
        &self,
        prefix_text: &mut String,
        target: &str,
        span: mysql_parser::Span,
        row_value_idx: &InsertRowValueIdx,
    ) -> String {
        Self::change_sql(prefix_text, span, target, 0);
        let mut row_value_text =
            self.raw_sql[row_value_idx.span.start()..row_value_idx.span.end()].to_string();
        row_value_text.push_str(", ");
        row_value_text
    }

    fn change_insert<'b>(
        strategy_typ: &StrategyTyp,
        inserts: &[InsertValsMeta],
        fields: &[FieldMeta],
        meta_base_info: &ShardingMetaBaseInfo<'b>,
    ) -> Result<Vec<InsertRowValueIdx>, ShardingRewriteError> {
        let insert_values = Self::find_inserts(&strategy_typ, inserts, fields, meta_base_info)?;
        let mut changes = vec![];

        match strategy_typ {
            StrategyTyp::Database => {
                for value in insert_values.iter() {
                    let idx = Self::calc_database_or_table_sharding_idx(
                        value.sharding_value.database.as_ref(),
                        &meta_base_info,
                    )?;
                    changes.push(InsertRowValueIdx {
                        sharding_idx: ShardingIdx { database: Some(idx), table: None },
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

                    changes.push(InsertRowValueIdx {
                        sharding_idx: ShardingIdx { database: None, table: Some(idx) },
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

                    changes.push(InsertRowValueIdx {
                        sharding_idx: ShardingIdx {
                            database: Some(db_idx),
                            table: Some(table_idx),
                        },
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
        &mut self,
        part: DatabaseTableStrategyPart,
        tables: Vec<(u8, Sharding, &TableIdent)>,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
        orders: &IndexMap<u8, Vec<OrderMeta>>,
        groups: &IndexMap<u8, Vec<GroupMeta>>,
    ) -> ShardingRewriteOutput {
        for t in tables.iter() {
            let sharding_column = t.1.get_sharding_column();
            let sharding_count = t.1.get_sharding_count().1.unwrap() as u64;

            let (actual_nodes, db_idx) = match part {
                DatabaseTableStrategyPart::Database(idx) | DatabaseTableStrategyPart::DatabaseTable(idx, _) => {
                    (vec![t.1.actual_datanodes[idx].clone()], Some(idx))
                },
                _ => { (t.1.actual_datanodes.clone(), None) }
            };

            for (node_idx, node) in actual_nodes.iter().enumerate() {
                let ep = self.endpoints.iter().find(|e| e.name.eq(node)).unwrap().clone();
                let _ = self.change_table(t.2, &ep.db, 0);
                let table =
                    TableIdent { span: t.2.span, schema: Some(ep.db), name: t.2.name.clone() };
                
                let db_idx = db_idx.unwrap_or_else(|| node_idx);
                let data_source = self.gen_data_source(&t.1, db_idx).unwrap();

                let sharding_range = match part {
                    DatabaseTableStrategyPart::Table(idx) | DatabaseTableStrategyPart::DatabaseTable(_, idx) => {
                        idx .. idx + 1
                    },

                    _ => { 0 .. sharding_count as usize }
                };

                for table_idx in sharding_range {
                    let target = self.change_table(&table, "", table_idx as u64);
                    let change_plan = ChangePlan {
                        query_id: t.0,
                        target: target.clone(),
                        span: t.2.span,
                        action: ChangePlanAction::Replace,
                        typ: ChangePlanTyp::Table,
                        target_meta: ChangeTargetMeta::Table(target.clone()),
                    };

                    let ds_sharding_idx = DataSourceShardingIdx {
                        idx: ShardingIdx { database: Some(db_idx as u64), table: Some(table_idx as u64) },
                        ds: data_source.clone(),
                        column: ShardingColumn { database: sharding_column.0.map(|x| x.to_string()), table: sharding_column.1.map(|x| x.to_string()) }
                    };

                    self.change_plans
                        .entry(ds_sharding_idx.clone())
                        .or_insert(vec![])
                        .push(change_plan);
                    let _ = self.change_order_group(
                        t.0,
                        orders.get(&t.0),
                        groups.get(&t.0),
                        fields.get(&t.0),
                        &ds_sharding_idx,
                    );

                    self.change_avg(t.0, fields.get(&t.0), &ds_sharding_idx);
                }
            }
        }
        self.change_plan_apply(fields)
    }

    fn database_strategy_iproduct(
        &mut self,
        part: DatabaseTableStrategyPart,
        tables: Vec<(u8, Sharding, &TableIdent)>,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
        orders: &IndexMap<u8, Vec<OrderMeta>>,
        groups: &IndexMap<u8, Vec<GroupMeta>>,
    ) -> ShardingRewriteOutput  {
        for t in tables.iter() {
            let sharding_column = Some(t.1.get_sharding_column().0.unwrap().to_string());

            let (actual_nodes, db_idx) = match part {
                DatabaseTableStrategyPart::Database(idx) => {
                    (vec![t.1.actual_datanodes[idx].clone()], Some(idx))
                },
                _ => { (t.1.actual_datanodes.clone(), None) }
            };

            for (node_idx, node) in actual_nodes.iter().enumerate() {
                let ep = self.endpoints.iter().find(|e| e.name.eq(node)).unwrap().clone();
                let target = self.change_table(t.2, &ep.db, 0);
                let data_source = self.gen_data_source(&t.1, db_idx.unwrap_or_else(|| node_idx)).unwrap();

                let change_plan = ChangePlan {
                    query_id: t.0,
                    target: target.clone(),
                    span: t.2.span,
                    action: ChangePlanAction::Replace,
                    typ: ChangePlanTyp::Table,
                    target_meta: ChangeTargetMeta::Table(target.clone()),
                };

                let ds_sharding_idx = DataSourceShardingIdx {
                    idx: ShardingIdx { database: Some(node_idx as u64), table: None },
                    ds: data_source,
                    column: ShardingColumn { database: sharding_column.clone(), table: None }
                };

                self.change_plans
                    .entry(ds_sharding_idx.clone())
                    .or_insert(vec![])
                    .push(change_plan);
                let _ = self.change_order_group(
                    t.0,
                    orders.get(&t.0),
                    groups.get(&t.0),
                    fields.get(&t.0),
                    &ds_sharding_idx,
                );

                self.change_avg(t.0, fields.get(&t.0), &ds_sharding_idx);
            }
        }

        self.change_plan_apply(fields)
    }

    fn table_strategy_iproduct(
        &mut self,
        part: DatabaseTableStrategyPart,
        tables: Vec<(u8, Sharding, &TableIdent)>,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
        orders: &IndexMap<u8, Vec<OrderMeta>>,
        groups: &IndexMap<u8, Vec<GroupMeta>>,
    ) -> ShardingRewriteOutput {
        for t in tables.iter() {
            let data_source = self.gen_data_source(&t.1, 0).unwrap();
            let sharding_column = Some(t.1.get_sharding_column().1.unwrap().to_string());
            let sharding_count = t.1.get_sharding_count().1.unwrap();

            let sharding_range = match part {
                DatabaseTableStrategyPart::Table(idx) => {
                    idx .. idx + 1
                },
                _ => { 0 .. sharding_count as usize }
            };

            for idx in sharding_range {
                let target = self.change_table(t.2, "", idx as u64);
                let change_plan = ChangePlan {
                    query_id: t.0,
                    target: target.clone(),
                    span: t.2.span,
                    action: ChangePlanAction::Replace,
                    typ: ChangePlanTyp::Table,
                    target_meta: ChangeTargetMeta::Table(target.clone()),
                };

                let ds_sharding_idx = DataSourceShardingIdx {
                    idx: ShardingIdx { database: None, table: Some(idx as u64) },
                    ds: data_source.clone(),
                    column: ShardingColumn { database: None, table: sharding_column.clone() }
                };

                self.change_plans
                    .entry(ds_sharding_idx.clone())
                    .or_insert(vec![])
                    .push(change_plan);
                let _ = self.change_order_group(
                    t.0,
                    orders.get(&t.0),
                    groups.get(&t.0),
                    fields.get(&t.0),
                    &ds_sharding_idx,
                );

                self.change_avg(t.0, fields.get(&t.0), &ds_sharding_idx);
            }
        }

        self.change_plan_apply(fields)
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

    fn get_agg_field1(
        fields: &IndexMap<u8, Vec<FieldMeta>>,
    ) -> IndexMap<u8, Vec<FieldMetaIdent>> {
        IndexMap::from_iter(fields.iter().filter_map(|(query_id, values)| {
            let counts = values
                .iter()
                .filter_map(|f| {
                    if let FieldMeta::Ident(meta) = f {
                        let is_match = matches!(meta.agg_func,  FieldAggFunc::None);
                        return (!is_match).then(|| meta.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            (counts.len() > 0).then(|| (*query_id, counts))
        }))
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

    fn change_plan_apply(
        &mut self,
        fields: &IndexMap<u8, Vec<FieldMeta>>,
    ) -> ShardingRewriteOutput {
        let query_length = self.query_metas.len() as u8;

        let mut target_sqls = IndexMap::<_, _>::new();
        let mut rewrite_outputs = vec![];

        for (ds_idx, plans) in self.change_plans.iter_mut() {
            let mut rewrite_changes = IndexMap::<u8, Vec<ChangeTargetMeta>>::new();
            let mut target_sql = self.raw_sql.to_string();


            for query_id in (1..query_length + 1).rev() {
                let mut replace_plans = plans
                    .iter()
                    .filter(|x| {
                        x.query_id == query_id
                            && x.action == ChangePlanAction::Replace
                            && x.typ == ChangePlanTyp::Field
                    })
                    .collect::<Vec<_>>();
                replace_plans.sort_by_cached_key(|x| x.span.start());
                let mut offset = 0;
                for plan in replace_plans.iter() {
                    for _ in 0..plan.span.len() {
                        target_sql.remove(plan.span.start() + offset);
                    }
                    target_sql.insert_str(plan.span.start() + offset, &plan.target);
                    offset += plan.target.len() - plan.span.len();
                    rewrite_changes
                        .entry(query_id)
                        .or_insert(vec![])
                        .push(plan.target_meta.clone());
                }

                let mut add_plans = plans
                    .iter()
                    .filter(|x| {
                        x.query_id == query_id
                            && x.action == ChangePlanAction::Add
                            && x.typ == ChangePlanTyp::Field
                    })
                    .collect::<Vec<_>>();
                add_plans.sort_by_cached_key(|x| x.span.start());
                for plan in add_plans.iter() {
                    target_sql.insert_str(plan.span.end() + offset, &plan.target);
                    offset += plan.target.len();
                    rewrite_changes
                        .entry(query_id)
                        .or_insert(vec![])
                        .push(plan.target_meta.clone());
                }

                let mut replace_plans = plans
                    .iter()
                    .filter(|x| {
                        x.query_id == query_id
                            && x.action == ChangePlanAction::Replace
                            && x.typ == ChangePlanTyp::Table
                    })
                    .collect::<Vec<_>>();
                replace_plans.sort_by_cached_key(|x| x.span.start());

                for plan in replace_plans.iter() {
                    for _ in 0..plan.span.len() {
                        target_sql.remove(plan.span.start() + offset);
                    }
                    target_sql.insert_str(plan.span.start() + offset, &plan.target);
                    offset += plan.target.len() - plan.span.len();
                    rewrite_changes
                        .entry(query_id)
                        .or_insert(vec![])
                        .push(plan.target_meta.clone());
                }
            }

            let result = ShardingRewriteResult {
                ds_idx: ds_idx.clone(),
                changes: rewrite_changes,
                target_sql: target_sql.clone(),
            };

            rewrite_outputs.push(result);
            target_sqls.insert(ds_idx.idx.clone(), target_sql);
        }

        let output = ShardingRewriteOutput {
            results: rewrite_outputs,
            agg_fields: Self::get_agg_field1(fields)
        };

        output
    }


}

impl ShardingRewriter<ShardingRewriteInput> for ShardingRewrite {
    type Output = Result<ShardingRewriteOutput, ShardingRewriteError>;
    fn rewrite(&mut self, mut input: ShardingRewriteInput) -> Self::Output {
        self.set_raw_sql(input.raw_sql);
        self.set_default_db(input.default_db);

        let meta = self.get_meta(&mut input.ast);
        self.query_metas = meta.get_queries().clone();
        self.field_block_metas = meta.get_field_blocks().clone();

        let tables = meta.get_tables().clone();
        let try_tables = self.find_table_rule(&tables);

        if try_tables.is_empty() {
            return Ok(ShardingRewriteOutput::default());
        }

        self.change_plans.clear();
        // Strategy according to first element of `try_tables`.
        let rule = &try_tables[0].1;

        if rule.database_strategy.is_some() {
            return self.database_strategy(meta, try_tables);
        } else if rule.table_strategy.is_some() {
            return self.table_strategy(meta, try_tables);
        } else if rule.database_table_strategy.is_some() {
            return self.database_table_strategy(meta, try_tables);
        }

        return Ok(ShardingRewriteOutput::default());
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
                        sharding_count: 4,
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
        //let raw_sql = "SELECT count(user_id) as c, avg(tt), avg(oid),user_id FROM db.tshard where idx = (SELECT user_id, avg(ss) from db.tshard where idx = 3 order by idx) group by idx order by idx";
        //let raw_sql = "UPDATE db.tshard set a=1 where idx=3";
        //let raw_sql = "SELECT idx from db.`tshard` where idx = 3 and idx = (SELECT idx from db.tshard where idx = 3)";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: Some("db".to_string()),
        };
        let res = sr.rewrite(input).unwrap();
        let sqls = res.results.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();
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
        let sqls = res.results.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();

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
        let sqls = res.results.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();
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
        let sqls = res.results.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();

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
        let sqls = res.results.iter().map(|x| x.target_sql.clone()).collect::<Vec<_>>();
        assert_eq!(sqls[0], "INSERT INTO `db0`.tshard_00002(didx, idx) VALUES (12, 22)");
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
        assert_eq!(res.results[0].target_sql, "SELECT idx from `db1`.tshard where idx = 3");

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 3)";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res.results[0].target_sql, "SELECT idx from db1.tshard where idx = 3 and idx = (SELECT idx from db1.tshard where idx = 3)");

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 4)";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.results.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
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
            res.results.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
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
            res.results.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
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
            res.results[0].target_sql,
            "SELECT idx from `db`.tshard_00000 where idx = 4".to_string()
        );

        let raw_sql = "SELECT idx from db.`tshard` where idx = 3 and idx = (SELECT idx from db.tshard where idx = 3)";
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res.results[0].target_sql, "SELECT idx from `db`.`tshard_00003` where idx = 3 and idx = (SELECT idx from `db`.tshard_00003 where idx = 3)");

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 4)";
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.results.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
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
            res.results.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
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
        assert_eq!(res.results[0].target_sql, "SELECT COUNT(price) AS PRICE_AVG_DERIVED_COUNT_00000, SUM(price) AS PRICE_AVG_DERIVED_SUM_00000 FROM `db`.tshard_00000 WHERE idx > 3");

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
        assert_eq!(res.results[0].target_sql, "SELECT COUNT(pbl) AS PBL_AVG_DERIVED_COUNT_00000, SUM(pbl) AS PBL_AVG_DERIVED_SUM_00000, COUNT(znl) AS ZNL_AVG_DERIVED_COUNT_00000, SUM(znl) AS ZNL_AVG_DERIVED_SUM_00000, COUNT(ngl) AS NGL_AVG_DERIVED_COUNT_00000, SUM(ngl) AS NGL_AVG_DERIVED_SUM_00000 FROM `db`.tshard_00000 WHERE idx > 3");

        let raw_sql = "SELECT AVG(pbl) FROM db.tshard WHERE idx = 3".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.clone(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res.results[0].target_sql, "SELECT COUNT(pbl) AS PBL_AVG_DERIVED_COUNT_00003, SUM(pbl) AS PBL_AVG_DERIVED_SUM_00003 FROM `db`.tshard_00003 WHERE idx = 3");

        let raw_sql = "SELECT AVG(znl) FROM db.tshard";
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), None, false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
            default_db: None,
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res.results[0].target_sql, "SELECT COUNT(znl) AS ZNL_AVG_DERIVED_COUNT_00000, SUM(znl) AS ZNL_AVG_DERIVED_SUM_00000 FROM `db`.tshard_00000");

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
        assert_eq!(res.results[0].target_sql, "SELECT * from `db`.tshard_00000 where znl > (SELECT COUNT(znl) AS ZNL_AVG_DERIVED_COUNT_00000, SUM(znl) AS ZNL_AVG_DERIVED_SUM_00000 from `db`.tshard_00000)")
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
            res.results.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
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
            res.results.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
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
            res.results.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
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
            res.results[0].target_sql,
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
            res.results[0].target_sql,
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
            res.results[0].target_sql,
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
            res.results[0].target_sql,
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
            res.results[0].target_sql,
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
        assert_eq!(res.results[0].target_sql, "SELECT id FROM `db`.tshard_00000 ORDER BY `id`");

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
            res.results[0].target_sql,
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
        assert_eq!(res.results[0].ds_idx.ds, DataSource::NodeGroup("ds001".to_string()));
    }
}
