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

use indexmap::IndexMap;
use aho_corasick::{AhoCorasickBuilder, AhoCorasick};
use mysql_parser::ast::*;

use super::ShardingRewrite;

#[derive(Debug, Clone)]
enum ScanState {
    Empty,
    TableName,
    Field,
    Order,
    Group,
    Where(Vec<String>),
    OnCond(Vec<OnCond>),
    Avg(mysql_parser::Span, bool),
    InsertRowValue(Vec<Vec<InsertValue>>)
}

type TableMeta = TableIdent;

#[derive(Debug)]
pub enum FieldMeta {
    Ident { span: mysql_parser::Span, name: String },
    TableWild(TableWild)
}

#[cfg(test)]
impl FieldMeta {
    fn to_string(&self) -> String {
        match self {
            Self::Ident { span: _, name } => name.clone(),
            Self::TableWild(val) => val.format()
        }
    }
}

#[derive(Debug)]
pub struct OrderMeta {
    pub span: mysql_parser::Span,
    pub name: String,
}

#[derive(Debug)]
pub struct GroupMeta {
    pub span: mysql_parser::Span,
    pub name: String,
}

#[derive(Debug)]
pub struct LimitMeta {
    pub span: mysql_parser::Span,
    pub name: String,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum WhereMeta {
    // left: Field right: value
    BinaryExpr { left: String, right: WhereMetaRightDataType }
}

#[derive(Debug)]
pub enum WhereMetaRightDataType {
    String(String),
    Num(String),
    SignedNum(String),
    FloatNum(String),
}

#[derive(Debug)]
pub struct JoinOnCond {
    pub left: OnCond,
    pub right: OnCond,
}

#[derive(Debug, Clone)]
pub struct OnCond {
    pub span: mysql_parser::Span,
    pub name: String,
}

#[derive(Debug)]
pub struct AvgMeta {
    pub span: mysql_parser::Span,
    pub name: String,
    pub distinct: bool,
}

pub type InsertValsMeta = Vec<Vec<InsertValue>>;

#[derive(Debug, Clone)]
pub struct InsertValue {
    pub span: mysql_parser::Span,
    pub value: String,
}

#[derive(Debug)]
struct QueryRequire {
    query_id: u8,
    expr_token: Option<String>,
    require_query_id: u8,
}

#[derive(Debug)]
pub struct RewriteMetaData {
    tables: IndexMap<u8, Vec<TableMeta>>,
    fields: IndexMap<u8, Vec<FieldMeta>>,
    orders: IndexMap<u8, Vec<OrderMeta>>,
    groups: IndexMap<u8, Vec<GroupMeta>>,
    limits: IndexMap<u8, Vec<LimitMeta>>,
    wheres: IndexMap<u8, Vec<WhereMeta>>,
    on_conds: IndexMap<u8, Vec<JoinOnCond>>,
    avgs: IndexMap<u8, Vec<AvgMeta>>,
    inserts: IndexMap<u8, Vec<InsertValsMeta>>,
    query_id: u8,
    state: ScanState,
    requires: Vec<QueryRequire>,
    prev_expr_type: Option<String>,
    // Replace "`" to ""
    ac: AhoCorasick,
}

impl Default for RewriteMetaData {
    fn default() -> Self {
        Self {
            query_id: 0,
            tables: IndexMap::new(),
            fields: IndexMap::new(),
            orders: IndexMap::new(),
            groups: IndexMap::new(),
            limits: IndexMap::new(),
            wheres: IndexMap::new(),
            on_conds: IndexMap::new(),
            avgs: IndexMap::new(),
            inserts: IndexMap::new(),
            state: ScanState::Empty,
            requires: vec![],
            prev_expr_type: None,
            ac: AhoCorasickBuilder::new().build(&["`"]),
        }
    }
}

macro_rules! gen_push_func {
    ($($push_func_name:ident, $get_func_name: ident, $meta_typ: tt, $field: tt), *) => {
        impl RewriteMetaData {
            $(
                fn $push_func_name(&mut self, meta: $meta_typ) {
                    self.$field.entry(self.query_id).or_insert(vec![]).push(meta);
                }

                pub fn $get_func_name(&self) -> &IndexMap<u8, Vec<$meta_typ>> {
                    &self.$field
                }
            )*
        }
    };
}

gen_push_func!(push_table, get_tables, TableMeta, tables);
gen_push_func!(push_where, get_wheres, WhereMeta, wheres);
gen_push_func!(push_field, get_fields, FieldMeta, fields);
gen_push_func!(push_limit, get_limits, LimitMeta, limits);
gen_push_func!(push_group, get_groups, GroupMeta, groups);
gen_push_func!(push_order, get_orders, OrderMeta, orders);
gen_push_func!(push_on_cond, get_on_conds, JoinOnCond, on_conds);
gen_push_func!(push_avg, get_avgs, AvgMeta, avgs);
gen_push_func!(push_insert_value, get_inserts, InsertValsMeta, inserts);


impl Transformer for RewriteMetaData {
    fn trans(&mut self, node: &mut Node<'_>) -> bool {
        match node {
            Node::Query(_q) => {
                self.query_id += 1;
            }

            Node::SubQuery(q) => {
                q.visit(self);
                self.query_id -= 1;
                return true;
            }

            Node::SingleTable(t) => {
                self.push_table(t.table_name.clone());
            }

            Node::FromClause(_) => {
                self.state = ScanState::TableName;
            }

            Node::WhereClause(_) => {
                self.state = ScanState::Where(vec![]);
            }

            Node::TableRef(..) => {
                self.state = ScanState::OnCond(vec![]);
            }

            Node::Items(t) => {
                self.state = ScanState::Field;
                match t {
                    Items::Items(t) => {
                        for i in t {
                            match i {
                                Item::TableWild(val) => {
                                    self.push_field(FieldMeta::TableWild(val.clone()))
                                }
                                _ => {}
                            }
                        }
                    }

                    _ => {}
                }
            }

            Node::OrderClause(_) => self.state = ScanState::Order,

            Node::GroupClause(_) => self.state = ScanState::Group,

            Node::LimitClause(val) => {
                for i in val.opts.iter() {
                    self.push_limit( LimitMeta { span: i.span, name: i.opt.clone() })
                }
            }

            Node::InsertStmt(val) => {
                self.push_table( val.table_name.clone() );
            }

            Node::RowValue(_) => {
                if let ScanState::InsertRowValue(args) = &mut self.state {
                    args.push(vec![])
                } else {
                    self.state = ScanState::InsertRowValue(vec![vec![]])
                }
            }

            Node::InsertIdent(val) => {
                match val {
                    InsertIdent::Ident(val) => {
                        if let Value::Ident { span, value, quoted } = val {
                            let name = if *quoted {
                                self.ac.replace_all(value, &[""])
                            } else {
                                value.clone()
                            };
                            self.push_field(FieldMeta::Ident { span: *span, name })
                        }
                    },
                    InsertIdent::TableWild(val) => {
                        self.push_field(FieldMeta::TableWild(val.clone()))
                    }
                }
            }

            Node::Expr(e) => match e {
                Expr::SimpleIdentExpr(Value::Ident { span, value, .. }) => match &mut self.state {
                    ScanState::Field => self.push_field(FieldMeta::Ident { span: *span, name: value.to_string() }),
                    ScanState::Order => self.push_order(OrderMeta { span: *span, name: value.to_string() }),
                    ScanState::Group => self.push_group(GroupMeta { span: *span, name: value.to_string() }),
                    ScanState::Where(args) => {
                        args.push(value.to_string());
                    }
                    ScanState::Avg(arg, distinct) => {
                        let arg = arg.clone();
                        let distinct = *distinct;
                        let meta = AvgMeta {
                            span: arg,
                            name: value.to_string(),
                            distinct,
                        };
                        self.push_avg(meta);
                        self.state = ScanState::Empty;
                        
                    }
                    _ => {}
                }

                Expr::SimpleIdentExpr(e) => {
                    let state = self.state.clone();

                    if let Value::TableIdent { span, .. } = e {
                        match state {
                            ScanState::OnCond(mut args) => {
                                if args.len() < 1 {
                                    args.push( OnCond { span: *span, name: e.format() });
                                    self.state = ScanState::OnCond(args);
                                    return true
                                }

                                self.push_on_cond(JoinOnCond { left: args[0].clone(), right: OnCond { span: *span, name: e.format() }} );
                                self.state = ScanState::Empty;

                            },

                            _ => {}
                        }                  

                    }
                    
                }

                Expr::BinaryOperationExpr { span:_, operator, left:_, right} => {
                    match &mut self.state {
                        ScanState::Where(args) => {
                            if let Expr::LiteralExpr(_) = **right {
                                let op = operator.format();
                                if op.as_str() == "=" {
                                    args.push(op);
                                }
                            }
                            
                            
                        },

                        _ => {}
                    }
                }

                Expr::AggExpr(e) => {
                    if e.name == AggFuncName::Avg {
                        self.state = ScanState::Avg(e.span, e.distinct);
                    }
                }

                Expr::InExpr { .. } => {
                    self.prev_expr_type = Some("In".to_string());
                }

                Expr::SubQueryExpr(_) => {
                    self.requires.push(QueryRequire {
                        query_id: self.query_id,
                        expr_token: self.prev_expr_type.clone(),
                        require_query_id: self.query_id + 1,
                    });

                    self.prev_expr_type = None
                }

                Expr::LiteralExpr(Value::Num { span, value, signed}) => {
                    match &mut self.state {
                        ScanState::Where(args)=>  {
                            //println!("args {:?}", args);
                            if args.len() > 1 && args[0] == "=" {
                                let right_value = if *signed {
                                    WhereMetaRightDataType::SignedNum(value.to_string())
                                } else {
                                    WhereMetaRightDataType::Num(value.to_string())
                                };

                                let where_meta = WhereMeta::BinaryExpr { left: args[1].clone(), right: right_value };
                                self.push_where(where_meta);
                                
                            }
                            self.state = ScanState::Empty;
                        }

                        ScanState::InsertRowValue(args) => {
                            args.last_mut().unwrap().push(InsertValue { span: *span, value: value.clone() });
                        }
                        _ => { }
                    }
                }

                _ => {}
            },
            _ => {}
        };

        false
    }

    fn complete(&mut self, _node: &mut Node) {
        if let ScanState::InsertRowValue(args) = &mut self.state {
            let args = args.clone();
            self.push_insert_value(args);
        }
    }
}

#[cfg(test)]
mod test {
    use std::vec;

    use mysql_parser::{ast::Visitor, parser::Parser};

    use super::RewriteMetaData;

    #[test]
    fn get_meta() {
        let inputs: Vec<(&str, Vec<&str>, Vec<&str>, Vec<&str>, Vec<&str>, usize)> = vec![
            (
                "SELECT order_id FROM d.t_order WHERE order_id=1 and order_id in (SELECT order_id FROM d.t_order WHERE order_id = 2);",
                vec!["t_order", "t_order"], // table
                vec!["order_id", "order_id"],  // field
                vec![],                        // group
                vec![],                        // order
                2,                              // tables count
            ),
            (
                "SELECT a.* FROM t_order o JOIN t_order_item i ON o.order_id=i.order_id  WHERE order_id IN (1, 2)",
                vec!["t_order", "t_order_item"],
                vec!["a.*"],
                vec![],
                vec![],
                1,              
            ),
            (
                "SELECT order_id FROM t_order WHERE order_id IN (SELECT order_id1 from t_order1  where id in ( SELECT order_id1 from t_order2) group by id order by id) group by id1 limit 1,2",
                vec!["t_order", "t_order1", "t_order2"],                // table
                vec!["order_id", "order_id1", "order_id1"],             // field
                vec!["id", "id1"],                                      // group
                vec!["id"],                                             // order
                3,                                                      // tables count
            ),
            (
                "SELECT AVG(price) FROM t_order WHERE order_id = 1 and order_id1 IN (SELECT AVG(distinct price), order_id1 from t_order1)",
                vec!["t_order", "t_order1"],                            // table
                vec![],                                                 // field
                vec![],                                                 // group
                vec![],                                                 // order
                2,                                                      // tables count
            ),
            (
                "INSERT INTO t(`id`) VALUES (111, 112),(222)",
                vec!["t"],
                vec!["id"],
                vec![],
                vec![],
                1,
            )
        ];

        let parser = Parser::new();

        for input in inputs {
            let mut ast = parser.parse(input.0).unwrap();
            let mut meta = RewriteMetaData::default();
            let _ = ast[0].visit(&mut meta);

            println!("meta {:#?}", meta);
            assert_eq!(
                meta.tables
                    .values()
                    .into_iter()
                    .map(|x| x)
                    .flatten()
                    .map(|x| x.name.as_str())
                    .collect::<Vec<_>>(),
                input.1
            );
            assert_eq!(
                meta.fields
                    .values()
                    .into_iter()
                    .map(|x| x)
                    .flatten()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>(),
                input.2
            );
            assert_eq!(
                meta.groups
                    .values()
                    .into_iter()
                    .map(|x| x)
                    .flatten()
                    .map(|x| x.name.as_str())
                    .collect::<Vec<_>>(),
                input.3
            );
            assert_eq!(
                meta.orders
                    .values()
                    .into_iter()
                    .map(|x| x)
                    .flatten()
                    .map(|x| x.name.as_str())
                    .collect::<Vec<_>>(),
                input.4
            );
            assert_eq!(meta.tables.len(), input.5);
        }
    }
}
