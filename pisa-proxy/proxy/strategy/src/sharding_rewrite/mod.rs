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
mod generic_meta;

use std::vec;

use endpoint::endpoint::Endpoint;
use indexmap::IndexMap;
use mysql_parser::ast::{SqlStmt, Visitor, TableIdent};

use self::{meta::{
    FieldMeta, InsertValsMeta, RewriteMetaData, WhereMeta, WhereMetaRightDataType,
}, generic_meta::ShardingMeta};
use crate::{
    config::{Sharding, ShardingAlgorithmName, StrategyType},
    rewrite::{ShardingRewriteInput, ShardingRewriter},
};

pub trait CalcShardingIdx<I> {
    fn calc(self, algo: &ShardingAlgorithmName, id: I) -> Option<u64>;
}

impl CalcShardingIdx<u64> for u64 {
    fn calc(self, algo: &ShardingAlgorithmName, id: u64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => {
                Some(self.wrapping_rem(id))
            },
            _ => None,
        }
    }
}

impl CalcShardingIdx<i64> for i64 {
    fn calc(self, algo: &ShardingAlgorithmName, id: i64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => {
                Some(self.wrapping_rem(id) as u64)
            },
            _ => None
        }
    }
}

impl CalcShardingIdx<f64> for f64 {
    fn calc(self, algo: &ShardingAlgorithmName, id: f64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => {
                Some((self % id).round() as u64)
            }
            _ => None
        }
    }
}

#[derive(Debug)]
pub enum RewriteChange {
    DatabaseChange(DatabaseChange),
}

#[derive(Debug)]
pub struct DatabaseChange {
    pub span: mysql_parser::Span,
    pub target: String,
    pub shard_idx: u64,
    pub rule: Sharding,
}

#[derive(Debug)]
pub struct ShardingRewriteOutput {
    pub changes: Vec<RewriteChange>,
    pub target_sql: String,
    pub data_source: DataSource,
    pub sharding_column: Option<String>,
}

#[derive(Debug)]
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
    pub has_rw: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ShardingRewriteError {
    #[error("sharding column not found {0:?}")]
    ShardingColumnNotFound(String),

    #[error("parse str to u64 error {0:?}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("parse str to u64 error {0:?}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    #[error("calc mod error")]
    CalcModError,

    #[error("enpoint not found when using actual_datanodes")]
    EndpointNotFound,

    #[error("fields is empty")]
    FieldsIsEmpty,
}

struct ChangeInsertMeta {
    row_sharding_value: String,
    row_value_span: mysql_parser::Span,
}

enum StrategyTyp {
    Database,
    Table,
}

impl ShardingRewrite {
    pub fn new(rules: Vec<Sharding>, endpoints: Vec<Endpoint>, has_rw: bool) -> Self {
        ShardingRewrite { rules, raw_sql: "".to_string(), endpoints, has_rw }
    }

    pub fn get_endpoints(&self) -> &Vec<Endpoint> {
        &self.endpoints
    }

    pub fn set_raw_sql(&mut self, raw_sql: String) {
        self.raw_sql = raw_sql;
    }

    fn database_strategy(
        &self,
        meta: RewriteMetaData,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        let wheres = meta.get_wheres();

        if wheres.is_empty() {
            return Ok(self.database_strategy_iproduct(try_tables));
        }

        let wheres = Self::find_try_where(StrategyTyp::Database, &try_tables, wheres)?.into_iter().filter_map(|x| 
            match x {
                Some((idx, num, _)) => Some((idx, num)),
                None => None,
            }
        
        ).collect::<Vec<_>>();

        let expect_sum = wheres[0].1 as usize * wheres.len();
        let sum: usize = wheres.iter().map(|x| x.1).sum::<u64>() as usize;

        if expect_sum != sum {
            return Ok(self.database_strategy_iproduct(try_tables));
        }

        let would_changes: Vec<DatabaseChange> = try_tables
            .iter()
            .filter_map(|x| {
                let w = wheres.iter().find(|w| w.0 == x.0);
                if let Some(w) = w {
                    let target = self.change_table(x.2, &x.1.actual_datanodes[w.1 as usize], 0);
                    Some(DatabaseChange {
                        span: x.2.span,
                        shard_idx: w.1,
                        target,
                        rule: x.1.clone(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut target_sql = self.raw_sql.to_string();
        let mut offset = 0;
        let shard_idx = would_changes[0].shard_idx;
        let sharding_rule = &would_changes[0].rule.clone();

        let changes = would_changes
            .into_iter()
            .map(|x| {
                Self::change_sql(&mut target_sql, x.span, &x.target, offset);
                offset = x.target.len() - x.span.len();

                RewriteChange::DatabaseChange(x)
            })
            .collect::<Vec<_>>();

        let ep = self
            .endpoints
            .iter()
            .find(|e| e.name == sharding_rule.actual_datanodes[shard_idx as usize]).ok_or_else(|| ShardingRewriteError::EndpointNotFound)?;

        let sharding_column = if let Some(StrategyType::DatabaseStrategyConfig(strategy)) = &sharding_rule.database_strategy {
            strategy.database_sharding_column.to_string() 
        } else {
            unreachable!()
        };

        Ok(
            vec![
                ShardingRewriteOutput { 
                    changes, 
                    target_sql, 
                    data_source: DataSource::Endpoint(ep.clone()),
                    sharding_column: Some(sharding_column)
                }
            ]
        )
    }

    fn table_strategy(
        &self,
        meta: RewriteMetaData,
        try_tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        let wheres = meta.get_wheres();
        let inserts = meta.get_inserts();
        let fields = meta.get_fields();

        if !inserts.is_empty() {
            if fields.is_empty() {
                return Err(ShardingRewriteError::FieldsIsEmpty)
            }

            return self.change_insert_sql(try_tables, fields, inserts);
       }

        if wheres.is_empty() {
            return Ok(self.table_strategy_iproduct(try_tables.clone()));
        }

        let wheres = Self::find_try_where(StrategyTyp::Table, &try_tables, wheres)?.into_iter().filter_map(|x| {
            match x {
                Some((idx, num, _)) => Some((idx, num)),
                None => None,
            }
        }).collect::<Vec<_>>();

        let expect_sum = wheres[0].1 as usize * wheres.len();
        let sum: usize = wheres.iter().map(|x| x.1).sum::<u64>() as usize;

        if expect_sum != sum {
            return Ok(self.table_strategy_iproduct(try_tables));
        }

        let would_changes: Vec<DatabaseChange> = try_tables
            .iter()
            .filter_map(|x| {
                let w = wheres.iter().find(|w| w.0 == x.0);
                if let Some(w) = w {
                    let target = self.change_table(x.2, "", w.1);
                    Some(DatabaseChange {
                        span: x.2.span,
                        shard_idx: w.1,
                        target,
                        rule: x.1.clone(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut target_sql = self.raw_sql.clone();
        let mut offset = 0;
        let sharding_rule = &would_changes[0].rule.clone();

        let changes = would_changes
            .into_iter()
            .map(|x| {
                Self::change_sql(&mut target_sql, x.span, &x.target, offset);
                offset = x.target.len() - x.span.len();

                RewriteChange::DatabaseChange(x)
            })
            .collect::<Vec<_>>();

        let ep = self.endpoints.iter().find(|e| e.name == sharding_rule.actual_datanodes[0]).ok_or_else(|| ShardingRewriteError::EndpointNotFound)?;
        
        let sharding_column = if let Some(StrategyType::TableStrategyConfig(strategy)) = &sharding_rule.table_strategy {
            strategy.table_sharding_column.to_string()   
        } else {
            unreachable!()
        };

        Ok(
            vec![
                ShardingRewriteOutput { 
                    changes, 
                    target_sql: target_sql.to_string(), 
                    data_source: DataSource::Endpoint(ep.clone()),
                    sharding_column: Some(sharding_column),
                }
            ]
        )
    }

    fn find_table_rule<'a>(
        &self,
        tables: &'a IndexMap<u8, Vec<TableIdent>>,
    ) -> Vec<(u8, Sharding, &'a TableIdent)> {
        Self::find_table(tables, |idx, meta| {
            let rule = self.rules.iter().find(|x| x.table_name == meta.name);
            if let Some(rule) = rule {
                if meta.schema.is_some() {
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
        calc_fn: F,
    ) -> Vec<(u8, Sharding, &TableIdent)>
    where
        F: Fn(u8, &TableIdent) -> (u8, Option<Sharding>, bool),
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

    fn find_try_where<'a>(strategy_typ: StrategyTyp, try_tables: &[(u8, Sharding, &TableIdent)], wheres: &'a IndexMap<u8, Vec<WhereMeta>>) -> Result<Vec<Option<(u8, u64, &'a WhereMeta)>>, ShardingRewriteError> {
        Self::find_where(wheres, |query_id, meta| {
            let rule = try_tables.iter().find(|x| x.0 == query_id);
            if let Some(rule) = rule {
                let (sharding_column, algo, sharding_count) = match strategy_typ {
                    StrategyTyp::Database => {
                        (rule.1.get_sharding_column().0.unwrap(), rule.1.get_algo().0.unwrap(), rule.1.get_sharding_count().0.unwrap())
                    }

                    StrategyTyp::Table => {
                        (rule.1.get_sharding_column().1.unwrap(), rule.1.get_algo().1.unwrap(), rule.1.get_sharding_count().1.unwrap())
                    }
                };

                return Self::parse_where(
                    meta,
                    algo,
                    sharding_count,
                    query_id,
                    sharding_column,
                );
            }

            Ok(None)
        })
    }

    fn find_where<F>(
        wheres: &IndexMap<u8, Vec<WhereMeta>>,
        calc_fn: F,
    ) -> Result<Vec<Option<(u8, u64, &WhereMeta)>>, ShardingRewriteError>
    where
        F: Fn(u8, &WhereMeta) -> Result<Option<(u8, u64, &WhereMeta)>, ShardingRewriteError>,
    {
        wheres
            .iter()
            .filter_map(|(k, v)| {
                Some(
                    v.iter()
                        .filter_map(|x| {
                            let res = calc_fn(*k, x);
                            match res {
                                Ok(None) => None,
                                _ => Some(res),
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect::<Result<Vec<_>, _>>()
    }

    fn parse_where<'b>(
        meta: &'b WhereMeta,
        algo: &ShardingAlgorithmName,
        sharding_count: u64,
        query_id: u8,
        sharding_column: &str,
    ) -> Result<Option<(u8, u64, &'b WhereMeta)>, ShardingRewriteError> {
        match meta {
            WhereMeta::BinaryExpr { left, right } => {
                if left != sharding_column {
                    return Ok(None);
                }

                let num = match right {
                    WhereMetaRightDataType::Num(val) => {
                        let val = val.parse::<u64>()?;
                        val.calc(algo, sharding_count)
                    },

                    WhereMetaRightDataType::SignedNum(val) => {
                        let val = val.parse::<i64>()?;
                        val.calc(algo, sharding_count as i64)
                    },

                    WhereMetaRightDataType::FloatNum(val) => {
                        let val = val.parse::<f64>()?;
                        val.calc(algo, sharding_count as f64)
                    }
                    _ => return Ok(None),
                };

                if let Some(num) = num {
                    return Ok(Some((query_id, num, meta)));
                }

                Ok(None)
            }
        }
    }

    fn change_insert_sql(&self, try_tables: Vec<(u8, Sharding, &TableIdent)>, fields: &IndexMap<u8, Vec<FieldMeta>>,inserts: &IndexMap<u8, Vec<InsertValsMeta>>) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        let outputs = try_tables.into_iter().map(|(query_id, rule, table)| {
            let strategy = if let Some(strategy) = &rule.table_strategy {
                if let StrategyType::TableStrategyConfig(config) = strategy {
                    config
                } else {
                    unreachable!()
                }
             } else {
                unreachable!()
             };

            self.change_insert_sql_inner(
                &rule,
                &table,
                &inserts.get(&query_id).unwrap(),
                &fields.get(&query_id).unwrap(),
                &strategy.table_sharding_column,
                &strategy.table_sharding_algorithm_name,
                *&strategy.sharding_count as u64,
            )
        }).collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect::<Vec<_>>();

        Ok(outputs)
    }

    fn change_insert_sql_inner(
        &self,
        rule: &Sharding,
        table: &TableIdent,
        inserts: &[InsertValsMeta],
        fields: &[FieldMeta],
        sharding_column: &str,
        algo: &ShardingAlgorithmName,
        sharding_count: u64,
    ) -> Result<Vec<ShardingRewriteOutput>, ShardingRewriteError> {
        let changes = Self::change_insert(inserts, fields, sharding_column, algo, sharding_count)?;

        let row_start_idx = changes[0].1.start();
        let row_prefix_text = &self.raw_sql[0..row_start_idx];

        let mut change_rows = IndexMap::<u64, String>::new();
        for change in changes.iter() {
            let target_table = self.change_table(table, "", change.0);
            let mut target_row_prefix_text = row_prefix_text.to_string();
            Self::change_sql(&mut target_row_prefix_text, table.span, &target_table, 0);
            let mut row_text = self.raw_sql[change.1.start()..change.1.end()].to_string();
            row_text.push_str(", ");
            change_rows.entry(change.0).or_insert(target_row_prefix_text).push_str(&row_text);
        }

        let ep = self.endpoints.iter().find(|e| e.name == rule.actual_datanodes[0]).ok_or_else(|| ShardingRewriteError::EndpointNotFound)?;

        let outputs = change_rows.into_iter().map(|(_, v)| {
            ShardingRewriteOutput {
                changes: vec![],
                target_sql: v.trim_end_matches(", ").to_string(),
                data_source: DataSource::Endpoint(ep.clone()),
                sharding_column: Some(sharding_column.to_string()),
            }

        }).collect::<Vec<ShardingRewriteOutput>>();

        Ok(outputs)
    }

    fn change_insert(
        inserts: &[InsertValsMeta],
        fields: &[FieldMeta],
        sharding_column: &str,
        algo: &ShardingAlgorithmName,
        sharding_count: u64,
    ) -> Result<Vec<(u64, mysql_parser::Span)>, ShardingRewriteError> {
        let insert_values = Self::find_inserts(inserts, fields, sharding_column)?;
        
        let mut changes = vec![];
        for value in insert_values.iter() {
            let shard_value =
                value.row_sharding_value.parse::<u64>().map_err(ShardingRewriteError::from)?;
            let shard_value = shard_value
                .calc(algo, sharding_count)
                .ok_or_else(|| ShardingRewriteError::CalcModError)?;

            changes.push((shard_value, value.row_value_span))
        }

        Ok(changes)
    }

    fn find_inserts(
        inserts: &[InsertValsMeta],
        fields: &[FieldMeta],
        sharding_column: &str,
    ) -> Result<Vec<ChangeInsertMeta>, ShardingRewriteError> {
        let field = fields
            .iter()
            .enumerate()
            .find_map(|x| {
                if let FieldMeta::Ident { span, name } = x.1 {
                    if name == sharding_column {
                        return Some((x.0, *span, name.clone()));
                    }
                }

                None
            })
            .ok_or_else(|| {
                ShardingRewriteError::ShardingColumnNotFound(sharding_column.to_string())
            })?;

        Ok(inserts
            .iter()
            .enumerate()
            .map(|(_, insert)| ChangeInsertMeta {
                row_sharding_value: insert.values[field.0].value.clone(),
                row_value_span: insert.span.clone(),
            })
            .collect::<Vec<ChangeInsertMeta>>())
    }

    fn database_strategy_iproduct(
        &self,
        tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Vec<ShardingRewriteOutput> {
        let mut output = vec![];
        let mut group_changes = IndexMap::<usize, Vec<DatabaseChange>>::new();
        let mut sharding_column = None;

        for t in tables.iter() {
            if let StrategyType::DatabaseStrategyConfig(config) = &t.1.database_strategy.as_ref().unwrap() {
                sharding_column = Some(config.database_sharding_column.clone())
            }

            for (idx, node) in t.1.actual_datanodes.iter().enumerate() {
                let target = self.change_table(t.2, node, 0);

                let change = DatabaseChange {
                    span: t.2.span,
                    target,
                    shard_idx: idx as u64,
                    rule: t.1.clone(),
                };

                group_changes.entry(idx).or_insert(vec![]).push(change);
            }
        }

        for (group, changes) in group_changes.into_iter() {
            let mut offset = 0;
            let mut target_sql = self.raw_sql.to_string();
            for change in changes.iter() {
                Self::change_sql(&mut target_sql, change.span, &change.target, offset);
                offset = change.target.len() - change.span.len();
            }

            let ep = self.endpoints[group].clone();
            output.push(ShardingRewriteOutput {
                changes: changes.into_iter().map(|x| RewriteChange::DatabaseChange(x)).collect(),
                target_sql,
                data_source: DataSource::Endpoint(ep),
                sharding_column: sharding_column.clone(),
            })
        }
        output
    }

    fn table_strategy_iproduct(
        &self,
        tables: Vec<(u8, Sharding, &TableIdent)>,
    ) -> Vec<ShardingRewriteOutput> {
        let mut output = vec![];
        let mut group_changes = IndexMap::<usize, Vec<DatabaseChange>>::new();
        let mut sharding_column = None; 

        for t in tables.iter() {
            match t.1.table_strategy.as_ref().unwrap() {
                StrategyType::TableStrategyConfig(config) => {
                    sharding_column = Some(config.table_sharding_column.clone());
                    for idx in 0..config.sharding_count as u64 {
                        let target = self.change_table(t.2, "", idx);

                        let change = DatabaseChange {
                            span: t.2.span,
                            target,
                            shard_idx: idx as u64,
                            rule: t.1.clone(),
                        };

                        group_changes.entry(idx as usize).or_insert(vec![]).push(change);
                    }
                }
                _ => unreachable!(),
            }
        }

        for (_, changes) in group_changes.into_iter() {
            let mut offset = 0;
            let mut target_sql = self.raw_sql.clone();
            for change in changes.iter() {
                Self::change_sql(&mut target_sql, change.span, &change.target, offset);
                offset = change.target.len() - change.span.len();
            }

            let ep = self.endpoints[0].clone();
            output.push(ShardingRewriteOutput {
                changes: changes.into_iter().map(|x| RewriteChange::DatabaseChange(x)).collect(),
                target_sql: target_sql.to_string(),
                data_source: DataSource::Endpoint(ep),
                sharding_column: sharding_column.clone(),
            })
        }

        output
    }

    fn change_table(&self, table: &TableIdent, actual_node: &str, table_idx: u64) -> String {
        let schema = table.schema.as_ref().unwrap();
        let mut target = String::with_capacity(schema.len());

        if actual_node.len() == 0 {
            target.push_str(schema);
            target.push('.');
            target.push_str(&format!("{}_{:05}", &table.name, table_idx));
        } else {
            target.push_str(actual_node);
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
        let mut meta = RewriteMetaData::default();
        let _ = ast.visit(&mut meta);
        meta
    }
}

impl ShardingRewriter<ShardingRewriteInput> for ShardingRewrite {
    type Output = Result<Vec<ShardingRewriteOutput>, ShardingRewriteError>;
    fn rewrite(&mut self, mut input: ShardingRewriteInput) -> Self::Output {
        self.set_raw_sql(input.raw_sql);
        let meta = self.get_meta(&mut input.ast);
        let tables = meta.get_tables().clone();
        let try_tables = self.find_table_rule(&tables);
        if try_tables.is_empty() {
            return Ok(vec![]);
        }

        // Strategy according to first element of `try_tables`.
        let rule = &try_tables[0].1;

        if rule.database_strategy.is_some() {
            return self.database_strategy(meta, try_tables)
        } 

        if rule.table_strategy.is_some() {
            return self.table_strategy(meta, try_tables)
        }

        return Ok(vec![])
    }
}

#[cfg(test)]
mod test {
    use endpoint::endpoint::Endpoint;
    use mysql_parser::parser::Parser;

    use super::ShardingRewrite;
    use crate::{config::{DatabaseStrategyConfig, Sharding, ShardingAlgorithmName, StrategyType}, rewrite::{ShardingRewriteInput, ShardingRewriter}};

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
                    db: String::from("db"),
                    user: String::from("user"),
                    password: String::from("password"),
                    addr: String::from("127.0.0.1"),
                },
                Endpoint {
                    weight: 1,
                    name: String::from("ds1"),
                    db: String::from("db"),
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
                        datanode_name: "ds001".to_string(),
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
    fn test_database_sharding_strategy() {
        let config = get_database_sharding_config();
        let raw_sql = "SELECT idx from db.tshard where idx = 3";
        let parser = Parser::new();
        let ast = parser.parse(raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from ds1.tshard where idx = 3");

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 3)";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from ds1.tshard where idx = 3 and idx = (SELECT idx from ds1.tshard where idx = 3)");

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 4)";
        let ast = parser.parse(raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "SELECT idx from ds0.tshard where idx = 3 and idx = (SELECT idx from ds0.tshard where idx = 4)", 
                "SELECT idx from ds1.tshard where idx = 3 and idx = (SELECT idx from ds1.tshard where idx = 4)", 
            ],
        )
    }

    #[test]
    fn test_table_sharding_strategy() {
        let config = get_table_sharding_config();
        let raw_sql = "SELECT idx from db.tshard where idx > 3".to_string();
        let parser = Parser::new();
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), false);
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "SELECT idx from db.tshard_00000 where idx > 3",
                "SELECT idx from db.tshard_00001 where idx > 3",
                "SELECT idx from db.tshard_00002 where idx > 3",
                "SELECT idx from db.tshard_00003 where idx > 3",
            ],
        );

        let raw_sql = "SELECT idx from db.tshard where idx = 4".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from db.tshard_00000 where idx = 4".to_string());

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 3)".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from db.tshard_00003 where idx = 3 and idx = (SELECT idx from db.tshard_00003 where idx = 3)".to_string());

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 4)".to_string();
        let ast = parser.parse(&raw_sql).unwrap();
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "SELECT idx from db.tshard_00000 where idx = 3 and idx = (SELECT idx from db.tshard_00000 where idx = 4)",
                "SELECT idx from db.tshard_00001 where idx = 3 and idx = (SELECT idx from db.tshard_00001 where idx = 4)",
                "SELECT idx from db.tshard_00002 where idx = 3 and idx = (SELECT idx from db.tshard_00002 where idx = 4)",
                "SELECT idx from db.tshard_00003 where idx = 3 and idx = (SELECT idx from db.tshard_00003 where idx = 4)",
            ],
        );
    }

    #[test]
    fn test_table_sharding_strategy_insert() {
        let config = get_table_sharding_config();
        let raw_sql = "INSERT INTO db.tshard(idx) VALUES (12), (13), (16)".to_string();
        let parser = Parser::new();
        let ast = parser.parse(&raw_sql).unwrap();
        let mut sr = ShardingRewrite::new(config.0.clone(), config.1.clone(), false);
        let input = ShardingRewriteInput {
            raw_sql: raw_sql.to_string(),
            ast: ast[0].clone(),
        };
        let res = sr.rewrite(input).unwrap();

        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "INSERT INTO db.tshard_00000(idx) VALUES (12), (16)",
                "INSERT INTO db.tshard_00001(idx) VALUES (13)",
            ],
        );
    }
}
