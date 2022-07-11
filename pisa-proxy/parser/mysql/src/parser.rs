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

use crate::{ast::*, lex::Scanner};

/// `ParseError` struct
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

// Load `grmmary.y.rs`, real parsing logic
lrpar_mod!("grammar.y");

pub struct Parser {
    // Load `grammar`
    pub grm: YaccGrammar<u32>,
    // Load `stabtetable` action
    pub stable: StateTable<u32>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        // Generate `grammar` and `statetable`, this is a hack method for native `grmtools parse`
        let (grm, stable) = ::lrpar::ctbuilder::_reconstitute::<u32>(
            grammar_y::__GRM_DATA,
            grammar_y::__STABLE_DATA,
        );

        Parser { grm, stable }
    }

    // Parse input
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
            "select mod(a+b, 4)+1",
            "select mod( year(a) - abs(weekday(a) + dayofweek(a)), 4) + 1",
            "select * from (with cte as (select * from t) select 1 union select * from t) qn",
            "select * from t where 1 > (with cte as (select * from t) select * from cte)",
            r#"select '\xc6\\' from `\xab`;"#,
            "select '啊';",
            r#"select '\xa5\\'"#,
            r#"select '''\xa5\\'"#,
            r#"select ```\xa5\\`"#,
            "select 'e\\'",
            "select * from t where id = ?",
            "update test set id = ?",
            "update test set a = 1, b = ?",
            "delete from test where a = 1",
            "PREPARE stmt1 FROM 'SELECT SQRT(POW(?,2) + POW(?,2)) AS hypotenuse'",
            "PREPARE stmt2 FROM @s",
            "DEALLOCATE PREPARE stmt2",
            "EXECUTE stmt2",
            "BEGIN WORK",
            "set AUTOCOMMIT=1",
            "SELECT w, SUM(w) OVER (ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING) FROM t;",
            "SHOW DATABASES LIKE 'ds%'",
            "SHOW FULL tables FROM test like 't_%'",
            "START TRANSACTION",
            "COMMIT",
            "ROLLBACK",
            "set names utf8mb4",
            "SET character_set_connection = gbk;",
            "SET character_set_results = gbk;",
            "SET character_set_client = \"gbk\";",
            "SET @@GLOBAL.character_set_client = gbk;",
            "SET @@SESSION.character_set_client = gbk;",
            "SELECT * from mysql.select;",
            "create database if not exists db CHARACTER SET = utf8;",
            "select * from test.test limit 1",
            "select * from test.1test limit 1",
            "SHOW COLUMNS FROM t_order;",
            "SHOW CREATE TABLE t_order;",
            "SHOW EXTENDED INDEX FROM t_order FROM db_order where column_name = 'order_id';",
            "SHOW VARIABLES LIKE '%size%';",
            "SHOW GLOBAL VARIABLES LIKE '%size%';",
            "SHOW SESSION VARIABLES LIKE '%size%';",
            "SHOW CREATE VIEW view_name;",
            "SHOW MASTER STATUS;",
            "SHOW CREATE PROCEDURE proc_name;",
            "SHOW CREATE FUNCTION fn_name;",
            "SHOW CREATE TRIGGER trigger_name;",
            "SHOW CREATE EVENT event_name;",
            "SHOW CREATE USER root;",
            "SHOW CREATE USER current_user;",
            "set session transaction isolation level read committed;",
            "SHOW ENGINES;",
            "SHOW STORAGE ENGINES;",
            "SHOW PLUGINS;",
            "SHOW PRIVILEGES;",
            "SHOW SLAVE HOSTS;",
            "SHOW REPLICAS;",
            "SHOW PROCESSLIST;",
            "SHOW FULL PROCESSLIST;",
            "SHOW SLAVE STATUS;",
            "SHOW REPLICA STATUS FOR CHANNEL channel_name;",
        ];

        let p = Parser::new();
        for input in inputs {
            let res = p.parse(input);
            assert!(res.is_ok());
            match res {
                Err(e) => {
                    println!("sql={:?} {:?}", input, e)
                }
                Ok(_stmt) => {
                    //println!("{:#?}", _stmt)
                }
            }
        }
    }
}
