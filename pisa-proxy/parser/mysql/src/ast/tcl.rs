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

use crate::ast::api::*;

// WITH CONSISTENT SNAPSHOT option
#[allow(dead_code)]
const MYSQL_START_TRANS_OPT_WITH_CONS_SNAPSHOT: u8 = 1;
// READ ONLY option
#[allow(dead_code)]
const MYSQL_START_TRANS_OPT_READ_ONLY: u8 = 2;
// READ WRITE option
#[allow(dead_code)]
const MYSQL_START_TRANS_OPT_READ_WRITE: u8 = 4;

#[derive(Debug, Clone)]
pub struct Start {
    pub span: Span,
    pub transaction_opts: u8,
}

impl Start {
    pub fn format(&self) -> String {
        let mut val = String::from("START TRANSACTION ");
        match self.transaction_opts {
            1 => val.push_str("WITH CONSISTENT SNAPSHOT "),
            2 => val.push_str("READ ONLY"),
            4 => val.push_str("READ WRITE"),
            3 => {
                val.push_str("WITH CONSISTENT SNAPSHOT ");
                val.push_str("READ ONLY");
            }
            5 => {
                val.push_str("WITH CONSISTENT SNAPSHOT ");
                val.push_str("READ WRITE");
            }
            6 => {
                val.push_str("READ ONLY ");
                val.push_str("READ WRITE");
            }
            7 => {
                val.push_str("WITH CONSISTENT SNAPSHOT ");
                val.push_str("READ ONLY ");
                val.push_str("READ WRITE");
            }
            _ => {}
        }

        val
    }
}

impl Visitor for Start {
    fn visit<T>(&mut self, _tf: &mut T) -> Self
    where
        T: Transformer,
    {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub span: Span,
    pub opt_work: bool,
    pub opt_chain: YesNoUnknown,
    pub opt_release: YesNoUnknown,
}

impl Commit {
    pub fn format(&self) -> String {
        let mut val = String::from("COMMIT ");
        if self.opt_work {
            val.push_str("WORK ")
        }

        val.push_str(&opt_chain_to_string(&self.opt_chain));
        val.push_str(&opt_release_to_string(&self.opt_release));
        val
    }
}

impl Visitor for Commit {
    fn visit<T>(&mut self, _tf: &mut T) -> Self
    where
        T: Transformer,
    {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Rollback {
    pub span: Span,
    pub opt_work: bool,
    pub opt_chain: YesNoUnknown,
    pub opt_release: YesNoUnknown,
    pub opt_savepoint: bool,
    pub ident: Option<String>,
}

impl Rollback {
    pub fn format(&self) -> String {
        let mut val = String::from("ROLLBACK ");
        if self.opt_work {
            val.push_str("WORK ")
        }

        val.push_str(&opt_chain_to_string(&self.opt_chain));
        val.push_str(&opt_release_to_string(&self.opt_release));

        if let Some(ident) = &self.ident {
            if self.opt_savepoint {
                val.push_str("SAVEPOINT");
            }

            val.push_str("TO");
            val.push_str(ident)
        }

        val
    }
}

impl Visitor for Rollback {
    fn visit<T>(&mut self, _tf: &mut T) -> Self
    where
        T: Transformer,
    {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub enum YesNoUnknown {
    TvlYes,
    TvlNo,
    TvlUnknown,
}

fn opt_chain_to_string(chain: &YesNoUnknown) -> String {
    let mut val = String::with_capacity(12);

    match chain {
        YesNoUnknown::TvlYes => val.push_str("AND CHAIN "),

        YesNoUnknown::TvlNo => val.push_str("AND NO CHAIN "),

        YesNoUnknown::TvlUnknown => {}
    };

    val
}

fn opt_release_to_string(release: &YesNoUnknown) -> String {
    let mut val = String::with_capacity(12);

    match release {
        YesNoUnknown::TvlYes => val.push_str("RELEASE "),

        YesNoUnknown::TvlNo => val.push_str("NO RELEASE "),

        YesNoUnknown::TvlUnknown => {}
    };

    val
}
