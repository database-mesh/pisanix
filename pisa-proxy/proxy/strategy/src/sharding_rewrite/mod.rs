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

use std::{vec, collections::HashMap};

use endpoint::endpoint::Endpoint;
use indexmap::IndexMap;
use mysql_parser::ast::{SqlStmt, Visitor, TableIdent};

use crate::{config::{Sharding, StrategyType, ShardingAlgorithmName}, rewrite::ShardingRewriter};

use self::meta::{RewriteMetaData, WhereMeta, WhereMetaRightDataType};

pub trait CalcShardingIdx<I> {
    fn calc(self, algo: &ShardingAlgorithmName, id: I) -> Option<u64>;
}

impl CalcShardingIdx<u64> for u64 {
    fn calc(self, algo: &ShardingAlgorithmName, id: u64) -> Option<u64> {
        match algo {
            ShardingAlgorithmName::Mod => {
                Some(self.wrapping_rem(id))
            },

            _ => None
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
            },

            _ => None
        }
    }
}

#[derive(Debug)]
pub enum RewriteChange {
    DatabaseChange(DatabaseChange)
}

#[derive(Debug)]
pub struct DatabaseChange {
    pub span: mysql_parser::Span,
    pub target: String,
    pub shard_idx: u64,
}

#[derive(Debug)]
pub struct ShardingRewriteOutput {
    pub changes: Vec<RewriteChange>,
    pub target_sql: String,
    pub endpoint: Endpoint,
}


#[derive(Debug)]
pub struct ShardingRewrite<'a> {
    rules: Vec<Sharding>,
    // Raw sql
    raw_sql: &'a str,

    // Endpoints
    endpoints: Vec<Endpoint>,
}



impl<'a> ShardingRewrite<'a> {
    pub fn new(rules: Vec<Sharding>, raw_sql: &'a str, endpoints: Vec<Endpoint>) -> Self {
        ShardingRewrite { rules, raw_sql, endpoints }
    }

    fn parse_rule(&self) {
        //self.rule.database_table_strategy.unwrap().

    }

    fn table_strategy(&self, rule: &Sharding, mut meta: RewriteMetaData) -> Result<Vec<ShardingRewriteOutput>, Box<dyn std::error::Error>> {
        let strategy = match &rule.table_strategy {
            Some(StrategyType::TableStrategyConfig(strategy)) => {
                strategy
            }

            _ => return Ok(vec![])
        };

        let algo = &strategy.table_sharding_algorithm_name;
        let tables = meta.get_tables();
        let try_tables = Self::find_table(&tables, |idx, meta| {
            (idx, meta.name == rule.table_name && meta.schema.is_some())
        });


        let wheres = meta.get_wheres();
        if try_tables.is_empty() {
            return Ok(vec![]);
        }

        if wheres.is_empty() {
            //return Ok(self.database_strategy_iproduct(rule, try_tables));
            return Ok(vec![])
        }
        

        Ok(vec![])
    }

    fn database_strategy(&self, rule: &Sharding, meta: RewriteMetaData) -> Result<Vec<ShardingRewriteOutput>, Box<dyn std::error::Error>>{
        let strategy = match &rule.database_strategy {
            Some(StrategyType::DatabaseStrategyConfig(strategy)) => {
                strategy
            }

            _ => return Ok(vec![]),
        };

        let algo = &strategy.database_sharding_algorithm_name;

        let tables = meta.get_tables();

        let try_tables = Self::find_table(&tables, |idx, meta| {
            (idx, meta.name == rule.table_name && meta.schema.is_some())
        });


        let actual_nodes_length  = rule.actual_datanodes.len() as u64;
        let wheres = meta.get_wheres();
        println!("wheres {:?}", wheres);

        if try_tables.is_empty() {
            return Ok(vec![]);
        }

        if wheres.is_empty() {
            return Ok(self.database_strategy_iproduct(rule, try_tables));
        }

        let try_where = Self::find_where(wheres, |query_id, meta| {
            Self::parse_where(meta, algo, actual_nodes_length, query_id, &strategy.database_sharding_column)    
        });

        let mut wheres = Vec::with_capacity(try_where.len());
        for v in try_where {
            let v = v?;
            match v {
                Some((idx, num, _)) => wheres.push((idx, num)),
                None => continue
            }
        }

        let expect_sum = wheres[0].1 as usize * wheres.len();
        let sum: usize = wheres.iter().map(|x| x.1).sum::<u64>() as usize;
        
        if expect_sum != sum {
            return Ok(self.database_strategy_iproduct(rule, try_tables));
        }

        println!("try_tables {:?}", try_tables);
        println!("wheres {:?}", wheres);

        let would_changes:Vec<DatabaseChange> = try_tables.iter().filter_map(|x| {
            let w = wheres.iter().find(|w| w.0 == x.0);
            if let Some(w) = w {
                let target = self.change_table(x.1, &rule.actual_datanodes[w.1 as usize]);
                Some(
                    DatabaseChange {
                        span: x.1.span,
                        shard_idx: w.1,
                        target,
                    }
                )
            } else {
                None
            }
            
        }).collect::<Vec<_>>();

        let mut target_sql = self.raw_sql.to_string();
        let mut offset  = 0;
        let shard_idx = would_changes[0].shard_idx;

        let changes = would_changes.into_iter().map(|x| { 
            Self::change_sql(&mut target_sql, x.span, &x.target, offset);
            offset = x.target.len() - x.span.len();

            RewriteChange::DatabaseChange(x)
        }).collect::<Vec<_>>();

        let mut ep = self.endpoints.iter().find(|e| e.name == rule.actual_datanodes[shard_idx as usize]);
        if ep.is_none() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "endpoint not found")))
        }

        Ok(
            vec![
                ShardingRewriteOutput { 
                    changes, 
                    target_sql, 
                    endpoint: ep.take().unwrap().clone(), 
                }
            ]
        )
    }

    fn find_table<F>(tables: &IndexMap<u8, Vec<TableIdent>>, calc_fn: F) -> Vec<(u8, &TableIdent)> 
    where F: Fn(u8, &TableIdent) -> (u8, bool)
    {
        tables.iter().filter_map(|(k, v)| {
            let res = v.iter().filter_map(|x| {
                    let res = calc_fn(*k, x);
                    if res.1 {
                        Some((res.0, x))
                    } else {
                        None
                    }
                }).collect::<Vec<_>>();
            
            if res.is_empty() {
                None
            } else {
                Some(res)
            }
        }).flatten().collect::<Vec<_>>()
    }

    fn find_where<F>(wheres: &IndexMap<u8, Vec<WhereMeta>>, calc_fn: F) -> Vec<Result<Option<(u8, u64, &WhereMeta)>, Box<dyn std::error::Error>>>
    where F: Fn(u8, &WhereMeta) -> Result<Option<(u8, u64, &WhereMeta)>, Box<dyn std::error::Error>>
    {
        wheres.iter().filter_map(|(k, v)| 
            Some(
                v.iter().filter_map(|x| {
                    let res = calc_fn(*k, x);
                    match res {
                        Ok(None) => None,
                        _ => Some(res),
                    }
                }).collect::<Vec<_>>()
            )
        ).flatten().collect::<Vec<_>>()
    }

    fn parse_where<'b>(meta: &'b WhereMeta, algo: &ShardingAlgorithmName, actual_nodes_length: u64, query_id: u8, sharding_column: &str) -> Result<Option<(u8, u64, &'b WhereMeta)>, Box<dyn std::error::Error>> {
        match meta {
            WhereMeta::BinaryExpr { left, right }  => {
                if left != sharding_column {
                    return Ok(None);
                }

                let num = match right {
                    WhereMetaRightDataType::Num(val) => {
                        let val = val.parse::<u64>()?;
                        val.calc(algo, actual_nodes_length)
                    },

                    WhereMetaRightDataType::SignedNum(val) => {
                        let val = val.parse::<i64>()?;
                        val.calc(algo, actual_nodes_length as i64)
                    },

                    WhereMetaRightDataType::FloatNum(val) => {
                        let val = val.parse::<f64>()?;
                        val.calc(algo, actual_nodes_length as f64)
                    }
                    _ => return Ok(None)
                };

                if let Some(num) = num {
                    return Ok(Some((query_id, num, meta)))
                }

                Ok(None)
            }                      
        }
    }

    fn database_strategy_iproduct(&self, rule: &Sharding, tables: Vec<(u8, &TableIdent)>) -> Vec<ShardingRewriteOutput> {
        //let products = itertools::iproduct!(0..rule.actual_datanodes.len(), tables.iter());
        let mut output = vec![];

        for (idx, node) in rule.actual_datanodes.iter().enumerate() {
            let mut changes = Vec::with_capacity(tables.len());
            let mut offset: usize = 0;
            let mut target_sql = self.raw_sql.to_string();

            for t in tables.iter() {
                let target = self.change_table(t.1, node);
                Self::change_sql(&mut target_sql, t.1.span, &target, offset);
                offset = target.len() - t.1.span.len();

                let change = DatabaseChange {
                    span: t.1.span,
                    target,
                    shard_idx: idx as u64,
                };
                changes.push(RewriteChange::DatabaseChange(change));
            }
            
            let endpoint = self.endpoints[idx].clone();

            output.push(ShardingRewriteOutput {
                changes,
                target_sql,
                endpoint,
            })
        }

        output
    }

    fn change_table(&self, table: &TableIdent, actual_node: &str) -> String {
        let schema = table.schema.as_ref().unwrap();
        let mut target = String::with_capacity(schema.len());
        target.push_str(actual_node);
        target.push('.'); 
        target.push_str(&table.name);
        //target.push(' ');
        target
    }

    fn change_sql(target_sql: &mut String, span: mysql_parser::Span, target: &str, offset: usize) {
        for _ in 0 .. span.len() {
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

impl<'a> ShardingRewriter<SqlStmt> for ShardingRewrite<'a> {
    fn rewrite(&mut self, mut ast: SqlStmt) {
       let meta = self.get_meta(&mut ast);

    }
}



#[cfg(test)]
mod test {
    use endpoint::endpoint::Endpoint;
    use mysql_parser::parser::Parser;

    use crate::config::{Sharding, DatabaseStrategyConfig, StrategyType, ShardingAlgorithmName};

    use super::ShardingRewrite;

    fn get_database_sharding_config() -> (Vec<Sharding>, Vec<Endpoint>) {
        (
            vec![
                Sharding { 
                    table_name: "tshard".to_string(), 
                    actual_datanodes: vec!["ds0".to_string(), "ds1".to_string()], 
                    binding_tables: None, 
                    broadcast_tables: None, 
                    database_strategy: Some(
                        StrategyType::DatabaseStrategyConfig(
                            DatabaseStrategyConfig { 
                                database_sharding_algorithm_name: ShardingAlgorithmName::Mod, 
                                database_sharding_column: "idx".to_string()
                            }
                        )
                    ), 
                    table_strategy: None, 
                    database_table_strategy: None, 
                }
            ],
        
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
            ]
        )
    }

    #[test]
    fn test_database_sharding_strategy() {
        let config = get_database_sharding_config();
        let raw_sql = "SELECT idx from db.tshard where idx = 3";
        let parser = Parser::new();
        let mut ast = parser.parse(raw_sql).unwrap();
        let sr = ShardingRewrite::new(config.0.clone(), raw_sql, config.1.clone());
        let meta = sr.get_meta(&mut ast[0]);

        let res = sr.database_strategy(&config.0[0], meta).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from ds1.tshard where idx = 3");
        println!("{:?}", res);

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 3)";
        let sr = ShardingRewrite::new(config.0.clone(), raw_sql, config.1.clone());
        let mut ast = parser.parse(raw_sql).unwrap();
        let meta = sr.get_meta(&mut ast[0]);
        let res = sr.database_strategy(&config.0[0], meta).unwrap();
        assert_eq!(res[0].target_sql, "SELECT idx from ds1.tshard where idx = 3 and idx = (SELECT idx from ds1.tshard where idx = 3)");

        let raw_sql = "SELECT idx from db.tshard where idx = 3 and idx = (SELECT idx from db.tshard where idx = 4)";
        let sr = ShardingRewrite::new(config.0.clone(), raw_sql, config.1);
        let mut ast = parser.parse(raw_sql).unwrap();
        let meta = sr.get_meta(&mut ast[0]);
        let res = sr.database_strategy(&config.0[0], meta).unwrap();
        println!("res {:#?}", res);
        assert_eq!(
            res.into_iter().map(|x| x.target_sql).collect::<Vec<_>>(),
            vec![
                "SELECT idx from ds0.tshard where idx = 3 and idx = (SELECT idx from ds0.tshard where idx = 4)", 
                "SELECT idx from ds1.tshard where idx = 3 and idx = (SELECT idx from ds1.tshard where idx = 4)", 
            ],
        )
    }
}