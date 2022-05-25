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

use std::{error::Error, fmt};

use cfgrammar::yacc::YaccGrammar;
use lrpar::lrpar_mod;
use lrtable::StateTable;

// use thiserror::Error as ThisError;
use crate::{ast::*, lex::Scanner};

/// define `ParseError` struct
#[derive(Debug, Clone)]
pub struct ParseError {
    details: String,
}

impl ParseError {
    fn new(msg: String) -> ParseError {
        ParseError { details: msg }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        &self.details
    }
}

// load `grmmary.y.rs`, real parsing logic
lrpar_mod!("grammar.y");

pub struct Parser {
    // load `grammar`
    pub grm: YaccGrammar<u32>,
    // load `stabtetable` action
    pub stable: StateTable<u32>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        // generate `grmmar` and `statetable`, this is a hack method for native `grmtools parse`
        let (grm, stable) = ::lrpar::ctbuilder::_reconstitute::<u32>(
            grammar_y::__GRM_DATA,
            grammar_y::__STABLE_DATA,
        );

        Parser { grm, stable }
    }

    // parse input
    pub fn parse<'input>(
        &'input self,
        input: &'input str,
    ) -> Result<Vec<SqlStmt>, Vec<ParseError>> {
        let (res, errs) = grammar_y::parse(input, &self.grm, &self.stable);
        if !errs.is_empty() {
            Err(errs)
        } else {
            Ok(res.unwrap())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::parser::Parser;
    #[test]
    fn test_dml_stmt() {
        let inputs = vec![
            //"select mod(a+b, 4)+1",
            //"select mod( year(a) - abs(weekday(a) + dayofweek(a)), 4) + 1",
            //"select * from (with cte as (select * from t) select 1 union select * from t) qn",
            //"select * from t where 1 > (with cte as (select * from t) select * from cte)",
            //r#"select '\xc6\\' from `\xab`;"#,
            //"select '啊';",
            //r#"select '\xa5\\'"#,
            //r#"select '''\xa5\\'"#,
            //r#"select ```\xa5\\`"#,
            //"select 'e\\'",
            //"select * from t where id = ?",
            //"update test set id = ?",
            //"update test set a = 1, b = ?",
            //// TODO FIX ME.
            //// "delete from test where a = 1",
            //"PREPARE stmt1 FROM 'SELECT SQRT(POW(?,2) + POW(?,2)) AS hypotenuse'",
            //// TODO FIX ME.
            //// "PREPARE stmt2 FROM @s",
            //"EXECUTE stmt2",
            //"BEGIN WORK",
            //"set AUTOCOMMIT=1",
            "SELECT w, SUM(w) OVER (ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING) FROM t;",
        ];

        let p = Parser::new();
        for input in inputs {
            let res = p.parse(input);
            match res {
                Err(e) => {
                    println!("sql={:?} {:?}", input, e)
                }
                Ok(s) => {
                    println!("{:#?}", s)
                }
            }
        }
    }
}