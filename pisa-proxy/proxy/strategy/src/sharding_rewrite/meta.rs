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

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use indexmap::IndexMap;
use mysql_parser::ast::*;

#[derive(Debug, Clone)]
enum ScanState {
    Empty,
    TableName,
    // Option<String> means whethe has `alias_name`
    Field(Option<String>),
    Order(OrderDirection),
    Group,
    OnCond(Vec<OnCond>),
    // Option<String> means whethe has `alias_name`
    Avg(mysql_parser::Span, Option<String>, bool),
    // Option<String> means whethe has `alias_name`
    FieldWrapFunc(mysql_parser::Span, FieldWrapFunc, Option<String>),
    InsertRowValue(Vec<InsertValue>),
}

type TableMeta = TableIdent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldWrapFunc {
    Min,
    Max,
    Count,
    None,
}

impl AsRef<str> for FieldWrapFunc {
    fn as_ref(&self) -> &str {
        match self {
            Self::Max => "max",
            Self::Min => "min",
            Self::Count => "count",
            Self::None => "none",
        }
    }
}

#[derive(Debug)]
pub enum FieldMeta {
    Ident(FieldMetaIdent),
    TableWild(TableWild),
}

#[derive(Debug, Clone)]
pub struct FieldMetaIdent {
    pub span: mysql_parser::Span,
    pub name: String,
    pub wrap_func: FieldWrapFunc,
}

#[cfg(test)]
impl FieldMeta {
    fn to_string(&self) -> String {
        match self {
            Self::Ident(field) => field.name.clone(),
            Self::TableWild(val) => val.format(),
        }
    }
}

#[derive(Debug)]
pub struct OrderMeta {
    pub span: mysql_parser::Span,
    pub name: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone)]
pub enum OrderDirection {
    Asc,
    Desc,
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
    BinaryExpr { left: String, right: WhereMetaRightDataType },
}

#[allow(dead_code)]
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
    // IS avg(t)
    pub avg_field_name: String,
    // IS `t` of avg(t)
    pub field_name: String,
    pub distinct: bool,
}

#[derive(Debug)]
pub struct InsertValsMeta {
    pub values: Vec<InsertValue>,
    pub span: mysql_parser::Span,
}

#[derive(Debug, Clone)]
pub struct InsertValue {
    pub span: mysql_parser::Span,
    pub value: String,
}

#[allow(dead_code)]
#[derive(Debug)]
struct QueryRequire {
    query_id: u8,
    expr_token: Option<String>,
    require_query_id: u8,
}

#[derive(Debug, Clone)]
pub struct QueryMeta {
    pub query_id: u8,
    pub span: mysql_parser::Span,
}

#[derive(Debug, Clone)]
pub struct FieldBlockMeta {
    pub query_id: u8,
    pub span: mysql_parser::Span,
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
    queries: IndexMap<u8, QueryMeta>, 
    field_blocks: IndexMap<u8, FieldBlockMeta>,
    query_id: u8,
    state: ScanState,
    requires: Vec<QueryRequire>,
    prev_expr_type: Option<String>,
    // Replace "`" to ""
    ac: AhoCorasick,

    // raw sql str
    input: String,
}

impl RewriteMetaData {
    pub fn new(input: String) -> Self {
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
            queries: IndexMap::new(),
            field_blocks: IndexMap::new(),
            state: ScanState::Empty,
            requires: vec![],
            prev_expr_type: None,
            ac: AhoCorasickBuilder::new().build(&["`"]),
            input,
        }
    }

    fn get_field_name(
        input: &str,
        span: &mysql_parser::Span,
        alias_name: &Option<String>,
    ) -> String {
        match alias_name {
            Some(name) => name.clone(),
            None => input[span.start()..span.end()].to_string(),
        }
    }

    fn push_query(&mut self, meta: QueryMeta) {
        self.queries.insert(self.query_id, meta);
    }

    pub fn get_queries(&self) -> &IndexMap<u8, QueryMeta> {
        &self.queries
    }

    fn push_field_block(&mut self, meta: FieldBlockMeta) {
        self.field_blocks.insert(self.query_id, meta);
    }

    pub fn get_field_blocks(&self) -> &IndexMap<u8, FieldBlockMeta> {
        &self.field_blocks
    }
}

macro_rules! gen_push_func {
    ($($push_func_name:ident, $get_func_name: ident, $meta_typ: tt, $field: tt), *) => {
        impl RewriteMetaData {
            $(
                fn $push_func_name(&mut self, meta: $meta_typ) {
                    self.$field.entry(self.query_id).or_insert(vec![]).push(meta);
                }

                #[allow(dead_code)]
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
            Node::Query(q) => {
                self.query_id += 1;
                
                self.push_query(QueryMeta {
                    query_id: self.query_id,
                    span: q.span,
                })
            }

            Node::SubQuery(q) => {
                q.visit(self);
                self.query_id -= 1;
                return true;
            }

            Node::SingleTable(t) => {
                self.push_table(t.table_name.clone());
            }

            Node::DeleteStmt(t) => {
                if let Some(table) = &t.table_name {
                    self.push_table(table.clone());
                }
            }

            Node::FromClause(_) => {
                self.state = ScanState::TableName;
            }

            Node::TableRef(..) => {
                self.state = ScanState::OnCond(vec![]);
            }

            Node::Items(t) => {
                self.push_field_block(FieldBlockMeta { query_id: self.query_id, span: t.span });
                self.state = ScanState::Field(None);
                for item in t.items.iter() {
                    match item {
                        Item::TableWild(val) => {
                            self.push_field(FieldMeta::TableWild(val.clone()))
                        }
                        _ => {}
                    }
                }
                
            }

            Node::ItemExpr(item) => {
                match &item.expr {
                    Expr::SimpleIdentExpr(Value::Ident { .. }) => {
                        let name = match &item.alias_name {
                            Some(name) => name.clone(),
                            None => self.input[item.span.start()..item.span.end()].to_string(),
                        };
                        self.push_field(FieldMeta::Ident(FieldMetaIdent {
                            span: item.span.clone(),
                            name,
                            wrap_func: FieldWrapFunc::None,
                        }))
                    }
                    Expr::SetFuncSpecExpr(e) => match e.as_ref() {
                        Expr::AggExpr(e) => {
                            match e.name {
                                AggFuncName::Avg => {
                                    self.state = ScanState::Avg(
                                        item.span,
                                        item.alias_name.clone(),
                                        e.distinct,
                                    );
                                }

                                AggFuncName::Max => {
                                    self.state = ScanState::FieldWrapFunc(
                                        item.span,
                                        FieldWrapFunc::Max,
                                        item.alias_name.clone(),
                                    );
                                }

                                AggFuncName::Min => {
                                    self.state = ScanState::FieldWrapFunc(
                                        item.span,
                                        FieldWrapFunc::Min,
                                        item.alias_name.clone(),
                                    );
                                }

                                AggFuncName::Count => {
                                    self.state = ScanState::FieldWrapFunc(
                                        item.span,
                                        FieldWrapFunc::Count,
                                        item.alias_name.clone(),
                                    );
                                }

                                _ => {}
                            }
                            return false;
                        }
                        _ => {}
                    },

                    _ => {}
                }

                return true;
            }

            Node::OrderExpr(val) => {
                if let Some(direction) = &val.direction {
                    if direction == "DESC" {
                        self.state = ScanState::Order(OrderDirection::Desc);
                        return true;
                    }
                }
                self.state = ScanState::Order(OrderDirection::Asc);
            }

            Node::GroupClause(_) => self.state = ScanState::Group,

            Node::LimitClause(val) => {
                for i in val.opts.iter() {
                    self.push_limit(LimitMeta { span: i.span, name: i.opt.clone() })
                }
            }

            Node::InsertStmt(val) => {
                self.push_table(val.table_name.clone());
            }

            Node::RowValue(_) => match self.state {
                ScanState::InsertRowValue(_) => {}
                _ => self.state = ScanState::InsertRowValue(vec![]),
            },

            Node::InsertIdent(val) => match val {
                InsertIdent::Ident(val) => {
                    if let Value::Ident { span, value, quoted } = val {
                        let name =
                            if *quoted { self.ac.replace_all(value, &[""]) } else { value.clone() };
                        self.push_field(FieldMeta::Ident(FieldMetaIdent {
                            span: *span,
                            name,
                            wrap_func: FieldWrapFunc::None,
                        }))
                    }
                }
                InsertIdent::TableWild(val) => self.push_field(FieldMeta::TableWild(val.clone())),
            },

            Node::Expr(e) => match e {
                Expr::Ori(_val) => match &self.state {
                    ScanState::FieldWrapFunc(span, wrap_func, alias_name) => {
                        let wrap_func = wrap_func.clone();
                        let span = span.clone();

                        let name = Self::get_field_name(&self.input, &span, alias_name);
                        self.push_field(FieldMeta::Ident(FieldMetaIdent { span, name, wrap_func }));
                    }
                    _ => {}
                },

                Expr::SimpleIdentExpr(Value::Ident { span, value, .. }) => match &mut self.state {
                    ScanState::Field(alias_name) => {
                        let name = match alias_name {
                            Some(name) => name.clone(),
                            None => value.to_string(),
                        };
                        self.push_field(FieldMeta::Ident(FieldMetaIdent {
                            span: *span,
                            name,
                            wrap_func: FieldWrapFunc::None,
                        }))
                    }
                    ScanState::FieldWrapFunc(span, wrap_func, alias_name) => {
                        let wrap_func = wrap_func.clone();
                        let span = span.clone();

                        let name = Self::get_field_name(&self.input, &span, alias_name);
                        self.push_field(FieldMeta::Ident(FieldMetaIdent { span, name, wrap_func }));
                    }
                    ScanState::Order(direction) => {
                        let direction = direction.clone();
                        self.push_order(OrderMeta {
                            span: *span,
                            name: value.to_string(),
                            direction,
                        })
                    }
                    ScanState::Group => {
                        self.push_group(GroupMeta { span: *span, name: value.to_string() })
                    }
                    ScanState::Avg(arg, alias_name, distinct) => {
                        let avg_field_name = match alias_name {
                            Some(name) => name.clone(),
                            None => self.input.as_str()[arg.start()..arg.end()].to_string(),
                        };
                        let arg = arg.clone();
                        let distinct = *distinct;
                        let meta = AvgMeta {
                            span: arg,
                            avg_field_name,
                            field_name: value.to_string(),
                            distinct,
                        };
                        self.push_avg(meta);
                        self.state = ScanState::Empty;
                    }
                    _ => {}
                },

                Expr::SimpleIdentExpr(e) => {
                    let state = self.state.clone();

                    if let Value::TableIdent { span, .. } = e {
                        match state {
                            ScanState::OnCond(mut args) => {
                                if args.len() < 1 {
                                    args.push(OnCond { span: *span, name: e.format() });
                                    self.state = ScanState::OnCond(args);
                                    return true;
                                }

                                self.push_on_cond(JoinOnCond {
                                    left: args[0].clone(),
                                    right: OnCond { span: *span, name: e.format() },
                                });
                                self.state = ScanState::Empty;
                            }

                            _ => {}
                        }
                    }
                }

                Expr::BinaryOperationExpr { span: _, operator, left, right } => {
                    if operator.format() != "=" {
                        return false;
                    }

                    let where_left =
                        if let Expr::SimpleIdentExpr(Value::Ident { value, .. }) = &**left {
                            Some(value.to_string())
                        } else {
                            None
                        };

                    let where_right =
                        if let Expr::LiteralExpr(Value::Num { value, signed, .. }) = &**right {
                            if *signed {
                                Some(WhereMetaRightDataType::SignedNum(value.to_string()))
                            } else {
                                Some(WhereMetaRightDataType::Num(value.to_string()))
                            }
                        } else {
                            None
                        };

                    if where_left.is_some() && where_right.is_some() {
                        let where_meta = WhereMeta::BinaryExpr {
                            left: where_left.unwrap(),
                            right: where_right.unwrap(),
                        };
                        self.push_where(where_meta);
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

                Expr::LiteralExpr(e) => {
                    if let ScanState::InsertRowValue(args) = &mut self.state {
                        match e {
                            Value::Num { span, value, .. } => {
                                args.push(InsertValue { span: *span, value: value.clone() });
                            },
                            Value::Text {span, value, ..} => {
                                args.push(InsertValue { span: *span, value: value.clone() });
                            },
                            _ => {}
                        }
                    };
                }

                _ => {}
            },
            _ => {}
        };

        false
    }

    fn complete(&mut self, node: &mut Node) {
        if let ScanState::InsertRowValue(args) = &mut self.state {
            let values = args.clone();
            *args = vec![];
            if let Node::RowValue(val) = node {
                self.push_insert_value(InsertValsMeta { values, span: val.span });
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::vec;

    use mysql_parser::{ast::Visitor, parser::Parser};

    use super::RewriteMetaData;
    use crate::sharding_rewrite::meta::FieldMeta;

    #[test]
    fn test_count() {
        let parser = Parser::new();
        let input = "select count(*) from t";
        let mut ast = parser.parse(input).unwrap();
        let mut meta = RewriteMetaData::new(input.to_string());
        let _ = ast[0].visit(&mut meta);
        let meta = meta.get_fields();
        if let FieldMeta::Ident(meta) = &meta[0][0] {
            assert_eq!(meta.wrap_func.as_ref(), "count")
        } else {
            assert!(false)
        }
    }

    #[test]
    fn test_get_alias_name() {
        let parser = Parser::new();
        let input = "select avg(ss) as tt from t";
        let mut ast = parser.parse(input).unwrap();
        let mut meta = RewriteMetaData::new(input.to_string());
        let _ = ast[0].visit(&mut meta);
        let avg_meta = meta.get_avgs();
        assert_eq!(avg_meta[0][0].avg_field_name, "tt");

        let input = "select avg(ss) from t";
        let mut ast = parser.parse(input).unwrap();
        let mut meta = RewriteMetaData::new(input.to_string());
        let _ = ast[0].visit(&mut meta);
        let avg_meta = meta.get_avgs();
        assert_eq!(avg_meta[0][0].avg_field_name, "avg(ss)");
    }

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
                vec!["order_id1"],                                      // field
                vec![],                                                 // group
                vec![],                                                 // order
                2,                                                      // tables count
            ),
            (
                "INSERT INTO t(`id`) VALUES (111, 112), (333)",
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
            let mut meta = RewriteMetaData::new(input.0.to_string());
            let _ = ast[0].visit(&mut meta);

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
