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

use crate::ast::{api::*, dml::*};

#[derive(Debug, Copy, Clone)]
pub enum Op {
    EQ,
    AssignEQ,
    NE,
    GT,
    GE,
    LT,
    LE,
    ShiftLeft,
    ShiftRight,
    OR,
    AND,
    PLUS,
    MINUS,
    MUL,
    DIV,
    MOD,
    XOR,
    NOT,
    TRUE,
    FALSE,
    UNKNOWN,
    NULL,
    NEG,
}

impl Op {
    pub fn format(&self) -> String {
        let op = match self {
            Self::AssignEQ => ":=",
            Self::EQ => "=",
            Self::NE => "!=",
            Self::GT => ">",
            Self::GE => ">=",
            Self::LT => "<",
            Self::LE => "<=",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
            Self::OR => "OR",
            Self::AND => "AND",
            Self::PLUS => "+",
            Self::MINUS => "-",
            Self::MUL => "*",
            Self::DIV => "/",
            Self::MOD => "%",
            Self::XOR => "^",
            Self::NOT => "NOT",
            Self::TRUE => "TRUE",
            Self::FALSE => "FALSE",
            Self::UNKNOWN => "UNKNOWN",
            Self::NULL => "NULL",
            Self::NEG => "~",
        };

        op.to_string()
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    BinaryOperationExpr {
        span: Span,
        operator: Op,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOperationExpr {
        span: Span,
        expr: Box<Expr>,
        operator: Op,
    },
    IsExpr {
        span: Span,
        expr: Box<Expr>,
        operator: Op,
        is_not: bool,
    },
    InExpr {
        span: Span,
        expr: Box<Expr>,
        is_not: bool,
        exprs: Vec<Expr>,
    },
    MemberOfExpr {
        span: Span,
        expr: Box<Expr>,
        of_expr: Box<Expr>,
    },
    BetweenExpr {
        span: Span,
        is_not: bool,
        expr: Box<Expr>,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    SoundsExpr {
        span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    LikeExpr {
        span: Span,
        expr: Box<Expr>,
        is_not: bool,
        pattern_expr: Box<Expr>,
        escape_expr: Option<Box<Expr>>,
    },
    RegexpExpr {
        span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
        is_not: bool,
    },
    FuncCallExpr {
        span: Span,
        name: String,
        args: Vec<Expr>,
    },
    VarExpr(String),
    ValuesExpr(Value),
    SimpleIdentExpr(Value),
    LiteralExpr(Value),
    RowExpr(Vec<Expr>),
    MatchAgainstExpr {
        span: Span,
        idents: Vec<String>,
        expr: Box<Expr>,
        fulltext_opts: Vec<String>,
    },
    BinaryExpr(Box<Expr>),
    CastExpr {
        span: Span,
        expr: Box<Expr>,
        cast_type: String,
    },
    CaseExpr {
        span: Span,
        expr: Box<Option<Expr>>,
        when_exprs: Vec<WhenExpr>,
        else_expr: Box<Option<Expr>>,
    },
    SubQueryExpr(Box<SelectStmt>),

    CompSubQueryExpr(Box<CompSubQueryExpr>),

    ExistsSubQuery(Box<SelectStmt>),
    DefaultExpr,
    TimeIntervalUnit(String),
    CollateExpr {
        span: Span,
        name: String,
        expr: Box<Expr>,
    },
    ParamMarkerExpr,
    InSumExpr {
        span: Span,
        expr: Box<Expr>,
        opt: bool,
    },
    SetFuncSpecExpr(Box<Expr>),

    //Note: Currently, 'None' represent empty rule
    None,

    Ori(String),

    AggExpr(AggExpr)
}

impl Expr {
    pub fn format(&self) -> String {
        match self {
            Self::BinaryOperationExpr { span: _, operator, left, right } => {
                let vals = vec![left.format(), operator.format(), right.format()];
                vals.join(" ")
            }

            Self::UnaryOperationExpr { span: _, expr, operator } => {
                let vals = vec![operator.format(), expr.format()];
                vals.join(" ")
            }

            Self::IsExpr { span: _, expr, operator, is_not } => {
                let mut vals = Vec::with_capacity(3);

                if *is_not {
                    vals.push("NOT".to_string());
                }

                vals.push(operator.format());
                vals.push(expr.format());

                vals.join(" ")
            }

            Self::InExpr { span: _, expr, is_not, exprs } => {
                let mut exprs_str = Vec::with_capacity(exprs.len());
                for v in exprs {
                    exprs_str.push(v.format())
                }

                let mut in_expr = Vec::with_capacity(4);
                in_expr.push(expr.format());

                if *is_not {
                    in_expr.push("NOT".to_string())
                }

                in_expr.push("IN".to_string());
                in_expr.push("(".to_string());
                in_expr.push(exprs_str.join(", "));
                in_expr.push(")".to_string());

                in_expr.join(" ")
            }

            Self::MemberOfExpr { span: _, expr, of_expr } => {
                let mut member_of = Vec::with_capacity(6);

                member_of.push(expr.format());
                member_of.push("MEMBER OF".to_string());
                member_of.push("(".to_string());
                member_of.push(of_expr.format());
                member_of.push(")".to_string());

                member_of.join(" ")
            }

            Self::BetweenExpr { span: _, is_not, expr, left, right } => {
                let mut between = Vec::with_capacity(6);

                between.push(expr.format());

                if *is_not {
                    between.push("NOT".to_string())
                }

                between.push("BETWEEN".to_string());
                between.push(left.format());
                between.push("AND".to_string());
                between.push(right.format());

                between.join(" ")
            }

            Self::SoundsExpr { span: _, left, right } => {
                let sounds = vec![left.format(), "SOUNDS LIKE".to_string(), right.format()];

                sounds.join(" ")
            }

            Self::LikeExpr { span: _, expr, is_not, pattern_expr, escape_expr } => {
                let mut like = Vec::with_capacity(6);

                like.push(expr.format());

                if *is_not {
                    like.push("NOT".to_string())
                }

                like.push("LIKE".to_string());
                like.push(pattern_expr.format());

                if let Some(escape) = escape_expr {
                    like.push("ESCAPE".to_string());
                    like.push(escape.format());
                }

                like.join(" ")
            }

            Self::RegexpExpr { span: _, left, right, is_not } => {
                let mut regexp = Vec::with_capacity(3);

                regexp.push(left.format());

                if *is_not {
                    regexp.push("NOT".to_string());
                }

                regexp.push("REGEXP".to_string());
                regexp.push(right.format());

                regexp.join(" ")
            }

            Self::FuncCallExpr { span: _, name, args } => {
                // bit_expr '-' 'INTERVAL' expr interval
                let mut func = Vec::with_capacity(args.len() + 10);

                func.push(args[0].format());

                if name == "DATE_ADD" {
                    func.push("+".to_string())
                }

                if name == "DATE_SUB" {
                    func.push("-".to_string())
                }

                func.push("INTERVAL".to_string());
                func.push(args[1].format());
                func.push(args[2].format());

                func.join(" ")
            }

            Self::VarExpr(val) => val.to_string(),

            Self::ValuesExpr(val) => {
                //| 'VALUES' '(' simple_ident_nospvar ')'
                let mut values = Vec::with_capacity(4);
                values.push("VALUES (".to_string());
                values.push(val.format());
                values.push(")".to_string());

                values.join(" ")
            }

            Self::SimpleIdentExpr(val) => val.format(),

            Self::LiteralExpr(val) => val.format(),

            Self::RowExpr(val) => {
                let mut exprs = Vec::with_capacity(val.len() + 2);
                exprs.push("(".to_string());
                exprs.extend(val.iter().map(|x| x.format()));
                exprs.push(")".to_string());

                exprs.join(" ,")
            }

            Self::MatchAgainstExpr { span: _, idents: _, expr: _, fulltext_opts: _ } => {
                // TODO impletment match .. against .. expr
                "".to_string()
            }

            Self::BinaryExpr(val) => {
                let binary = vec!["BINARY".to_string(), val.format()];

                binary.join(" ")
            }

            Self::CastExpr { span: _, expr: _, cast_type: _ } => {
                // TODO impletment cast expr
                "".to_string()
            }

            Self::CaseExpr { span: _, expr, when_exprs, else_expr } => {
                //'CASE' opt_expr when_list opt_else` 'END'`
                let mut case = Vec::with_capacity(4 + when_exprs.len());
                case.push("CASE".to_string());

                if let Some(e) = *expr.clone() {
                    case.push(e.format())
                }

                case.extend(when_exprs.iter().map(|x| x.format()));

                if let Some(e) = *else_expr.clone() {
                    case.push(e.format())
                }

                case.push("END".to_string());

                case.join(" ")
            }

            Self::SubQueryExpr(val) => val.format(),

            Self::CompSubQueryExpr(val) => val.format(),

            Self::ExistsSubQuery(val) => val.format(),

            Self::DefaultExpr => "DEFAULT".to_string(),

            Self::TimeIntervalUnit(val) => val.clone(),

            Self::CollateExpr { span: _, name, expr } => {
                let collate = vec![expr.format(), "COLLATE".to_string(), (*name).to_string()];

                collate.join(" ")
            }

            Self::ParamMarkerExpr => "?".to_string(),

            Self::InSumExpr { span: _, expr, opt } => {
                let mut in_sum = Vec::with_capacity(2);
                if *opt {
                    in_sum.push("ALL".to_string())
                }

                in_sum.push(expr.format());

                in_sum.join(" ")
            }

            Self::SetFuncSpecExpr(val) => val.format(),

            Self::None => "".to_string(),

            Self::Ori(val) => (*val).to_string(),
            Self::AggExpr(val) => val.format(),
        }
    }
}

impl Visitor for Expr {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::BinaryOperationExpr { span, operator, left, right } => {
                let mut new_left = Node::Expr(left);
                tf.trans(&mut new_left);

                let mut new_right = Node::Expr(right);
                tf.trans(&mut new_right);

                Self::BinaryOperationExpr {
                    span: *span,
                    operator: *operator,
                    left: Box::new(new_left.into_expr().unwrap().visit(tf)),
                    right: Box::new(new_right.into_expr().unwrap().visit(tf)),
                }
            }

            Self::UnaryOperationExpr { span, expr, operator } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                Self::UnaryOperationExpr {
                    span: *span,
                    operator: *operator,
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                }
            }

            Self::IsExpr { span, expr, operator, is_not } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                Self::IsExpr {
                    span: *span,
                    operator: *operator,
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                    is_not: *is_not,
                }
            }

            Self::InExpr { span, expr, is_not, exprs } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                let mut new_exprs = Vec::with_capacity(exprs.len());
                for v in exprs {
                    let mut node = Node::Expr(v);
                    tf.trans(&mut node);

                    new_exprs.push(node.into_expr().unwrap().visit(tf));
                }

                Self::InExpr {
                    span: *span,
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                    is_not: *is_not,
                    exprs: new_exprs,
                }
            }

            Self::MemberOfExpr { span, expr, of_expr } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                let mut new_of_expr = Node::Expr(of_expr);
                tf.trans(&mut new_of_expr);

                Self::MemberOfExpr {
                    span: *span,
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                    of_expr: Box::new(new_of_expr.into_expr().unwrap().visit(tf)),
                }
            }
            Self::BetweenExpr { span, is_not, expr, left, right } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                let mut new_left_expr = Node::Expr(left);
                tf.trans(&mut new_left_expr);

                let mut new_right_expr = Node::Expr(right);
                tf.trans(&mut new_right_expr);

                Self::BetweenExpr {
                    span: *span,
                    is_not: *is_not,
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                    left: Box::new(new_left_expr.into_expr().unwrap().visit(tf)),
                    right: Box::new(new_right_expr.into_expr().unwrap().visit(tf)),
                }
            }

            Self::SoundsExpr { span, left, right } => {
                let mut new_left_expr = Node::Expr(left);
                tf.trans(&mut new_left_expr);

                let mut new_right_expr = Node::Expr(right);
                tf.trans(&mut new_right_expr);

                Self::SoundsExpr {
                    span: *span,
                    left: Box::new(new_left_expr.into_expr().unwrap().visit(tf)),
                    right: Box::new(new_right_expr.into_expr().unwrap().visit(tf)),
                }
            }

            Self::LikeExpr { span, expr, is_not, pattern_expr, escape_expr } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                let mut new_pattern_expr = Node::Expr(pattern_expr);
                tf.trans(&mut new_pattern_expr);

                if let Some(e) = escape_expr {
                    let mut new_escape_expr = Node::Expr(e);
                    tf.trans(&mut new_escape_expr);
                    *escape_expr = Some(Box::new(new_escape_expr.into_expr().unwrap().visit(tf)))
                }

                Self::LikeExpr {
                    span: *span,
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                    is_not: *is_not,
                    pattern_expr: pattern_expr.clone(),
                    escape_expr: escape_expr.clone(),
                }
            }

            Self::RegexpExpr { span, left, right, is_not } => {
                let mut new_left_expr = Node::Expr(left);
                tf.trans(&mut new_left_expr);

                let mut new_right_expr = Node::Expr(right);
                tf.trans(&mut new_right_expr);

                Self::RegexpExpr {
                    span: *span,
                    is_not: *is_not,
                    left: Box::new(new_left_expr.into_expr().unwrap().visit(tf)),
                    right: Box::new(new_right_expr.into_expr().unwrap().visit(tf)),
                }
            }

            Self::FuncCallExpr { span, name, args } => {
                let mut new_exprs = Vec::with_capacity(args.len());
                for v in args {
                    let mut node = Node::Expr(v);
                    tf.trans(&mut node);

                    new_exprs.push(node.into_expr().unwrap().visit(tf));
                }

                Self::FuncCallExpr { span: *span, name: name.clone(), args: new_exprs }
            }

            Self::VarExpr(val) => Self::VarExpr(val.clone()),

            Self::ValuesExpr(val) => {
                let mut node = Node::Value(val);
                tf.trans(&mut node);

                Self::ValuesExpr(node.into_value().unwrap().visit(tf))
            }

            Self::CastExpr { span, expr, cast_type } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                Self::CastExpr {
                    span: *span,
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                    cast_type: cast_type.clone(),
                }
            }

            Self::SimpleIdentExpr(val) => {
                let mut node = Node::Value(val);
                tf.trans(&mut node);

                Self::SimpleIdentExpr(node.into_value().unwrap().visit(tf))
            }

            Self::CaseExpr { span, expr, when_exprs, else_expr } => {
                if let Some(mut e) = expr.take() {
                    let mut new_expr = Node::Expr(&mut e);
                    tf.trans(&mut new_expr);
                    *expr = Box::new(Some(new_expr.into_expr().unwrap().visit(tf)))
                }

                let mut new_when_exprs = Vec::with_capacity(when_exprs.len());
                for v in when_exprs {
                    let mut node = Node::WhenExpr(v);
                    tf.trans(&mut node);

                    new_when_exprs.push(node.into_when_expr().unwrap().visit(tf));
                }

                if let Some(mut e) = else_expr.take() {
                    let mut new_expr = Node::Expr(&mut e);
                    tf.trans(&mut new_expr);
                    *else_expr = Box::new(Some(new_expr.into_expr().unwrap().visit(tf)))
                }

                Self::CaseExpr {
                    span: *span,
                    expr: expr.clone(),
                    when_exprs: new_when_exprs,
                    else_expr: else_expr.clone(),
                }
            }

            Self::CollateExpr { span, name, expr } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                Self::CollateExpr {
                    span: *span,
                    name: name.to_string(),
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                }
            }

            Self::CompSubQueryExpr(val) => {
                let mut node = Node::CompSubQueryExpr(val);
                tf.trans(&mut node);

                Self::CompSubQueryExpr(Box::new(node.into_comp_sub_query_expr().unwrap().visit(tf)))
            }

            Self::ExistsSubQuery(val) => {
                let mut node = Node::SelectStmt(val);
                tf.trans(&mut node);

                Self::ExistsSubQuery(Box::new(node.into_select_stmt().unwrap().visit(tf)))
            }

            Self::InSumExpr { span, expr, opt } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                Self::InSumExpr {
                    span: *span,
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                    opt: *opt,
                }
            }

            Self::MatchAgainstExpr { span, idents, expr, fulltext_opts } => {
                let mut new_expr = Node::Expr(expr);
                tf.trans(&mut new_expr);

                Self::MatchAgainstExpr {
                    span: *span,
                    idents: idents.clone(),
                    expr: Box::new(new_expr.into_expr().unwrap().visit(tf)),
                    fulltext_opts: fulltext_opts.clone(),
                }
            }

            Self::RowExpr(val) => {
                let mut new_exprs = Vec::with_capacity(val.len());
                for v in val {
                    let mut node = Node::Expr(v);
                    tf.trans(&mut node);

                    new_exprs.push(node.into_expr().unwrap().visit(tf));
                }

                Self::RowExpr(new_exprs)
            }

            Self::LiteralExpr(val) => {
                let mut node = Node::Value(val);
                tf.trans(&mut node);

                Self::LiteralExpr(node.into_value().unwrap().visit(tf))
            }

            Self::BinaryExpr(val) => {
                let mut new_expr = Node::Expr(val);
                tf.trans(&mut new_expr);

                Self::BinaryExpr(Box::new(new_expr.into_expr().unwrap().visit(tf)))
            }

            Self::SubQueryExpr(val) => {
                let mut new_expr = Node::SelectStmt(val);
                tf.trans(&mut new_expr);

                Self::SubQueryExpr(Box::new(new_expr.into_select_stmt().unwrap().visit(tf)))
            }

            Self::SetFuncSpecExpr(val) => {
                let mut node = Node::Expr(val);
                tf.trans(&mut node);

                Self::SetFuncSpecExpr(Box::new(node.into_expr().unwrap().visit(tf)))
            }

            Self::TimeIntervalUnit(val) => Self::TimeIntervalUnit(val.clone()),

            Self::DefaultExpr => Self::DefaultExpr,

            Self::ParamMarkerExpr => Self::ParamMarkerExpr,

            Self::None => Self::None,

            Self::Ori(val) => Self::Ori(val.clone()),

            Self::AggExpr(val) => {
                let mut node = Node::AggExpr(val);
                tf.trans(&mut node);

                Self::AggExpr(node.into_agg_expr().unwrap().visit(tf))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct WhenExpr {
    pub span: Span,
    pub when: Box<Expr>,
    pub then: Box<Expr>,
}

impl WhenExpr {
    pub fn format(&self) -> String {
        let vals =
            vec!["WHEN".to_string(), self.when.format(), "THEN".to_string(), self.then.format()];
        vals.join(" ")
    }
}

impl Visitor for WhenExpr {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_when_expr = Node::Expr(&mut self.when);
        tf.trans(&mut new_when_expr);
        self.when = Box::new(new_when_expr.into_expr().unwrap().visit(tf));

        let mut new_then_expr = Node::Expr(&mut self.then);
        tf.trans(&mut new_then_expr);
        self.then = Box::new(new_then_expr.into_expr().unwrap().visit(tf));

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct CompSubQueryExpr {
    pub span: Span,
    pub expr: Box<Expr>,
    pub subquery: SelectStmt,
    pub operator: Op,
    pub opt: String,
}

impl CompSubQueryExpr {
    pub fn format(&self) -> String {
        let vals = vec![
            self.expr.format(),
            self.operator.format(),
            self.opt.to_string(),
            self.subquery.format(),
        ];
        vals.join(" ")
    }
}

impl Visitor for CompSubQueryExpr {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_expr = Node::Expr(&mut self.expr);
        tf.trans(&mut new_expr);
        self.expr = Box::new(new_expr.into_expr().unwrap().visit(tf));

        let mut new_subqury = Node::SelectStmt(&mut self.subquery);
        tf.trans(&mut new_subqury);
        self.subquery = new_subqury.into_select_stmt().unwrap().visit(tf);

        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum CharsetOrBinary {
    Ascii,
    Binary { set_type: Option<String>, name: Option<String> },
    Unicode,
    Byte,
    Charset { set_type: String, name: String, bin_mod: bool },
    None,
}

impl CharsetOrBinary {
    pub fn format(&self) -> String {
        match self {
            Self::Ascii => "ASCII".to_string(),
            Self::Binary { set_type, name } => {
                let mut binary = Vec::with_capacity(2);
                binary.push("BINARY".to_string());

                if let Some(typ) = set_type {
                    binary.push(typ.to_string());
                }

                if let Some(name) = name {
                    binary.push(name.to_string());
                }

                binary.join(" ")
            }

            Self::Unicode => "UNICODE".to_string(),
            Self::Byte => "BYTE".to_string(),
            Self::Charset { set_type, name, bin_mod } => {
                let mut charset = Vec::with_capacity(3);

                charset.push(set_type.to_string());
                charset.push(name.to_string());

                if *bin_mod {
                    charset.push("BINARY".to_string());
                }

                charset.join(" ")
            }
            Self::None => "".to_string(),
        }
    }
}

impl Visitor for CharsetOrBinary {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Text {
        span: Span,
        value: String,
    },

    TextN {
        span: Span,
        value: String,
    },

    Ident {
        span: Span,
        value: String,
        quoted: bool,
    },

    TableIdent {
        span: Span,
        field: String,
        table: String,
        schema: Option<String>,
    },

    SelectVarIdent {
        span: Span,
        value: String,
        is_at: bool,
    },

    Num {
        span: Span,
        value: String,
        signed: bool,
    },

    HexNum {
        span: Span,
        // todo add decode hex string
        value: String,
    },

    FloatNum {
        span: Span,
        value: String,
        signed: bool,
    },

    BinNum {
        span: Span,
        // todo add decode bit string
        value: String,
    },
    True,
    False,
    Null,
    DefaultIdent,

    // for json_on_response
    Error,
    Default {
        span: Span,
        value: Box<Value>,
    },
}

impl Value {
    pub fn format(&self) -> String {
        match self {
            Self::Text { span: _, value } => (*value).to_string(),

            Self::TextN { span: _, value } => value.clone(),

            Self::Ident { span: _, value, quoted: _ } => (*value).to_string(),

            Self::TableIdent { span: _, field, table, schema } => {
                let mut value = String::with_capacity(field.len() + table.len());
                if let Some(schema) = schema {
                    value.push_str(schema);
                }

                value.push_str(table);
                value.push('.');
                value.push_str(field);

                value
            }

            Self::SelectVarIdent { span: _, value, is_at } => {
                let mut var_ident = String::with_capacity(value.len() + 1);

                if *is_at {
                    var_ident.push('@');
                }

                var_ident.push_str(value);

                var_ident
            }

            Self::Num { span: _, signed, value } | Self::FloatNum { span:_, value, signed }=> {
                let mut value = (*value).to_string();
                if *signed {
                    value.insert(0, '-');
                }
                
                value
            }

            Self::HexNum { span: _, value } => (*value).to_string(),

            Self::BinNum { span: _, value } => (*value).to_string(),

            Self::True => "true".to_string(),
            Self::False => "false".to_string(),
            Self::Null => "NULL".to_string(),
            Self::DefaultIdent => "DEFAULT".to_string(),
            Self::Error => "ERROR".to_string(),

            Self::Default { span: _, value } => (*value).format(),
        }
    }
}

impl Visitor for Value {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::Default { span, value } => {
                let mut new_value = Node::Value(value);
                tf.trans(&mut new_value);

                Self::Default {
                    span: *span,
                    value: Box::new(new_value.into_value().unwrap().visit(tf)),
                }
            }

            x => {
                let mut new_value = Node::Value(x);
                tf.trans(&mut new_value);

                new_value.into_value().unwrap().clone()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SetOptValues {
    OptValues(OptValues),
    Transaction(Transaction),
    OptTypeFollowing(OptTypeFollowing),
}

impl Visitor for SetOptValues {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::OptValues(value) => {
                let mut node = Node::OptValues(value);
                tf.trans(&mut node);

                let new_value = node.into_opt_values().unwrap().visit(tf);
                Self::OptValues(new_value)
            }

            Self::Transaction(value) => {
                let mut node = Node::Transaction(value);
                tf.trans(&mut node);

                let new_value = node.into_transaction().unwrap().visit(tf);
                Self::Transaction(new_value)
            }

            Self::OptTypeFollowing(value) => {
                let mut node = Node::OptTypeFollowing(value);
                tf.trans(&mut node);

                let new_value = node.into_opt_type_following().unwrap().visit(tf);
                Self::OptTypeFollowing(new_value)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptValues {
    pub opt: SetOpts,
    pub values: Vec<OptValue>,
}

impl Visitor for OptValues {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut opt_node = Node::SetOpts(&mut self.opt);
        tf.trans(&mut opt_node);
        let new_opt = opt_node.into_set_opts().unwrap().visit(tf);

        let mut new_values = Vec::with_capacity(self.values.len());
        for v in &mut self.values {
            let mut node = Node::OptValue(v);
            tf.trans(&mut node);
            let new_value = node.into_opt_value().unwrap().visit(tf);
            new_values.push(new_value)
        }

        OptValues { opt: new_opt, values: new_values }
    }
}

#[derive(Debug, Clone)]
pub enum SetOpts {
    SetVariable(SetVariable),
    SetUserVar(SetUserVar),
    SetSystemVar(SetSystemVar),
    SetCharset(SetCharset),
    SetNames(SetNames),
}

impl Visitor for SetOpts {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::SetVariable(value) => {
                let mut node = Node::SetVariable(value);
                tf.trans(&mut node);

                let new_value = node.into_set_variable().unwrap().visit(tf);
                Self::SetVariable(new_value)
            }
            Self::SetUserVar(value) => {
                let mut node = Node::SetUserVar(value);
                tf.trans(&mut node);

                let new_value = node.into_set_user_var().unwrap().visit(tf);
                Self::SetUserVar(new_value)
            }
            Self::SetSystemVar(value) => {
                let mut node = Node::SetSystemVar(value);
                tf.trans(&mut node);

                let new_value = node.into_set_system_var().unwrap().visit(tf);
                Self::SetSystemVar(new_value)
            }
            Self::SetCharset(value) => {
                let mut node = Node::SetCharset(value);
                tf.trans(&mut node);

                let new_value = node.into_set_charset().unwrap().visit(tf);
                Self::SetCharset(new_value)
            }
            Self::SetNames(value) => {
                let mut node = Node::SetNames(value);
                tf.trans(&mut node);

                let new_value = node.into_set_names().unwrap().visit(tf);
                Self::SetNames(new_value)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetVariable {
    pub var: String,
    pub value: ExprOrDefault,
}

impl Visitor for SetVariable {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::ExprOrDefault(&mut self.value);
        tf.trans(&mut node);

        let new_value = node.into_expr_or_default().unwrap().visit(tf);
        SetVariable { var: self.var.clone(), value: new_value }
    }
}

#[derive(Debug, Clone)]
pub enum ExprOrDefault {
    Expr(Expr),
    Default,
    On,
    All,
    Binary,
    Row,
    System,
}

impl Visitor for ExprOrDefault {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::Expr(value) => {
                let mut node = Node::Expr(value);
                tf.trans(&mut node);

                let new_value = node.into_expr().unwrap().visit(tf);
                Self::Expr(new_value)
            }

            x => x.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetUserVar {
    pub var: String,
    pub expr: Expr,
}

impl Visitor for SetUserVar {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::Expr(&mut self.expr);
        tf.trans(&mut node);

        let new_value = node.into_expr().unwrap().visit(tf);

        SetUserVar { var: self.var.clone(), expr: new_value }
    }
}

#[derive(Debug, Clone)]
pub struct SetSystemVar {
    pub opt_var: SetVarIdentType,
    pub var: String,
    pub value: ExprOrDefault,
}

impl Visitor for SetSystemVar {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut opt_node = Node::SetVarIdentType(&mut self.opt_var);
        tf.trans(&mut opt_node);
        let new_opt = opt_node.into_set_var_ident_type().unwrap().visit(tf);

        let mut value_node = Node::ExprOrDefault(&mut self.value);
        tf.trans(&mut value_node);
        let new_value = value_node.into_expr_or_default().unwrap().visit(tf);

        Self { opt_var: new_opt, var: self.var.clone(), value: new_value }
    }
}

#[derive(Debug, Clone)]
pub struct SetCharset {
    pub charset: String,
    pub new_value: NewCharset,
}

impl Visitor for SetCharset {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node = Node::NewCharset(&mut self.new_value);
        tf.trans(&mut node);

        let new_value = node.into_new_charset().unwrap().visit(tf);

        Self { charset: self.charset.clone(), new_value }
    }
}

#[derive(Debug, Clone)]
pub struct SetNames {
    pub charset_name: Option<String>,
    pub collate: Option<String>,
}

impl Visitor for SetNames {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum SetVarIdentType {
    Persist,
    PersistOnly,
    Global,
    Local,
    Session,
    None,
}

impl Visitor for SetVarIdentType {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum NewCharset {
    CharsetName(String),
    Default,
    Binary,
}

impl Visitor for NewCharset {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum OptType {
    Global,
    Persist,
    PersistOnly,
    Local,
    Session,
}

impl Visitor for OptType {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum OptValue {
    OptValueType(OptValueType),
    SetOpts(SetOpts),
}

impl Visitor for OptValue {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::OptValueType(value) => {
                let mut node = Node::OptValueType(value);
                tf.trans(&mut node);

                let new_value = node.into_opt_value_type().unwrap().visit(tf);
                Self::OptValueType(new_value)
            }
            Self::SetOpts(value) => {
                let mut node = Node::SetOpts(value);
                tf.trans(&mut node);

                let new_value = node.into_set_opts().unwrap().visit(tf);
                Self::SetOpts(new_value)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptValueType {
    pub opt_type: OptType,
    pub set_var: SetVariable,
}

impl Visitor for OptValueType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut opt_node = Node::OptType(&mut self.opt_type);
        tf.trans(&mut opt_node);
        let opt_type = opt_node.into_opt_type().unwrap().visit(tf);

        let mut set_node = Node::SetVariable(&mut self.set_var);
        tf.trans(&mut set_node);
        let set_var = set_node.into_set_variable().unwrap().visit(tf);

        Self { opt_type, set_var }
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub mode: Option<TransactionType>,
    pub isolation_level: Option<IsolationType>,
}

impl Visitor for Transaction {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mode = match &mut self.mode {
            Some(mode) => {
                let mut node = Node::TransactionType(mode);
                tf.trans(&mut node);
                Some(node.into_transaction_type().unwrap().visit(tf))
            }
            None => None,
        };

        let isolation_level = match &mut self.isolation_level {
            Some(level) => {
                let mut node = Node::IsolationType(level);
                tf.trans(&mut node);
                Some(node.into_isolation_type().unwrap().visit(tf))
            }
            None => None,
        };

        Self { mode, isolation_level }
    }
}

#[derive(Debug, Clone)]
pub enum TransactionType {
    ReadOnly,
    ReadWrite,
}

impl Visitor for TransactionType {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum IsolationType {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl Visitor for IsolationType {
    fn visit<T: Transformer>(&mut self, _tf: &mut T) -> Self {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum FollowingOptType {
    TypeEq(FollowingOptTypeEq),
    Transaction(Transaction),
}

impl Visitor for FollowingOptType {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        match self {
            Self::TypeEq(value) => {
                let mut node = Node::FollowingOptTypeEq(value);
                tf.trans(&mut node);
                let new_value = node.into_following_opt_type_eq().unwrap().visit(tf);
                Self::TypeEq(new_value)
            }
            Self::Transaction(value) => {
                let mut node = Node::Transaction(value);
                tf.trans(&mut node);
                let new_value = node.into_transaction().unwrap().visit(tf);
                Self::Transaction(new_value)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FollowingOptTypeEq {
    pub opt_type: SetVariable,
    pub values: Vec<OptValue>,
}

impl Visitor for FollowingOptTypeEq {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut opt_node = Node::SetVariable(&mut self.opt_type);
        tf.trans(&mut opt_node);
        let opt_type = opt_node.into_set_variable().unwrap().visit(tf);

        let mut values = Vec::with_capacity(self.values.len());
        for v in self.values.iter_mut() {
            let mut node = Node::OptValue(v);
            tf.trans(&mut node);
            let value = node.into_opt_value().unwrap().visit(tf);
            values.push(value)
        }

        Self { opt_type, values }
    }
}

#[derive(Debug, Clone)]
pub struct OptTypeFollowing {
    pub opt_type: OptType,
    pub following: FollowingOptType,
}

impl Visitor for OptTypeFollowing {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut opt_node = Node::OptType(&mut self.opt_type);
        tf.trans(&mut opt_node);
        let opt_type = opt_node.into_opt_type().unwrap().visit(tf);

        let mut following_node = Node::FollowingOptType(&mut self.following);
        tf.trans(&mut following_node);
        let following = following_node.into_following_opt_type().unwrap().visit(tf);

        Self { opt_type, following }
    }
}

#[derive(Debug, Clone)]
pub enum User {
    CurrentUser,
    UserIdentOrText(String),
}

#[derive(Debug, Clone)]
pub struct OrderExpr {
    pub span: Span,
    pub expr: Expr,
    pub direction: Option<String>,
}

impl OrderExpr {
    pub fn format(&self) -> String {
        let mut order = Vec::with_capacity(2);

        order.push(self.expr.format());

        if let Some(direct) = &self.direction {
            order.push(direct.to_string())
        }

        order.join(" ")
    }
}

impl Visitor for OrderExpr {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut node_expr = Node::Expr(&mut self.expr);
        tf.trans(&mut node_expr);
        let new_node_expr = node_expr.into_expr().unwrap();
        self.expr = new_node_expr.visit(tf);
        self.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggFuncName {
    Avg,
    Count,
    BitAnd,
    BitOr,
    BitXor,
    JsonArrayAgg,
    JsonObjectAgg,
    StCollect,
    Min,
    Max,
    Variance,
    Std,
    StdDev,
    StdDevPop,
    StdDevSamp,
    VarPop,
    VarSamp,
    Sum,
    GroupConcat,
}

impl AsRef<str> for AggFuncName {
    #[inline]
    fn as_ref(&self) -> &str {
        match self {
            Self::Avg               => "AVG",
            Self::Count             => "COUNT",
            Self::BitAnd            => "BIT_AND",
            Self::BitOr             => "BIT_OR",
            Self::BitXor            => "BIT_XOR",
            Self::JsonArrayAgg      => "JSON_ARRAYAGG",
            Self::JsonObjectAgg     => "JSON_OBJECTAGG",
            Self::StCollect         => "ST_COLLECT",
            Self::Min               => "MIN",
            Self::Max               => "MAX",
            Self::Variance          => "VARIANCE",
            Self::Std               => "Std",
            Self::StdDev            => "STDDEV",
            Self::StdDevPop         => "STDDEV_POP",
            Self::StdDevSamp        => "STDDEV_SAMP",
            Self::VarPop            => "VAR_POP",
            Self::VarSamp           => "VAR_SAMP",
            Self::Sum               => "SUM",
            Self::GroupConcat       => "GROUP_CONCAT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AggExpr {
    pub span: Span,
    pub name: AggFuncName,
    pub distinct: bool,
    pub exprs: Vec<Expr>,

    // GroupConcat info 
    pub group_concat_distinct: Option<String>,
    pub gorder_clause: Vec<OrderExpr>,
    pub gconcat_separator: Option<String>,
}

impl AggExpr {
    pub fn format(&self) -> String {
        let mut agg = Vec::with_capacity(10);
        agg.push(self.name.as_ref().to_string());
        agg.push("(".to_string());

        let exprs = self.exprs.iter().map(|x| x.format()).collect::<Vec<_>>().join(", ");
        
        if self.name != AggFuncName::GroupConcat {
            if self.distinct {
                agg.push("DISTINCT".to_string());
            }

            agg.push(exprs);

        } else {
            if let Some(opt) = &self.group_concat_distinct {
                agg.push(opt.clone());
            }

            agg.push(exprs);

            agg.push(self.gorder_clause.iter().map(|x| x.format()).collect::<Vec<_>>().join(", "));

            if let Some(opt) = &self.gconcat_separator {
                agg.push(opt.clone());
            }
        }

        agg.push(")".to_string());
        agg.join(" ")
    }
}

impl Visitor for AggExpr {
    fn visit<T: Transformer>(&mut self, tf: &mut T) -> Self {
        let mut new_exprs = Vec::with_capacity(self.exprs.len());
        for v in self.exprs.iter_mut() {
            let mut node = Node::Expr(v);
            tf.trans(&mut node);

            new_exprs.push(node.into_expr().unwrap().visit(tf));
        }

        self.exprs = new_exprs;
        self.clone()
    }
}

#[cfg(test)]
mod test {
    use lrpar::Span;

    use crate::ast::*;

    #[derive(Clone)]
    struct S {
        is_default: bool,
    }

    #[test]
    fn test_value() {
        impl Transformer for S {
            fn trans(&mut self, node: &mut Node) -> &mut Self {
                match node {
                    Node::Value(Value::Text { span: _, value }) => {
                        if self.is_default {
                            *value = "default".to_string();
                        } else {
                            *value = "trans".to_string();
                        }
                    }

                    Node::Value(Value::Num { span: _, value }) => {
                        *value = "2".to_string();
                    }

                    Node::Value(Value::Default { span: _, value: _ }) => self.is_default = true,

                    _ => {}
                };

                self
            }
        }

        let mut s_ins = S { is_default: false };

        test_value_text(&mut s_ins);
        test_value_num(&mut s_ins);
        test_value_default_json(&mut s_ins);
    }

    fn test_value_text(s: &mut S) {
        let mut val = Value::Text { span: Span::new(1, 1), value: "test".to_string() };

        let new_val = val.visit(s);
        if let Value::Text { span: _, value } = new_val {
            assert_eq!(value, "trans")
        }
    }

    fn test_value_num(s: &mut S) {
        let mut val = Value::Num { span: Span::new(1, 1), value: "1".to_string() };

        let new_val = val.visit(s);
        if let Value::Num { span: _, value } = new_val {
            assert_eq!(value, "2")
        }
    }

    fn test_value_default_json(s: &mut S) {
        let mut val = Value::Default {
            span: Span::new(1, 1),
            value: Box::new(Value::Text { span: Span::new(1, 1), value: "test".to_string() }),
        };

        s.is_default = true;
        let new_val = val.visit(s);
        if let Value::Default { span: _, value } = new_val {
            if let Value::Text { span: _, value } = *value {
                assert_eq!(value, "default")
            }
        }
    }
}
