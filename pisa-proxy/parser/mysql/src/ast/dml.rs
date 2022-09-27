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

use lrpar::Span;

use crate::ast::{api::*, base::*};

#[derive(Debug, Clone)]
pub enum SelectStmt {
    Query(Box<Query>),
    SubQuery(Box<SubQuery>),
    With(Box<WithQuery>),
    ValueConstructor(Vec<Vec<Expr>>),
    ExplicitTable(TableIdent),
    None,
}

impl SelectStmt {
    pub fn format(&self) -> String {
        match self {
            Self::Query(val) => {
                val.format()
            }

            Self::SubQuery(val) => {
                val.format()
            }

            Self::With(val) => {
                val.format()
            }

            Self::ValueConstructor(val) => {
                let mut cons = Vec::with_capacity(val.len());

                for v in val {
                    let row_values = vec![
                        "ROW (".to_string(),
                        v.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
                        ")".to_string(),
                    ];

                    cons.push(row_values.join(" "))
                }

                vec!["VALUES".to_string(), cons.join(",")].join(" ")
            }

            Self::ExplicitTable(val) => vec!["TABLE".to_string(), val.format()].join(" "),

            Self::None => "".to_string(),
        }
    }
}

impl Visitor for SelectStmt {
    fn visit<T>(&mut self, tf: &mut T) -> Self
    where
        T: Transformer,
    {
        match self {
            Self::Query(query) => {
                let mut node = Node::Query(query);
                let is_skip = tf.trans(&mut node);

                let new_node = node.into_query().unwrap(); 
                if is_skip {
                    *self = Self::Query(Box::new(new_node.clone()));
                    tf.complete(&mut Node::SelectStmt(self));
                    return self.clone();
                }

                let new_node = new_node.visit(tf);
                *self = Self::Query(Box::new(new_node));
                tf.complete(&mut Node::SelectStmt(self));
                self.clone()
            }

            Self::SubQuery(query) => {
                let mut node = Node::SubQuery(query);
                let is_skip = tf.trans(&mut node);

                let new_node = node.into_sub_query().unwrap(); 
                if is_skip {
                    *self = Self::SubQuery(Box::new(new_node.clone()));
                    tf.complete(&mut Node::SelectStmt(self));
                    return self.clone();
                }

                let new_node = new_node.visit(tf);
                *self = Self::SubQuery(Box::new(new_node));
                tf.complete(&mut Node::SelectStmt(self));
                self.clone()
            }

            Self::With(query) => {
                let mut node = Node::WithQuery(query);
                tf.trans(&mut node);

                let new_node = node.into_with_query().unwrap().visit(tf);
                Self::With(Box::new(new_node))
            }

            Self::ValueConstructor(exprs) => {
                let mut new_exprs = Vec::with_capacity(exprs.len());

                for v in exprs {
                    let mut sub_new_exprs = Vec::with_capacity(v.len());
                    for vv in v {
                        let mut node = Node::Expr(vv);
                        tf.trans(&mut node);

                        let new_node = node.into_expr().unwrap().visit(tf);
                        sub_new_exprs.push(new_node);
                    }

                    new_exprs.push(sub_new_exprs);
                }

                Self::ValueConstructor(new_exprs)
            }

            Self::ExplicitTable(table) => Self::ExplicitTable(table.clone()),

            Self::None => Self::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Query {
    pub span: Span,
    pub opts: Vec<QuerySpecOpt>,
    pub items: Items,
    pub into_clause: Option<IntoClause>,
    pub from_clause: Option<FromClause>,
    pub where_clause: Option<WhereClause>,
    pub group_clause: Option<GroupClause>,
    pub having_clause: Option<HavingClause>,
    pub window_clause: Option<WindowClause>,
    pub union_opt: Option<UnionOpt>,
    pub union_query: Option<Box<SelectStmt>>,
    pub lock_clauses: Vec<LockClause>,
    pub order_clause: Option<OrderClause>,
    pub limit_clause: Option<LimitClause>,
}

impl Query {
    pub fn format(&self) -> String {
        let mut query = Vec::with_capacity(15);

        query.push("SELECT".to_string());

        query.push(self.opts.iter().map(|x| x.format()).collect::<Vec<String>>().join(" "));
        query.push(self.items.format());

        if let Some(clause) = &self.into_clause {
            query.push(clause.format())
        }

        if let Some(clause) = &self.from_clause {
            query.push(clause.format())
        }

        if let Some(clause) = &self.where_clause {
            query.push(clause.format())
        }

        if let Some(clause) = &self.group_clause {
            query.push(clause.format())
        }

        if let Some(clause) = &self.having_clause {
            query.push(clause.format())
        }

        if let Some(clause) = &self.window_clause {
            query.push(clause.format())
        }

        if let Some(clause) = &self.order_clause {
            query.push(clause.format())
        }

        if let Some(clause) = &self.limit_clause {
            query.push(clause.format())
        }

        if !self.lock_clauses.is_empty() {
            let lock =
                self.lock_clauses.iter().map(|x| x.format()).collect::<Vec<String>>().join(" ");
            query.push(lock);
        }

        if let Some(opt) = &self.union_opt {
            query.push("UNION".to_string());
            query.push(opt.format());
        }

        if let Some(union_query) = &self.union_query {
            if self.union_opt.is_none() {
                query.push("UNION".to_string());
            }
            query.push(union_query.format())
        }

        query.join(" ")
    }
}

impl Visitor for Query {
    fn visit<T>(&mut self, tf: &mut T) -> Self
    where
        T: Transformer,
    {
        let mut items = Node::Items(&mut self.items);
        tf.trans(&mut items);
        let new_items = items.into_items().unwrap().visit(tf);
        self.items = new_items;

        if let Some(v) = &mut self.into_clause {
            let mut node = Node::IntoClause(v);
            tf.trans(&mut node);
            self.into_clause = Some(node.into_into_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.from_clause {
            let mut node = Node::FromClause(v);
            tf.trans(&mut node);
            self.from_clause = Some(node.into_from_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.where_clause {
            let mut node = Node::WhereClause(v);
            tf.trans(&mut node);
            self.where_clause = Some(node.into_where_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.group_clause {
            let mut node = Node::GroupClause(v);
            tf.trans(&mut node);
            self.group_clause = Some(node.into_group_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.having_clause {
            let mut node = Node::HavingClause(v);
            tf.trans(&mut node);
            self.having_clause = Some(node.into_having_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.window_clause {
            let mut node = Node::WindowClause(v);
            tf.trans(&mut node);
            self.window_clause = Some(node.into_window_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.order_clause {
            let mut node = Node::OrderClause(v);
            tf.trans(&mut node);
            self.order_clause = Some(node.into_order_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.limit_clause {
            let mut node = Node::LimitClause(v);
            tf.trans(&mut node);
            self.limit_clause = Some(node.into_limit_clause().unwrap().visit(tf));
        }

        let mut new_locks = Vec::with_capacity(self.lock_clauses.len());

        for v in self.lock_clauses.iter_mut() {
            let mut node = Node::LockClause(v);
            tf.trans(&mut node);
            let new_node = node.into_lock_clause().unwrap();
            new_locks.push(new_node.visit(tf))
        }

        if !new_locks.is_empty() {
            self.lock_clauses = new_locks
        }

        if let Some(union) = &mut self.union_query {
            let mut node = Node::SelectStmt(union);
            tf.trans(&mut node);
            let new_node = node.into_select_stmt().unwrap().visit(tf);
            self.union_query = Some(Box::new(new_node));
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum QuerySpecOpt {
    StraightJoin,
    HighPriority,
    LowPriority,
    Delayed,
    Distinct,
    Distinctrow,
    SqlSmallResult,
    SqlBigResult,
    SqlBufferResult,
    SqlCache,
    SqlNoCache,
    SqlCalcFoundRows,
    All,
}

impl QuerySpecOpt {
    pub fn format(&self) -> String {
        let opt = match self {
            Self::StraightJoin => "STRAIGHT_JOIN",
            Self::HighPriority => "HIGH_PRIORITY",
            Self::LowPriority => "LOW_PRIORITY",
            Self::Delayed => "DELAYED",
            Self::Distinct => "DISTINCT",
            Self::Distinctrow => "DISTINCTROW",
            Self::SqlSmallResult => "SQL_SMALL_RESULT",
            Self::SqlBigResult => "SQL_BIG_RESULT",
            Self::SqlBufferResult => "SQL_BUFFER_RESULT",
            Self::SqlCache => "SQL_CACHE",
            Self::SqlNoCache => "SQL_NO_CACHE",
            Self::SqlCalcFoundRows => "SQL_CALC_FOUND_ROWS",
            Self::All => "ALL",
        };

        opt.to_string()
    }
}

#[derive(Debug, Clone)]
pub enum UnionOpt {
    Distinct,
    All,
}

impl UnionOpt {
    pub fn format(&self) -> String {
        match self {
            Self::Distinct => "DISTINCT".to_string(),
            Self::All => "ALL".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Items {
    Wild(ItemWild),
    Items(Vec<Item>),
    None,
}


impl Items {
    pub fn format(&self) -> String {
        match self {
            Self::Wild(_) => "*".to_string(),
            Self::None => "".to_string(),
            Self::Items(val) => val.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
        }
    }
}

impl Visitor for Items {
    fn visit<T>(&mut self, tf: &mut T) -> Self
    where
        T: Transformer,
    {
        match self {
            Self::Items(items) => {
                let mut new_items = Vec::with_capacity(items.len());
                for v in items {
                    let mut node = Node::Item(v);
                    tf.trans(&mut node);
                    let new_node = node.into_item().unwrap();
                    new_items.push(new_node.visit(tf));
                }

                Items::Items(new_items)
            }
            x => x.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ItemWild {
    pub span: Span
}

#[derive(Debug, Clone)]
pub enum Item {
    TableWild(TableWild),

    //clippy::large_enum_variant
    ItemExpr(Box<ItemExpr>),
}

impl Item {
    pub fn format(&self) -> String {
        match self {
            Self::TableWild(val) => val.format(),

            Self::ItemExpr(val) => val.format(),
        }
    }
}

impl Visitor for Item {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::TableWild(wild) => {
                let mut node = Node::TableWild(wild);
                tf.trans(&mut node);
                let new_node = node.into_table_wild().unwrap();
                Item::TableWild(new_node.visit(tf))
            }
            Self::ItemExpr(item) => {
                let mut node = Node::ItemExpr(item);
                tf.trans(&mut node);
                let new_node = node.into_item_expr().unwrap();
                Item::ItemExpr(Box::new(new_node.visit(tf)))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableWild {
    pub span: Span,
    pub schema: Option<String>,
    pub table: String,
}

impl TableWild {
    pub fn format(&self) -> String {
        let mut wild = Vec::with_capacity(3);

        if let Some(schema) = &self.schema {
            wild.push(schema.to_string())
        }

        wild.push(self.table.clone());
        wild.push(".*".to_string());

        wild.join("")
    }
}

impl Visitor for TableWild {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ItemExpr {
    pub span: Span,
    pub expr: Expr,
    pub alias_name: Option<String>,
}

impl ItemExpr {
    pub fn format(&self) -> String {
        let mut item = Vec::with_capacity(2);
        item.push(self.expr.format());

        if let Some(name) = &self.alias_name {
            item.push("AS".to_string());
            item.push(name.to_string());
        }

        item.join(" ")
    }
}

impl Visitor for ItemExpr {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::Expr(&mut self.expr);
        tf.trans(&mut node);

        let new_node = node.into_expr().unwrap().visit(tf);
        self.expr = new_node;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct LockClause {
    pub lock_strength: LockStrength,
    pub locked_row_action: Option<LockedRowAction>,
    pub tables: Vec<String>,
}

impl LockClause {
    pub fn format(&self) -> String {
        let mut clause = Vec::with_capacity(4);

        clause.push(self.lock_strength.format());

        if !self.tables.is_empty() {
            clause.push("OF".to_string());
            clause.push(self.tables.join(","))
        }

        if let Some(lock) = &self.locked_row_action {
            clause.push(lock.format());
        }

        clause.join(" ")
    }
}

impl Visitor for LockClause {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum LockStrength {
    Update,
    Share,
    InShare,
}

impl LockStrength {
    pub fn format(&self) -> String {
        match self {
            Self::Update => "FOR UPDATE".to_string(),
            Self::Share => "FOR SHARE".to_string(),
            Self::InShare => "LOCK IN SHARE MODE".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LockedRowAction {
    SkipLocked,
    NoWait,
}

impl LockedRowAction {
    pub fn format(&self) -> String {
        match self {
            Self::SkipLocked => "SKIP LOCKED".to_string(),
            Self::NoWait => "NOWAIT".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum IntoClause {
    OutFile(OutFile),
    DumpFile(DumpFile),
    Vars(Vec<Value>),
}

impl IntoClause {
    pub fn format(&self) -> String {
        let out = match self {
            Self::OutFile(val) => val.format(),

            Self::DumpFile(val) => val.format(),

            Self::Vars(val) => val.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
        };

        let mut clause = String::with_capacity(out.len() + 5);
        clause.push_str("INTO ");
        clause.push_str(&out);

        clause
    }
}

impl Visitor for IntoClause {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::OutFile(out) => {
                let mut n = Node::OutFile(out);
                tf.trans(&mut n);
                Self::OutFile(n.into_out_file().unwrap().visit(tf))
            }

            Self::DumpFile(dump) => {
                let mut n = Node::DumpFile(dump);
                tf.trans(&mut n);
                Self::DumpFile(n.into_dump_file().unwrap().visit(tf))
            }

            Self::Vars(vars) => {
                let mut new_vars = Vec::with_capacity(vars.len());

                for v in vars {
                    let mut node = Node::Value(v);
                    tf.trans(&mut node);

                    new_vars.push(node.into_value().unwrap().visit(tf))
                }
                Self::Vars(new_vars)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutFile {
    pub span: Span,
    pub name: String,
    pub character_set: Option<CharsetName>,
    pub field_term: Option<FieldTermColumn>,
    pub line_term: Vec<LineTerm>,
}

impl OutFile {
    pub fn format(&self) -> String {
        let mut out = Vec::with_capacity(4);
        out.push("OUTFILE".to_string());
        out.push(self.name.to_string());

        if let Some(charset) = &self.character_set {
            out.push(charset.format());
        }

        if let Some(field) = &self.field_term {
            out.push(field.format())
        }

        if !self.line_term.is_empty() {
            out.push("LINES".to_string());
            out.push(self.line_term.iter().map(|x| x.format()).collect::<Vec<String>>().join(" "));
        }

        out.join(" ")
    }
}

impl Visitor for OutFile {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct DumpFile {
    pub span: Span,
    pub name: String,
}

impl DumpFile {
    pub fn format(&self) -> String {
        let mut dump = String::from("DUMPFILE ");
        dump.push_str(&self.name);

        dump
    }
}

impl Visitor for DumpFile {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum FromClause {
    Dual,
    TableRefs(Vec<TableRef>),
}

impl FromClause {
    pub fn format(&self) -> String {
        let from = match self {
            Self::Dual => "DUAL".to_string(),

            Self::TableRefs(refs) => {
                refs.iter().map(|x| x.format()).collect::<Vec<String>>().join(",")
            }
        };

        let mut clause = "FROM ".to_string();
        clause.push_str(&from);

        clause
    }
}

impl Visitor for FromClause {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::Dual => {
                FromClause::Dual
            }
            Self::TableRefs(refs) => {
                let mut new_refs = Vec::with_capacity(refs.len());
                for v in refs {
                    let mut node = Node::TableRef(v);
                    tf.trans(&mut node);
                    let new_node = node.into_table_ref().unwrap();
                    new_refs.push(new_node.visit(tf).clone());
                }
                FromClause::TableRefs(new_refs)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TableRef {
    TableFactor(Box<TableFactor>),
    JoinedTable(Box<JoinedTable>),
    OjTableFactor(Box<TableFactor>),
    OjJoinedTable(Box<JoinedTable>),
}

impl TableRef {
    pub fn format(&self) -> String {
        match self {
            Self::TableFactor(val) => val.format(),

            Self::JoinedTable(val) => val.format(),

            Self::OjTableFactor(val) => {
                let oj = vec!["{ OJ".to_string(), val.format(), "}".to_string()];

                oj.join(" ")
            }

            Self::OjJoinedTable(val) => {
                let oj = vec!["{ OJ".to_string(), val.format(), "}".to_string()];

                oj.join(" ")
            }
        }
    }
}

impl Visitor for TableRef {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::TableFactor(factor) => {
                let mut n = Node::TableFactor(factor);
                tf.trans(&mut n);
                let new_node = n.into_table_factor().unwrap();
                TableRef::TableFactor(Box::new(new_node.visit(tf)))
            }

            Self::JoinedTable(joined) => {
                let mut n = Node::JoinedTable(joined);
                tf.trans(&mut n);
                let new_node = n.into_joined_table().unwrap();
                TableRef::JoinedTable(Box::new(new_node.visit(tf)))
            }

            Self::OjTableFactor(factor) => {
                let mut n = Node::TableFactor(factor);
                tf.trans(&mut n);
                let new_node = n.into_table_factor().unwrap();
                TableRef::OjTableFactor(Box::new(new_node.visit(tf)))
            }

            Self::OjJoinedTable(joined) => {
                let mut n = Node::JoinedTable(joined);
                tf.trans(&mut n);
                let new_node = n.into_joined_table().unwrap();
                TableRef::OjJoinedTable(Box::new(new_node.visit(tf)))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TableFactor {
    SingleTable(SingleTable),
    SingleTableParens(SingleTable),
    DerivedTable(DerivedTable),
    JoinedTableParens(Box<JoinedTable>),
    TableRefsParens(Vec<TableRef>),
    TableFunc(TableFunc),
}

impl TableFactor {
    pub fn format(&self) -> String {
        match self {
            Self::SingleTable(val) => val.format(),

            Self::SingleTableParens(val) => {
                let parens = vec!["(".to_string(), val.format(), ")".to_string()];

                parens.join(" ")
            }

            Self::DerivedTable(val) => {
                let mut parens = Vec::with_capacity(3);
                let derived = val.format();

                if val.is_parens {
                    parens.push("(".to_string());
                    parens.push(derived);
                    parens.push(")".to_string());
                    return parens.join(" ");
                }

                derived
            }

            Self::JoinedTableParens(val) => {
                let parens = vec!["(".to_string(), val.format(), ")".to_string()];

                parens.join(" ")
            }

            Self::TableRefsParens(val) => {
                let parens = vec![
                    "(".to_string(),
                    val.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
                    ")".to_string(),
                ];

                parens.join(" ")
            }

            Self::TableFunc(val) => val.format(),
        }
    }
}

impl Visitor for TableFactor {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::SingleTable(st) => {
                let mut n = Node::SingleTable(st);
                tf.trans(&mut n);
                let new_node = n.into_single_table().unwrap();
                TableFactor::SingleTable(new_node.visit(tf))
            }

            Self::SingleTableParens(stp) => {
                let mut node = Node::SingleTable(stp);
                tf.trans(&mut node);
                let new_node = node.into_single_table().unwrap();
                TableFactor::SingleTableParens(new_node.visit(tf))
            }

            Self::DerivedTable(dt) => {
                let mut node = Node::DerivedTable(dt);
                tf.trans(&mut node);
                let new_node = node.into_derived_table().unwrap();
                TableFactor::DerivedTable(new_node.visit(tf))
            }

            Self::JoinedTableParens(jtp) => {
                let mut node = Node::JoinedTable(jtp);
                tf.trans(&mut node);
                let new_node = node.into_joined_table().unwrap();
                TableFactor::JoinedTableParens(Box::new(new_node.visit(tf)))
            }

            Self::TableRefsParens(refs) => {
                let mut new_refs = Vec::with_capacity(refs.len());
                for v in refs {
                    let mut node = Node::TableRef(v);
                    tf.trans(&mut node);
                    let new_node = node.into_table_ref().unwrap();
                    new_refs.push(new_node.visit(tf));
                }

                TableFactor::TableRefsParens(new_refs)
            }

            Self::TableFunc(func) => {
                let mut node = Node::TableFunc(func);
                tf.trans(&mut node);
                let new_node = node.into_table_func().unwrap();
                TableFactor::TableFunc(new_node.visit(tf))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SingleTable {
    pub span: Span,
    pub table_name: TableIdent,
    pub partition_names: Vec<String>,
    pub alias_name: Option<String>,
    //field `index_hints` unused yet
    pub index_hints: Option<String>,
    pub is_parens: bool,
}

impl SingleTable {
    pub fn format(&self) -> String {
        let mut single = vec![
            self.table_name.format(),

        ];

        if !self.partition_names.is_empty() {
            single.push("PARTITION (".to_string());
            single.push(self.partition_names.join(","));
            single.push(")".to_string());
        }

        if let Some(name) = &self.alias_name {
            single.push("AS".to_string());
            single.push(name.to_string());
        }

        if let Some(index) = &self.index_hints {
            single.push(index.to_string());
        }

        single.join(" ")
    }
}

impl Visitor for SingleTable {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        //let mut new_hints = Vec::with_capacity(self.index_hints.len());
        //for v in &self.index_hints {
        //    let mut node = Node::IndexHint(v.clone());
        //    tf.trans(&mut node);
        //    new_hints.push(node.into_index_hint().unwrap().visit(tf));
        //}

        //if !new_hints.is_empty() {
        //    self.index_hints = new_hints;
        //}
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct TableIdent {
    pub span: Span,
    pub schema: Option<String>,
    pub name: String
}

impl TableIdent {
    pub fn format(&self) -> String {
        let mut table = String::with_capacity(self.name.len());

        if let Some(schema) = &self.schema {
            table.push_str(schema);
            table.push('.');
        }

        table.push_str(&self.name);

        table
    }
}

impl Visitor for TableIdent {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}


#[derive(Debug, Clone)]
pub enum SingleTableParens {
    SingleTable(SingleTable),
    SingleTableParens(Box<SingleTableParens>),
}

impl SingleTableParens {
    pub fn format(&self) -> String {
        "".to_string()
    }
}

impl Visitor for SingleTableParens {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::SingleTable(st) => {
                let mut node = Node::SingleTable(st);
                tf.trans(&mut node);
                let new_node = node.into_single_table().unwrap();
                SingleTableParens::SingleTable(new_node.visit(tf))
            }

            Self::SingleTableParens(stp) => {
                let mut node = Node::SingleTableParens(stp);
                tf.trans(&mut node);
                let new_node = node.into_single_table_parens().unwrap();
                SingleTableParens::SingleTableParens(Box::new(new_node.visit(tf)))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DerivedTable {
    pub span: Span,
    pub subquery: SelectStmt,
    pub alias_name: Option<String>,
    pub columns: Vec<Value>,
    pub lateral: bool,
    pub is_parens: bool,
}

impl DerivedTable {
    pub fn format(&self) -> String {
        let mut derived = Vec::with_capacity(5);

        if self.lateral {
            derived.push("LATERAL".to_string());
        }

        derived.push(self.subquery.format());

        if let Some(name) = &self.alias_name {
            derived.push("AS".to_string());
            derived.push(name.to_string());
        }

        if !self.columns.is_empty() {
            derived.push("(".to_string());
            derived
                .push(self.columns.iter().map(|x| x.format()).collect::<Vec<String>>().join(","));
            derived.push(")".to_string());
        }

        derived.join(" ")
    }
}

impl Visitor for DerivedTable {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        self.subquery = self.subquery.visit(tf);
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct IndexHint {
    pub span: Span,
    pub hint_type: Value,
    pub key_or_index: Value,
    pub hint_clause: Option<Value>,
    pub keys: Vec<Value>,
}

impl IndexHint {
    pub fn format(&self) -> String {
        "".to_string()
    }
}

impl Visitor for IndexHint {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node_hint_type = Node::Value(&mut self.hint_type);
        tf.trans(&mut node_hint_type);
        self.hint_type = node_hint_type.into_value().unwrap().visit(tf);

        let mut node_key = Node::Value(&mut self.key_or_index);
        tf.trans(&mut node_key);
        self.key_or_index = node_key.into_value().unwrap().visit(tf);

        if let Some(hint) = &mut self.hint_clause {
            let mut node = Node::Value(hint);

            tf.trans(&mut node);
            self.hint_clause = Some(node.into_value().unwrap().visit(tf));
        }

        let mut new_keys = Vec::with_capacity(self.keys.len());
        for v in self.keys.iter_mut() {
            let mut node = Node::Value(v);
            tf.trans(&mut node);
            let new_node = node.into_value().unwrap();
            new_keys.push(new_node.visit(tf));
        }

        if !new_keys.is_empty() {
            self.keys = new_keys;
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct TableFunc {
    pub span: Span,
    pub expr: Expr,
    pub text: Value,
    pub columns_clause: Vec<String>,
    pub alias_name: Option<String>,
}

impl TableFunc {
    pub fn format(&self) -> String {
        let mut func = Vec::with_capacity(8);

        func.push("JSON_TABLE (".to_string());
        func.push(self.expr.format());
        func.push(",".to_string());
        func.push(self.text.format());

        func.push("COLUMNS (".to_string());
        func.push(self.columns_clause.join(","));
        func.push(")".to_string());

        if let Some(name) = &self.alias_name {
            func.push("AS".to_string());
            func.push(name.to_string())
        }

        func.join(" ")
    }
}

impl Visitor for TableFunc {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node_expr = Node::Expr(&mut self.expr);
        tf.trans(&mut node_expr);
        self.expr = node_expr.into_expr().unwrap().visit(tf);

        let mut node_text = Node::Value(&mut self.text);
        tf.trans(&mut node_text);
        self.text = node_text.into_value().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct JoinedTable {
    pub span: Span,
    pub left: TableRef,
    pub join_type: String,
    pub right: TableRef,
    pub on_cond: Option<Box<Expr>>,
    pub using_list: Vec<String>,
}

impl JoinedTable {
    pub fn format(&self) -> String {
        let mut joined = Vec::with_capacity(6);

        joined.push(self.left.format());
        joined.push(self.join_type.clone());
        joined.push(self.right.format());

        if let Some(cond) = &self.on_cond {
            joined.push("ON".to_string());
            joined.push(cond.format());

            return joined.join(" ");
        }

        if !self.using_list.is_empty() {
            joined.push("USING (".to_string());
            joined.push(
                self.using_list.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
            );
            joined.push(")".to_string());

            return joined.join(" ");
        }

        joined.join(" ")
    }
}

impl Visitor for JoinedTable {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node_left = Node::TableRef(&mut self.left);
        tf.trans(&mut node_left);
        self.left = node_left.into_table_ref().unwrap().visit(tf);

        let mut node_right = Node::TableRef(&mut self.right);
        tf.trans(&mut node_right);
        self.right = node_right.into_table_ref().unwrap().visit(tf);

        if let Some(expr) = &mut self.on_cond {
            let mut node_expr = Node::Expr(expr);
            tf.trans(&mut node_expr);
            self.on_cond = Some(Box::new(node_expr.into_expr().unwrap().visit(tf)));
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub span: Span,
    pub expr: Box<Expr>,
}

impl WhereClause {
    pub fn format(&self) -> String {
        let clause = vec!["WHERE".to_string(), self.expr.format()];

        clause.join(" ")
    }
}

impl Visitor for WhereClause {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node_expr = Node::Expr(&mut self.expr);
        tf.trans(&mut node_expr);

        let new_node_expr = node_expr.into_expr().unwrap();
        self.expr = Box::new(new_node_expr.visit(tf));

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct GroupClause {
    pub span: Span,
    pub exprs: Vec<Expr>,
    pub olap: bool,
}

impl GroupClause {
    pub fn format(&self) -> String {
        let mut clause = Vec::with_capacity(3);

        clause.push("GROUP BY".to_string());
        clause.push(self.exprs.iter().map(|x| x.format()).collect::<Vec<String>>().join(","));

        if self.olap {
            clause.push("WITH ROLLUP".to_string())
        }

        clause.join(" ")
    }
}

impl Visitor for GroupClause {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_exprs = Vec::with_capacity(self.exprs.len());

        for v in &mut self.exprs {
            let mut node_expr = Node::Expr(v);
            tf.trans(&mut node_expr);

            let new_node_expr = node_expr.into_expr().unwrap();
            new_exprs.push(new_node_expr.visit(tf));
        }

        self.exprs = new_exprs;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct HavingClause {
    pub span: Span,
    pub expr: Box<Expr>,
}

impl HavingClause {
    pub fn format(&self) -> String {
        let clause = vec!["HAVING".to_string(), self.expr.format()];

        clause.join(" ")
    }
}

impl Visitor for HavingClause {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node_expr = Node::Expr(&mut self.expr);
        tf.trans(&mut node_expr);

        let new_node_expr = node_expr.into_expr().unwrap();
        self.expr = Box::new(new_node_expr.visit(tf));
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct WindowClause {
    pub span: Span,
    pub defs: Vec<WindowDef>,
}

impl WindowClause {
    pub fn format(&self) -> String {
        let mut clause = Vec::with_capacity(1 + self.defs.len());

        clause.push("WINDOW".to_string());
        clause.push(self.defs.iter().map(|x| x.format()).collect::<Vec<String>>().join(","));

        clause.join(" ")
    }
}

impl Visitor for WindowClause {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_defs = Vec::with_capacity(self.defs.len());

        for v in &mut self.defs {
            let mut node = Node::WindowDef(v);
            tf.trans(&mut node);

            let new_node = node.into_window_def().unwrap();
            new_defs.push(new_node.visit(tf));
        }

        self.defs = new_defs;
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct WindowDef {
    pub span: Span,
    pub name: String,
    pub spec: String,
}

impl WindowDef {
    pub fn format(&self) -> String {
        let def = vec![self.name.to_string(), "AS".to_string(), self.spec.to_string()];

        def.join(" ")
    }
}

impl Visitor for WindowDef {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

// `WindowSpec` struct unused yet
#[derive(Debug, Clone)]
pub struct WindowSpec {
    pub span: Span,
    pub name: Option<Value>,
    pub partition_clause: Vec<Expr>,
    pub window_order_clause: Vec<OrderExpr>,
    pub window_frame_clause: Option<WindowFrameClause>,
}

impl WindowSpec {
    pub fn format(&self) -> String {
        "".to_string()
    }
}

impl Visitor for WindowSpec {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        if let Some(name) = &mut self.name {
            let mut node = Node::Value(name);
            tf.trans(&mut node);

            let new_node = node.into_value().unwrap();
            self.name = Some(new_node.visit(tf));
        }

        let mut new_exprs = Vec::with_capacity(self.partition_clause.len());
        for v in &mut self.partition_clause {
            let mut node = Node::Expr(v);
            tf.trans(&mut node);
            let new_node = node.into_expr().unwrap();
            new_exprs.push(new_node.visit(tf));
        }

        if !new_exprs.is_empty() {
            self.partition_clause = new_exprs;
        }

        let mut new_orders = Vec::with_capacity(self.window_order_clause.len());

        for v in &mut self.window_order_clause {
            let mut node = Node::OrderExpr(v);
            tf.trans(&mut node);
            let new_node = node.into_order_expr().unwrap();
            new_orders.push(new_node.visit(tf));
        }

        if !new_orders.is_empty() {
            self.window_order_clause = new_orders;
        }

        if let Some(frame) = &mut self.window_frame_clause {
            let mut node = Node::WindowFrameClause(frame);
            tf.trans(&mut node);
            let new_node = node.into_window_frame_clause().unwrap();
            self.window_frame_clause = Some(new_node.visit(tf));
        }

        self.clone()
    }
}

// `WindowFrameClause` struct unused yet
#[derive(Debug, Clone)]
pub struct WindowFrameClause {
    pub span: Span,
}

impl WindowFrameClause {
    pub fn format(&self) -> String {
        "".to_string()
    }
}

impl Visitor for WindowFrameClause {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct OrderClause {
    pub span: Span,
    pub orders: Vec<OrderExpr>,
}

impl OrderClause {
    pub fn format(&self) -> String {
        let clause = vec![
            "ORDER BY".to_string(),
            self.orders.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
        ];

        clause.join(" ")
    }
}

impl Visitor for OrderClause {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_orders = Vec::with_capacity(self.orders.len());
        for v in &mut self.orders {
            let mut node = Node::OrderExpr(v);
            tf.trans(&mut node);
            let new_node = node.into_order_expr().unwrap();
            new_orders.push(new_node.visit(tf).clone());
        }

        if !new_orders.is_empty() {
            self.orders = new_orders
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct LimitClause {
    pub span: Span,
    pub opts: Vec<LimitOption>,
    pub offset: bool,
}

impl LimitClause {
    pub fn format(&self) -> String {
        let mut clause = Vec::with_capacity(self.opts.len() + 1);
        clause.push("LIMIT".to_string());
        clause.push(self.opts[0].opt.clone());

        if self.offset {
            clause.push("OFFSET".to_string());
            clause.push(self.opts[1].opt.clone());

            return clause.join(" ");
        }

        if let Some(opt) = self.opts.get(1) {
            clause.push(opt.opt.clone());
            return clause.join(", ");
        }

        clause.join("")
    }
}

impl Visitor for LimitClause {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct LimitOption {
    pub span: Span,
    pub opt: String,
}

#[derive(Debug, Clone)]
pub struct SubQuery {
    pub span: Span,
    pub query: SelectStmt,
    pub union_opt: Option<UnionOpt>,
    pub union_query: Option<Box<SelectStmt>>,
    pub into_clause: Option<IntoClause>,
    pub order_clause: Option<OrderClause>,
    pub limit_clause: Option<LimitClause>,
}

impl SubQuery {
    pub fn format(&self) -> String {
        let query = self.query.format();
        let mut subquery = Vec::with_capacity(query.len() + 2);
        subquery.push("(".to_string());
        subquery.push(query);
        subquery.push(")".to_string());
        
        subquery.join(" ")
    }
}

impl Visitor for SubQuery {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::SelectStmt(&mut self.query);
        tf.trans(&mut node);
        let new_node = node.into_select_stmt().unwrap();
        self.query = new_node.visit(tf);

        if let Some(union) = &mut self.union_query {
            let mut node = Node::SelectStmt(union);
            tf.trans(&mut node);
            let new_node = node.into_select_stmt().unwrap().visit(tf);
            self.union_query = Some(Box::new(new_node));
        }

        if let Some(v) = &mut self.into_clause {
            let mut node = Node::IntoClause(v);
            tf.trans(&mut node);
            self.into_clause = Some(node.into_into_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.order_clause {
            let mut node = Node::OrderClause(v);
            tf.trans(&mut node);
            self.order_clause = Some(node.into_order_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.limit_clause {
            let mut node = Node::LimitClause(v);
            tf.trans(&mut node);
            self.limit_clause = Some(node.into_limit_clause().unwrap().visit(tf));
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct WithQuery {
    pub with_clause: WithClause,
    pub expr_body: SelectStmt,
}

impl WithQuery {
    pub fn format(&self) -> String {
        let mut with = vec![self.with_clause.format()];
        with.push(self.expr_body.format());

        with.join(" ")
    }
}

impl Visitor for WithQuery {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::WithClause(&mut self.with_clause);
        tf.trans(&mut node);
        self.with_clause = node.into_with_clause().unwrap().visit(tf);

        let mut node = Node::SelectStmt(&mut self.expr_body);
        tf.trans(&mut node);
        self.expr_body = node.into_select_stmt().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct WithClause {
    pub span: Span,
    pub with_exprs: Vec<CommonTableExpr>,
    pub recursive: bool,
}

impl WithClause {
    pub fn format(&self) -> String {
        let mut with = Vec::with_capacity(self.with_exprs.len() + 1);
        with.push("WITH".to_string());

        if self.recursive {
            with.push("RECURSIVE".to_string())
        }

        with.extend(self.with_exprs.iter().map(|x| x.format()));

        with.join(" ")
    }
}

impl Visitor for WithClause {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_exprs = Vec::with_capacity(self.with_exprs.len());
        for v in &mut self.with_exprs {
            let mut node = Node::CommonTableExpr(v);
            tf.trans(&mut node);
            let new_node = node.into_common_table_expr().unwrap();
            new_exprs.push(new_node.visit(tf))
        }

        if !self.with_exprs.is_empty() {
            self.with_exprs = new_exprs
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct CommonTableExpr {
    pub span: Span,
    pub ident: String,
    pub columns: Vec<Value>,
    pub subquery: SelectStmt,
}

impl CommonTableExpr {
    pub fn format(&self) -> String {
        let mut common = Vec::with_capacity(3 + self.columns.len());
        common.push(self.ident.to_string());
        common.extend(self.columns.iter().map(|x| x.format()));
        common.push("AS".to_string());
        common.push(self.subquery.format());

        common.join(" ")
    }
}

impl Visitor for CommonTableExpr {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_columns = Vec::with_capacity(self.columns.len());

        for v in &mut self.columns {
            let mut node = Node::Value(v);
            tf.trans(&mut node);
            let new_node = node.into_value().unwrap();
            new_columns.push(new_node.visit(tf).clone())
        }

        if !new_columns.is_empty() {
            self.columns = new_columns
        }

        let mut node_query = Node::SelectStmt(&mut self.subquery);
        tf.trans(&mut node_query);
        let new_query = node_query.into_select_stmt().unwrap();
        self.subquery = new_query.visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FieldTerm {
    pub span: Span,
    pub term_type: String,
    pub value: String,
}

impl FieldTerm {
    pub fn format(&self) -> String {
        let mut term = String::with_capacity(3);

        term.push_str(&self.term_type);
        term.push_str(" BY ");
        term.push_str(&self.value);

        term
    }
}

impl Visitor for FieldTerm {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct LineTerm {
    pub span: Span,
    pub term_type: String,
    pub value: String,
}

impl LineTerm {
    pub fn format(&self) -> String {
        let mut term = String::with_capacity(3);

        term.push_str(&self.term_type);
        term.push_str(" BY ");
        term.push_str(&self.value);

        term
    }
}

impl Visitor for LineTerm {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FieldTermColumn {
    pub span: Span,
    pub column_type: String,
    pub terms: Vec<FieldTerm>,
}

impl FieldTermColumn {
    pub fn format(&self) -> String {
        let terms = self.terms.iter().map(|x| x.format()).collect::<Vec<String>>().join(" ");
        let mut term_column = String::with_capacity(self.column_type.len() + terms.len() + 2);

        term_column.push_str(&self.column_type);
        term_column.push(' ');
        term_column.push_str(&terms);

        term_column
    }
}

impl Visitor for FieldTermColumn {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_terms = Vec::with_capacity(self.terms.len());

        for v in &mut self.terms {
            let mut node = Node::FieldTerm(v);
            tf.trans(&mut node);

            let new_node = node.into_field_term().unwrap();
            new_terms.push(new_node.visit(tf))
        }

        if !new_terms.is_empty() {
            self.terms = new_terms
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct CharsetName {
    pub span: Span,
    pub set_type: String,
    pub name: String,
}

impl CharsetName {
    pub fn format(&self) -> String {
        let mut charset = String::with_capacity(self.set_type.len() + self.name.len() + 1);
        charset.push_str(&self.set_type);
        charset.push(' ');
        charset.push_str(&self.name);

        charset
    }
}

impl Visitor for CharsetName {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum FieldType {
    FieldIntType(FieldIntType),
    FieldRealType(FieldRealType),
    FieldNumericType(FieldNumericType),
    FieldBitType(FieldBitType),
    FieldBoolType,
    FieldBooleanType,
    FieldDateType,
    FieldYearType(FieldYearType),
    FieldTimeType(FieldTimeType),
    FieldTextType(FieldTextType),
    FieldBinaryType(FieldBinaryType),
    FieldBlobType(FieldBlobType),
    FieldCharType(FieldCharType),
    FieldNCharType(FieldNCharType),
    FieldVarCharType(FieldVarCharType),
    FieldNVarCharType(FieldNVarCharType),
    FieldSetType(FieldSetType),
    FieldEnumType(FieldEnumType),
    FieldVarBinaryType(FieldVarBinaryType),
    FieldSpatialType(String),
    FieldSerialType,
    FieldJsonType,

    // unified cast type，we have not implement for cast type yet
    FieldCastType(String),
}

impl FieldType {
    pub fn format(&self) -> String {
        match self {
            Self::FieldIntType(val) => val.format(),

            Self::FieldRealType(val) => val.format(),

            Self::FieldNumericType(val) => val.format(),

            Self::FieldBitType(val) => val.format(),

            Self::FieldBoolType => "BOOL".to_string(),

            Self::FieldBooleanType => "BOOLEAN".to_string(),

            Self::FieldDateType => "Date".to_string(),

            Self::FieldYearType(val) => val.format(),

            Self::FieldTimeType(val) => val.format(),

            Self::FieldTextType(val) => val.format(),

            Self::FieldBinaryType(val) => val.format(),

            Self::FieldBlobType(val) => val.format(),

            Self::FieldCharType(val) => val.format(),

            Self::FieldNCharType(val) => val.format(),

            Self::FieldVarCharType(val) => val.format(),

            Self::FieldNVarCharType(val) => val.format(),

            Self::FieldSetType(val) => val.format(),

            Self::FieldEnumType(val) => val.format(),
            Self::FieldVarBinaryType(val) => val.format(),

            Self::FieldSpatialType(val) => val.to_string(),

            Self::FieldSerialType => "SERIAL".to_string(),

            Self::FieldJsonType => "JSON".to_string(),

            Self::FieldCastType(val) => val.to_string(),
        }
    }
}

impl Visitor for FieldType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::FieldIntType(t) => {
                let mut node = Node::FieldIntType(t);
                tf.trans(&mut node);
                FieldType::FieldIntType(node.into_field_int_type().unwrap().clone())
            }

            Self::FieldRealType(t) => {
                let mut node = Node::FieldRealType(t);
                tf.trans(&mut node);
                FieldType::FieldRealType(node.into_field_real_type().unwrap().clone())
            }

            Self::FieldNumericType(t) => {
                let mut node = Node::FieldNumericType(t);
                tf.trans(&mut node);
                FieldType::FieldNumericType(node.into_field_numeric_type().unwrap().clone())
            }

            Self::FieldBitType(t) => {
                let mut node = Node::FieldBitType(t);
                tf.trans(&mut node);
                FieldType::FieldBitType(node.into_field_bit_type().unwrap().clone())
            }

            Self::FieldBoolType => self.clone(),
            Self::FieldBooleanType => self.clone(),
            Self::FieldDateType => self.clone(),

            Self::FieldYearType(t) => {
                let mut node = Node::FieldYearType(t);
                tf.trans(&mut node);
                FieldType::FieldYearType(node.into_field_year_type().unwrap().clone())
            }

            Self::FieldTimeType(t) => {
                let mut node = Node::FieldTimeType(t);
                tf.trans(&mut node);
                FieldType::FieldTimeType(node.into_field_time_type().unwrap().clone())
            }

            Self::FieldTextType(t) => {
                let mut node = Node::FieldTextType(t);
                tf.trans(&mut node);
                FieldType::FieldTextType(node.into_field_text_type().unwrap().clone())
            }

            Self::FieldBinaryType(t) => {
                let mut node = Node::FieldBinaryType(t);
                tf.trans(&mut node);
                FieldType::FieldBinaryType(node.into_field_binary_type().unwrap().clone())
            }

            Self::FieldBlobType(t) => {
                let mut node = Node::FieldBlobType(t);
                tf.trans(&mut node);
                FieldType::FieldBlobType(node.into_field_blob_type().unwrap().clone())
            }

            Self::FieldCharType(t) => {
                let mut node = Node::FieldCharType(t);
                tf.trans(&mut node);
                FieldType::FieldCharType(node.into_field_char_type().unwrap().clone())
            }

            Self::FieldNCharType(t) => {
                let mut node = Node::FieldNCharType(t);
                tf.trans(&mut node);
                FieldType::FieldNCharType(node.into_field_n_char_type().unwrap().clone())
            }

            Self::FieldVarCharType(t) => {
                let mut node = Node::FieldVarCharType(t);
                tf.trans(&mut node);
                FieldType::FieldVarCharType(node.into_field_var_char_type().unwrap().clone())
            }

            Self::FieldNVarCharType(t) => {
                let mut node = Node::FieldNVarCharType(t);
                tf.trans(&mut node);
                FieldType::FieldNVarCharType(node.into_field_n_var_char_type().unwrap().clone())
            }

            Self::FieldSetType(t) => {
                let mut node = Node::FieldSetType(t);
                tf.trans(&mut node);
                FieldType::FieldSetType(node.into_field_set_type().unwrap().clone())
            }

            Self::FieldEnumType(t) => {
                let mut node = Node::FieldEnumType(t);
                tf.trans(&mut node);
                FieldType::FieldEnumType(node.into_field_enum_type().unwrap().clone())
            }

            Self::FieldVarBinaryType(t) => {
                let mut node = Node::FieldVarBinaryType(t);
                tf.trans(&mut node);
                FieldType::FieldVarBinaryType(node.into_field_var_binary_type().unwrap().clone())
            }

            Self::FieldSpatialType(_t) => self.clone(),

            Self::FieldSerialType => self.clone(),
            Self::FieldJsonType => self.clone(),
            Self::FieldCastType(_) => self.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldIntType {
    pub span: Span,
    pub name: String,
    pub length: Option<String>,
    pub opts: Vec<String>,
}

impl FieldIntType {
    pub fn format(&self) -> String {
        let mut int_str = Vec::with_capacity(3);

        int_str.push(self.name.to_string());

        if let Some(length) = &self.length {
            int_str.push(length.to_string())
        }

        int_str.extend(self.opts.iter().map(|x| x.to_string()));

        int_str.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldRealType {
    pub span: Span,
    pub name: String,
    pub precision: Option<String>,
    pub opts: Vec<String>,
}

impl FieldRealType {
    pub fn format(&self) -> String {
        let mut real = Vec::with_capacity(3);

        real.push(self.name.to_string());

        if let Some(prec) = &self.precision {
            real.push(prec.clone())
        }

        real.extend(self.opts.iter().map(|x| x.to_string()));

        real.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldNumericType {
    pub span: Span,
    pub name: String,
    pub float_opts: Option<String>,
    pub opts: Vec<String>,
}

impl FieldNumericType {
    pub fn format(&self) -> String {
        let mut numeric = Vec::with_capacity(3);

        numeric.push(self.name.to_string());

        if let Some(opts) = &self.float_opts {
            numeric.push(opts.clone())
        }

        numeric.extend(self.opts.iter().map(|x| x.to_string()));

        numeric.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldBitType {
    pub span: Span,
    pub length: Option<String>,
}

impl FieldBitType {
    pub fn format(&self) -> String {
        let mut bits = Vec::with_capacity(2);

        bits.push("BIT".to_string());

        if let Some(length) = &self.length {
            bits.push(length.to_string())
        }

        bits.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldCharType {
    pub span: Span,
    pub name: String,
    pub length: Option<String>,
    pub opt: CharsetOrBinary,
}

impl FieldCharType {
    pub fn format(&self) -> String {
        let mut char_str = Vec::with_capacity(3);
        char_str.push(self.name.to_string());

        if let Some(length) = &self.length {
            char_str.push(length.to_string())
        }

        char_str.push(self.opt.format());

        char_str.join(" ")
    }
}

impl Visitor for FieldCharType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::CharsetOrBinary(&mut self.opt);
        tf.trans(&mut node);

        self.opt = node.into_charset_or_binary().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FieldNCharType {
    pub span: Span,
    pub name: String,
    pub length: Option<String>,
    pub bin_mod: Option<String>,
}

impl FieldNCharType {
    pub fn format(&self) -> String {
        let mut nchar = Vec::with_capacity(3);
        nchar.push(self.name.clone());

        if let Some(length) = &self.length {
            nchar.push(length.to_string())
        }

        if let Some(bin_mod) = &self.bin_mod {
            nchar.push(bin_mod.to_string())
        }

        nchar.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldBinaryType {
    pub span: Span,
    pub length: Option<String>,
}

impl FieldBinaryType {
    pub fn format(&self) -> String {
        let mut binary = Vec::with_capacity(2);
        binary.push("BINARY".to_string());

        if let Some(length) = &self.length {
            binary.push(length.to_string())
        }

        binary.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldVarCharType {
    pub span: Span,
    pub name: String,
    pub length: Option<String>,
    pub opt: CharsetOrBinary,
}

impl FieldVarCharType {
    pub fn format(&self) -> String {
        let mut varchar = Vec::with_capacity(3);
        varchar.push(self.name.clone());

        if let Some(length) = &self.length {
            varchar.push(length.to_string())
        }

        varchar.push(self.opt.format());

        varchar.join(" ")
    }
}

impl Visitor for FieldVarCharType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::CharsetOrBinary(&mut self.opt);
        tf.trans(&mut node);

        self.opt = node.into_charset_or_binary().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FieldNVarCharType {
    pub span: Span,
    pub name: String,
    pub length: Option<String>,
    pub bin_mod: Option<String>,
}

impl FieldNVarCharType {
    pub fn format(&self) -> String {
        let mut nvarchar = Vec::with_capacity(3);
        nvarchar.push(self.name.clone());

        if let Some(length) = &self.length {
            nvarchar.push(length.to_string())
        }

        if let Some(bin_mod) = &self.bin_mod {
            nvarchar.push(bin_mod.to_string())
        }

        nvarchar.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldVarBinaryType {
    pub span: Span,
    pub length: String,
}

impl FieldVarBinaryType {
    pub fn format(&self) -> String {
        let mut binary = Vec::with_capacity(3);

        binary.push("VARBINARY".to_string());
        binary.push(self.length.to_string());

        binary.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldYearType {
    pub span: Span,
    pub length: Option<String>,
    pub opts: Vec<String>,
}

impl FieldYearType {
    pub fn format(&self) -> String {
        let mut year = Vec::with_capacity(1 + self.opts.len());

        year.push("YEAR".to_string());

        if let Some(length) = &self.length {
            year.push(length.to_string())
        }

        year.extend(self.opts.iter().map(|x| x.to_string()));

        year.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldTimeType {
    pub span: Span,
    pub name: String,
    pub precision: Option<String>,
}

impl FieldTimeType {
    pub fn format(&self) -> String {
        let mut time_str = Vec::with_capacity(2);

        time_str.push(self.name.to_string());

        if let Some(prec) = &self.precision {
            time_str.push(prec.clone())
        }

        time_str.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct FieldBlobType {
    pub span: Span,
    pub name: String,
    pub length: Option<String>,
    pub varchar: Option<String>,
    pub opt: CharsetOrBinary,
}

impl FieldBlobType {
    pub fn format(&self) -> String {
        let mut blob = Vec::with_capacity(4);

        blob.push(self.name.to_string());

        if let Some(varchar) = &self.varchar {
            blob.push(varchar.clone())
        }

        if let Some(length) = &self.length {
            blob.push(length.to_string())
        }

        blob.push(self.opt.format());

        blob.join(" ")
    }
}

impl Visitor for FieldBlobType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::CharsetOrBinary(&mut self.opt);
        tf.trans(&mut node);

        self.opt = node.into_charset_or_binary().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FieldTextType {
    pub span: Span,
    pub name: String,
    pub length: Option<String>,
    pub opt: CharsetOrBinary,
}

impl FieldTextType {
    pub fn format(&self) -> String {
        let mut text = Vec::with_capacity(3);

        text.push(self.name.clone());

        if let Some(length) = &self.length {
            text.push(length.to_string())
        }

        text.push(self.opt.format());

        text.join(" ")
    }
}

impl Visitor for FieldTextType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::CharsetOrBinary(&mut self.opt);
        tf.trans(&mut node);

        self.opt = node.into_charset_or_binary().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FieldSetType {
    pub span: Span,
    pub values: Vec<String>,
    pub opt: CharsetOrBinary,
}

impl FieldSetType {
    pub fn format(&self) -> String {
        let mut sets = Vec::with_capacity(4 + self.values.len());

        sets.push("SET (".to_string());
        sets.extend(self.values.iter().map(|x| x.to_string()));
        sets.push(self.opt.format());
        sets.push(")".to_string());

        sets.join(" ")
    }
}

impl Visitor for FieldSetType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::CharsetOrBinary(&mut self.opt);
        tf.trans(&mut node);

        self.opt = node.into_charset_or_binary().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FieldEnumType {
    pub span: Span,
    pub values: Vec<String>,
    pub opt: CharsetOrBinary,
}

impl FieldEnumType {
    pub fn format(&self) -> String {
        let mut enums = Vec::with_capacity(4 + self.values.len());

        enums.push("ENUM (".to_string());
        enums.extend(self.values.iter().map(|x| x.to_string()));
        enums.push(self.opt.format());
        enums.push(")".to_string());

        enums.join(" ")
    }
}

impl Visitor for FieldEnumType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::CharsetOrBinary(&mut self.opt);
        tf.trans(&mut node);

        self.opt = node.into_charset_or_binary().unwrap().visit(tf);

        self.clone()
    }
}

//insert stmt
#[derive(Debug, Clone)]
pub struct InsertStmt {
    pub span: Span,
    pub lock: InsertLockOpt,
    pub ignore: bool,
    pub into: bool,
    pub table_name: TableIdent,
    pub partition_names: Vec<String>,
    pub is_set: bool,
    pub from_construct: Option<InsertFromConstruct>,
    pub values_ref: Option<ValuesRef>,
    pub ins_updates: Option<InsUpdates>,
    pub updates: Vec<UpdateElem>,
    pub query_expr: Option<InsertQueryExpr>,
}

impl InsertStmt {
    pub fn format(&self) -> String {
        let mut insert = Vec::with_capacity(10);

        insert.push("INSERT".to_string());
        insert.push(self.lock.format());

        if self.ignore {
            insert.push("IGNORE".to_string())
        }

        if self.into {
            insert.push("INTO".to_string())
        }

        insert.push(self.table_name.format());

        if !self.partition_names.is_empty() {
            insert.push(
                vec![
                    "PARTITION (".to_string(),
                    self.partition_names
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(","),
                    ")".to_string(),
                ]
                .join(" "),
            );
        }
        

        if self.is_set {
            insert.push("SET".to_string())
        }

        if !self.updates.is_empty() {
            insert.push(self.updates.iter().map(|x| x.format()).collect::<Vec<String>>().join(","))
        }

        if let Some(val) = &self.from_construct {
            insert.push(val.format())
        }

        if let Some(val) = &self.values_ref {
            insert.push(val.format())
        }

        if let Some(query) = &self.query_expr {
            insert.push(query.format())
        }

        if let Some(val) = &self.ins_updates {
            insert.push(val.format())
        }

        insert.join(" ")
    }
}

impl Visitor for InsertStmt {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        if let Some(fc) = &mut self.from_construct {
            let mut node = Node::InsertFromConstruct(fc);
            tf.trans(&mut node);

            self.from_construct = Some(node.into_insert_from_construct().unwrap().visit(tf));
        }

        if let Some(vr) = &mut self.values_ref {
            let mut node = Node::ValuesRef(vr);
            tf.trans(&mut node);

            self.values_ref = Some(node.into_values_ref().unwrap().visit(tf));
        }

        if let Some(iu) = &mut self.ins_updates {
            let mut node = Node::InsUpdates(iu);
            tf.trans(&mut node);

            self.ins_updates = Some(node.into_ins_updates().unwrap().visit(tf));
        }

        let mut new_updates = Vec::with_capacity(self.updates.len());
        for v in self.updates.iter_mut() {
            let mut node = Node::UpdateElem(v);
            tf.trans(&mut node);

            new_updates.push(node.into_update_elem().unwrap().visit(tf));
        }

        self.updates = new_updates;

        if let Some(qe) = &mut self.query_expr {
            let mut node = Node::InsertQueryExpr(qe);
            tf.trans(&mut node);

            self.query_expr = Some(node.into_insert_query_expr().unwrap().visit(tf));
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum InsertLockOpt {
    LowPriority,
    Delayed,
    HighPriority,
    None,
}

impl InsertLockOpt {
    pub fn format(&self) -> String {
        let opt = match self {
            Self::LowPriority => "LOW_PRIORITY",
            Self::Delayed => "DELAYED",
            Self::HighPriority => "HIGH_PRIORITY",
            Self::None => "",
        };

        opt.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct UpdateElem {
    pub span: Span,
    pub var_name: Value,
    pub equal: String,
    pub expr: Expr,
}

impl UpdateElem {
    pub fn format(&self) -> String {
        vec![self.var_name.format(), self.equal.to_string(), self.expr.format()].join(" ")
    }
}

impl Visitor for UpdateElem {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_var = Node::Value(&mut self.var_name);
        tf.trans(&mut new_var);

        self.var_name = new_var.into_value().unwrap().visit(tf);

        let mut new_expr = Node::Expr(&mut self.expr);
        tf.trans(&mut new_expr);

        self.expr = new_expr.into_expr().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ValuesRef {
    pub span: Span,
    pub alias_name: String,
    pub derived_columns: Vec<Value>,
}

impl ValuesRef {
    pub fn format(&self) -> String {
        let columns = vec![
            "(".to_string(),
            self.derived_columns.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
            ")".to_string(),
        ]
        .join(" ");

        vec!["AS".to_string(), self.alias_name.to_string(), columns].join(" ")
    }
}

impl Visitor for ValuesRef {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_dc = Vec::with_capacity(self.derived_columns.len());
        for v in self.derived_columns.iter_mut() {
            let mut node = Node::Value(v);
            tf.trans(&mut node);

            new_dc.push(node.into_value().unwrap().visit(tf));
        }

        self.derived_columns = new_dc;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct InsUpdates {
    pub span: Span,
    pub modifers: String,
    pub updates: Vec<UpdateElem>,
}

impl InsUpdates {
    pub fn format(&self) -> String {
        vec![
            self.modifers.to_string(),
            self.updates.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
        ]
        .join(" ")
    }
}

impl Visitor for InsUpdates {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_updates = Vec::with_capacity(self.updates.len());
        for v in self.updates.iter_mut() {
            let mut node = Node::UpdateElem(v);
            tf.trans(&mut node);

            new_updates.push(node.into_update_elem().unwrap().visit(tf));
        }

        self.updates = new_updates;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct InsertFromConstruct {
    pub span: Span,
    pub values: InsertVals,
    pub is_parens: bool,
    pub fields: Vec<InsertIdent>,
}

impl InsertFromConstruct {
    pub fn format(&self) -> String {
        if self.is_parens {
            return vec![
                "(".to_string(),
                self.fields.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
                ")".to_string(),
                self.values.format(),
            ]
            .join(" ");
        }

        self.values.format()
    }
}

impl Visitor for InsertFromConstruct {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_values = Node::InsertVals(&mut self.values);
        tf.trans(&mut new_values);

        self.values = new_values.into_insert_vals().unwrap().visit(tf);

        let mut new_fields = Vec::with_capacity(self.fields.len());
        for v in self.fields.iter_mut() {
            let mut node = Node::InsertIdent(v);
            tf.trans(&mut node);

            new_fields.push(node.into_insert_ident().unwrap().visit(tf));
        }

        self.fields = new_fields;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum ValOrVals {
    Value,
    Values,
}

impl ValOrVals {
    pub fn format(&self) -> String {
        match self {
            Self::Value => "VALUE".to_string(),
            Self::Values => "VALUES".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InsertVals {
    pub span: Span,
    pub val_ident: ValOrVals,
    pub values: Vec<RowValue>,
}

impl InsertVals {
    pub fn format(&self) -> String {
        let mut values = Vec::with_capacity(3);

        for v in &self.values {
            values.push(v.format())
        }

        vec![self.val_ident.format(), values.join(",")].join(" ")
    }
}

impl Visitor for InsertVals {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_values = Vec::with_capacity(self.values.len());

        for v in self.values.iter_mut() {
            let mut node = Node::RowValue(v);
            tf.trans(&mut node);
            let new_node = node.into_row_value().unwrap().visit(tf);
            new_values.push(new_node);
        }

        self.values = new_values;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct RowValue {
    pub span: Span,
    pub values: Vec<Expr>
}

impl RowValue {
    fn format(&self) -> String {
        let values = vec![ 
            "(".to_string(),
            self.values.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
            ")".to_string(),
        ];

        values.join(" ")
    }
}


impl Visitor for RowValue {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_values = Vec::with_capacity(self.values.len());

        for v in self.values.iter_mut() {
            let mut node = Node::Expr(v);
            tf.trans(&mut node);

            let new_node = node.into_expr().unwrap().visit(tf);
            new_values.push(new_node);
        }

        self.values = new_values;

        tf.complete(&mut Node::RowValue(self));

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum InsertIdent {
    Ident(Value),
    TableWild(TableWild),
}

impl InsertIdent {
    pub fn format(&self) -> String {
        match self {
            Self::Ident(val) => val.format(),

            Self::TableWild(val) => val.format(),
        }
    }
}

impl Visitor for InsertIdent {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::Ident(val) => {
                let mut node = Node::Value(val);
                tf.trans(&mut node);

                Self::Ident(node.into_value().unwrap().visit(tf))
            }

            Self::TableWild(val) => {
                let mut node = Node::TableWild(val);
                tf.trans(&mut node);

                Self::TableWild(node.into_table_wild().unwrap().visit(tf))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct InsertQueryExpr {
    pub span: Span,
    pub query: SelectStmt,
    pub fields: Vec<InsertIdent>,
    pub is_parens: bool,
}

impl InsertQueryExpr {
    pub fn format(&self) -> String {
        let query = self.query.format();

        if self.is_parens {
            return vec![
                "(".to_string(),
                self.fields.iter().map(|x| x.format()).collect::<Vec<String>>().join(","),
                ")".to_string(),
                query,
            ]
            .join(" ");
        }

        query
    }
}

impl Visitor for InsertQueryExpr {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_query = Node::SelectStmt(&mut self.query);
        tf.trans(&mut new_query);

        self.query = new_query.into_select_stmt().unwrap().visit(tf);

        let mut new_fields = Vec::with_capacity(self.fields.len());
        for v in self.fields.iter_mut() {
            let mut node = Node::InsertIdent(v);
            tf.trans(&mut node);

            new_fields.push(node.into_insert_ident().unwrap().visit(tf));
        }

        self.fields = new_fields;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct UpdateStmt {
    pub span: Span,
    pub with_clause: Option<WithClause>,
    pub low_priority: bool,
    pub ignore: bool,
    pub table_refs: Vec<TableRef>,
    pub updates: Vec<UpdateElem>,
    pub where_clause: Option<WhereClause>,
    pub order_clause: Option<OrderClause>,
    pub simple_limit: Option<LimitOption>,
}

impl UpdateStmt {
    pub fn format(&self) -> String {
        let mut update = Vec::with_capacity(10);

        if let Some(with) = &self.with_clause {
            update.push(with.format())
        }

        update.push("UPDATE".to_string());

        if self.low_priority {
            update.push("LOW_PRIORITY".to_string())
        }

        if self.ignore {
            update.push("ignore".to_string())
        }

        update.push(self.table_refs.iter().map(|x| x.format()).collect::<Vec<String>>().join(","));

        update.push("SET".to_string());
        update.push(self.updates.iter().map(|x| x.format()).collect::<Vec<String>>().join(","));

        if let Some(clause) = &self.where_clause {
            update.push(clause.format())
        }

        if let Some(clause) = &self.order_clause {
            update.push(clause.format())
        }

        if let Some(limit) = &self.simple_limit {
            update.push("LIMIT".to_string());
            update.push(limit.opt.clone())
        }

        update.join(" ")
    }
}

impl Visitor for UpdateStmt {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        if let Some(with) = &mut self.with_clause {
            let mut node = Node::WithClause(with);
            tf.trans(&mut node);

            self.with_clause = Some(node.into_with_clause().unwrap().visit(tf));
        }

        let mut new_refs = Vec::with_capacity(self.table_refs.len());
        for v in self.table_refs.iter_mut() {
            let mut node = Node::TableRef(v);
            tf.trans(&mut node);
            let new_node = node.into_table_ref().unwrap();
            new_refs.push(new_node.visit(tf));
        }

        self.table_refs = new_refs;

        let mut new_updates = Vec::with_capacity(self.updates.len());
        for v in self.updates.iter_mut() {
            let mut node = Node::UpdateElem(v);
            tf.trans(&mut node);

            new_updates.push(node.into_update_elem().unwrap().visit(tf));
        }

        self.updates = new_updates;

        if let Some(v) = &mut self.where_clause {
            let mut node = Node::WhereClause(v);
            tf.trans(&mut node);
            self.where_clause = Some(node.into_where_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.order_clause {
            let mut node = Node::OrderClause(v);
            tf.trans(&mut node);
            self.order_clause = Some(node.into_order_clause().unwrap().visit(tf));
        }

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteStmt {
    pub span: Span,
    pub with_clause: Option<WithClause>,
    pub quick: bool,
    pub ignore: bool,
    pub low_priority: bool,
    pub is_using: bool,
    pub table_name: Option<TableIdent>,
    pub alias_name: Option<String>,
    pub partition_names: Vec<String>,
    pub where_clause: Option<WhereClause>,
    pub order_clause: Option<OrderClause>,
    pub simple_limit: Option<LimitOption>,
    pub table_alias_refs: Vec<String>,
    pub table_refs: Vec<TableRef>,
}

impl DeleteStmt {
    pub fn format(&self) -> String {
        let mut delete = Vec::with_capacity(15);

        if let Some(with) = &self.with_clause {
            delete.push(with.format())
        }

        delete.push("DELETE".to_string());

        if self.quick {
            delete.push("QUICK".to_string())
        }

        if self.low_priority {
            delete.push("LOW_PRIORITY".to_string())
        }

        if self.ignore {
            delete.push("ignore".to_string())
        }

        if let Some(name) = &self.table_name {
            delete.push("FROM".to_string());
            delete.push(name.format());
            if let Some(name) = &self.alias_name {
                delete.push((*name).to_string());
            }

            if !self.partition_names.is_empty() {
                delete.push("PARTITION (".to_string());
                delete.push(
                    self.partition_names
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(","),
                );
            }

            if let Some(clause) = &self.where_clause {
                delete.push(clause.format())
            }

            if let Some(clause) = &self.order_clause {
                delete.push(clause.format())
            }

            if let Some(limit) = &self.simple_limit {
                delete.push("LIMIT".to_string());
                delete.push(limit.opt.clone())
            }

            return delete.join(" ");
        }

        if self.is_using {
            delete.push("FROM".to_string());
            delete.push(self.table_alias_refs.join(","));
            delete.push("USING".to_string());
        } else {
            delete.push(self.table_alias_refs.join(","));
            delete.push("FROM".to_string());
        }

        delete.push(self.table_refs.iter().map(|x| x.format()).collect::<Vec<String>>().join(","));

        if let Some(clause) = &self.where_clause {
            delete.push(clause.format())
        }

        delete.join(" ")
    }
}

impl Visitor for DeleteStmt {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        if let Some(with) = &mut self.with_clause {
            let mut node = Node::WithClause(with);
            tf.trans(&mut node);

            self.with_clause = Some(node.into_with_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.where_clause {
            let mut node = Node::WhereClause(v);
            tf.trans(&mut node);
            self.where_clause = Some(node.into_where_clause().unwrap().visit(tf));
        }

        if let Some(v) = &mut self.order_clause {
            let mut node = Node::OrderClause(v);
            tf.trans(&mut node);
            self.order_clause = Some(node.into_order_clause().unwrap().visit(tf));
        }

        let mut new_refs = Vec::with_capacity(self.table_refs.len());
        for v in self.table_refs.iter_mut() {
            let mut node = Node::TableRef(v);
            tf.trans(&mut node);
            let new_node = node.into_table_ref().unwrap();
            new_refs.push(new_node.visit(tf));
        }

        self.table_refs = new_refs;

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Prepare {
    pub span: Span,
    pub stmt_name: String,
    pub preparable_stmt: Value,
}

impl Prepare {
    pub fn format(&self) -> String {
        vec![
            "PREPARE".to_string(),
            self.stmt_name.to_string(),
            "FROM".to_string(),
            self.preparable_stmt.format(),
        ]
        .join(" ")
    }
}

impl Visitor for Prepare {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::Value(&mut self.preparable_stmt);
        tf.trans(&mut node);
        self.preparable_stmt = node.into_value().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ExecuteStmt {
    pub span: Span,
    pub stmt_name: String,
    pub execute_using: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct BeginStmt {
    pub span: Span,
    pub work: bool,
}

#[derive(Debug, Clone)]
pub struct Deallocate {
    pub span: Span,
    pub deallocate_or_drop: String,
    pub stmt_name: String,
}

impl Deallocate {
    pub fn format(&self) -> String {
        vec![
            self.deallocate_or_drop.to_string().to_uppercase(),
            "PREPARE".to_string(),
            self.stmt_name.to_string(),
        ]
        .join(" ")
    }
}

impl Visitor for Deallocate {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ShowDatabasesStmt {
    pub span: Span,
    pub opt_wild_or_where: Option<WildOrWhere>,
}

#[derive(Debug, Clone)]
pub enum WildOrWhere {
    LikeTextString(String),
    WhereClause(WhereClause),
}

#[derive(Debug, Clone)]
pub struct ShowTablesStmt {
    pub span: Span,
    pub opt_show_cmd_type: Option<ShowCmdType>,
    pub opt_db: Option<ShowTableDb>,
    pub opt_wild_or_where: Option<WildOrWhere>,
}

#[derive(Debug, Clone)]
pub struct ShowColumnsStmt {
    pub span: Span,
    pub opt_show_cmd_type: Option<ShowCmdType>,
    pub columns_cmd_type: ShowColumnsCmdType,
    pub from_table: ShowFromTable,
    pub opt_db: Option<ShowTableDb>,
    pub opt_wild_or_where: Option<WildOrWhere>,
}

#[derive(Debug, Clone)]
pub enum ShowColumnsCmdType {
    Columns,
    Fields,
}

#[derive(Debug, Clone)]
pub enum ShowCmdType {
    Full,
    Extended,
    ExtendedFull,
}

#[derive(Debug, Clone)]
pub enum FromOrIn {
    From,
    In,
}

#[derive(Debug, Clone)]
pub struct ShowFromTable {
    pub span: Span,
    pub from_or_in: FromOrIn,
    pub table: String,
}

#[derive(Debug, Clone)]
pub struct ShowTableDb {
    pub span: Span,
    pub from_or_in: FromOrIn,
    pub db: String,
}

#[derive(Debug, Clone)]
pub struct ShowCreateTableStmt {
    pub span: Span,
    pub table: TableIdent,
}

#[derive(Debug, Clone)]
pub struct ShowKeysStmt {
    pub span: Span,
    pub is_extended: bool,
    pub keys_or_index: KeysOrIndex,
    pub from_table: ShowFromTable,
    pub opt_db: Option<ShowTableDb>,
    pub opt_where_clause: Option<WhereClause>,
}

#[derive(Debug, Clone)]
pub enum KeysOrIndex {
    Index,
    Indexes,
    Keys,
}

#[derive(Debug, Clone)]
pub struct ShowVariablesStmt {
    pub span: Span,
    pub opt_var_type: Option<ShowVariableType>,
    pub opt_wild_or_where: Option<WildOrWhere>,
}

#[derive(Debug, Clone)]
pub enum ShowVariableType {
    Global,
    Session,
}

#[derive(Debug, Clone)]
pub struct ShowCreateViewStmt {
    pub span: Span,
    pub view_name: TableIdent,
}

#[derive(Debug, Clone)]
pub struct ShowDetailsStmt {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ShowCreateSpStmt {
    pub span: Span,
    pub sp_name: String,
}

#[derive(Debug, Clone)]
pub struct ShowCreateUserStmt {
    pub span: Span,
    pub user: User,
}

#[derive(Debug, Clone)]
pub struct ShowReplicaStatusStmt {
    pub span: Span,
    pub replica: Replica,
    pub opt_channel: Option<Channel>,
}

#[derive(Debug, Clone)]
pub enum Replica {
    Slave,
    Replica,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub span: Span,
    pub channel: String,
}

#[derive(Debug, Clone)]
pub struct ShowStatusStmt {
    pub span: Span,
    pub opt_var_type: Option<ShowVariableType>,
    pub opt_wild_or_where: Option<WildOrWhere>,
}

#[derive(Debug, Clone)]
pub struct ShowGrantsStmt {
    pub span: Span,
    pub user: Option<User>,
    pub user_list: Option<Vec<User>>,
}

#[derive(Debug, Clone)]
pub struct ShowEnginesStmt {
    pub span: Span,
    pub is_storage: bool,
}

#[derive(Debug, Clone)]
pub struct ShowProcessListStmt {
    pub span: Span,
    pub is_full: bool,
}
