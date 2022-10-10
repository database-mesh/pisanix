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
            "select * from test.test limit 1",
            "select * from test.1test limit 1",
            "SHOW COLUMNS FROM t_order;",
            "SHOW CREATE TABLE t_order;",
            "SHOW EXTENDED INDEX FROM t_order FROM db_order where column_name = 'order_id';",
            "SHOW VARIABLES LIKE '%size%';",
            "SHOW GLOBAL VARIABLES LIKE '%size%';",
            "SHOW SESSION VARIABLES LIKE '%size%';",
            "SHOW STATUS LIKE '%size%';",
            "SHOW GLOBAL STATUS LIKE '%size%';",
            "SHOW SESSION STATUS LIKE '%size%';",
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
            "SHOW REPLICA STATUS FOR CHANNEL 'channel_name';",
            "SHOW GRANTS;",
            "SHOW GRANTS FOR 'root';",
            "SHOW GRANTS FOR 'u1'@'localhost';",
            "SHOW GRANTS FOR 'u1'@'localhost' USING 'r1';",
            "SHOW GRANTS FOR 'u1'@'localhost' USING 'r1', 'r2';",
        ];

        parser(inputs);
    }

    #[test]
    fn test_ddl_stmt() {
        let inputs = vec![
            "CREATE DATABASE IF NOT EXISTS db CHARACTER SET = utf8;",
            "CREATE INDEX idx_order_id ON t_order (order_id);",
            "CREATE UNIQUE INDEX idx_order_compose USING BTREE ON t_order (name(10) DESC, (col1 + col2) ASC);",
            "CREATE FULLTEXT INDEX idx_order_name ON t_order (name) WITH PARSER parser_name;",
            "CREATE SPATIAL INDEX idx_order_name ON t_order (name) INVISIBLE;",
            "CREATE FULLTEXT INDEX idx_order_name ON t_order (name) WITH PARSER parser_name ALGORITHM DEFAULT;",
            "CREATE FULLTEXT INDEX idx_order_name ON t_order (name) WITH PARSER parser_name LOCK = EXCLUSIVE;",
            "CREATE LOGFILE GROUP logfile_group ADD UNDOFILE 'undo_file' INITIAL_SIZE = 1M UNDO_BUFFER_SIZE = 2M REDO_BUFFER_SIZE = 3M;",
            "CREATE LOGFILE GROUP logfile_group ADD UNDOFILE 'undo_file' NODEGROUP 1234, WAIT, COMMENT 'logfile_group_comment', ENGINE  = 'logfile_group_engine';",
            "CREATE TABLESPACE `ts1` ADD DATAFILE 'ts1.ibd' ENGINE=INNODB;",
            "CREATE TABLESPACE `ts2` ADD DATAFILE 'ts2.ibd' FILE_BLOCK_SIZE = 8192 Engine=InnoDB;",
            "CREATE TABLESPACE myts ADD DATAFILE 'mydata-1.dat' USE LOGFILE GROUP mylg ENGINE=NDB;",
            r#"CREATE TABLESPACE ts1 ENGINE_ATTRIBUTE='{"key":"value"}';"#,
            "CREATE UNDO TABLESPACE undo_003 ADD DATAFILE 'undo_003.ibu' ENGINE=INNODB;",
            "CREATE SERVER s FOREIGN DATA WRAPPER mysql OPTIONS (USER 'Remote', HOST '198.51.100.106', DATABASE 'test');",
            "CREATE TABLE t_order (id bigint, order_id bigint, user_id bigint);",
            "CREATE TABLE new_tbl LIKE orig_tbl;",
            "CREATE TABLE new_tbl AS SELECT * FROM orig_tbl;",
            "CREATE TABLE t (c CHAR(20) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin);",
            "CREATE TABLE test (blob_col BLOB, INDEX(blob_col(10)));",
            r#"CREATE TABLE t1 (c1 INT ENGINE_ATTRIBUTE='{"key":"value"}');"#,
            "CREATE TABLE t1 ( c1 INT STORAGE DISK, c2 INT STORAGE MEMORY) TABLESPACE ts_1 ENGINE NDB;",
            "CREATE TABLE lookup(id INT, INDEX USING BTREE (id)) ENGINE = MEMORY;",
            r#"CREATE TABLE t1 (c1 INT NOT NULL AUTO_INCREMENT PRIMARY KEY, c2 VARCHAR(100), c3 VARCHAR(100)) ENGINE=NDB COMMENT="NDB_TABLE=READ_BACKUP=0,PARTITION_BALANCE=FOR_RP_BY_NODE";"#,
            r#"CREATE TABLE t1 (c1 INT) ENGINE_ATTRIBUTE='{"key":"value"}';"#,
            "CREATE TABLE t1 (col1 INT, col2 CHAR(5)) PARTITION BY HASH(col1);",
            "CREATE TABLE t1 (col1 INT, col2 CHAR(5), col3 DATETIME) PARTITION BY HASH (YEAR(col3));",
            "CREATE TABLE tk (col1 INT, col2 CHAR(5), col3 DATE) PARTITION BY KEY(col3) PARTITIONS 4;",
            "CREATE TABLE tk (col1 INT, col2 CHAR(5), col3 DATE) PARTITION BY LINEAR KEY(col3) PARTITIONS 5;",
            // "CREATE TABLE t1 (year_col INT, some_data INT) PARTITION BY RANGE (year_col) (
            //     PARTITION p0 VALUES LESS THAN (1991),
            //     PARTITION p1 VALUES LESS THAN (1995),
            //     PARTITION p2 VALUES LESS THAN (1999),
            //     PARTITION p3 VALUES LESS THAN (2002),
            //     PARTITION p4 VALUES LESS THAN (2006),
            //     PARTITION p5 VALUES LESS THAN MAXVALUE
            //  );",
            "CREATE TABLE t1 (year_col INT, some_data INT) PARTITION BY RANGE (year_col) (
                PARTITION p0 VALUES LESS THAN (1991),
                PARTITION p1 VALUES LESS THAN (1995),
                PARTITION p2 VALUES LESS THAN (1999),
                PARTITION p3 VALUES LESS THAN (2002),
                PARTITION p4 VALUES LESS THAN (2006),
                PARTITION p5 VALUES LESS THAN (MAXVALUE)
             );",
            "CREATE TABLE rc (a INT NOT NULL, b INT NOT NULL) PARTITION BY RANGE COLUMNS(a,b) (
                PARTITION p0 VALUES LESS THAN (10,5),
                PARTITION p1 VALUES LESS THAN (20,10),
                PARTITION p2 VALUES LESS THAN (50,MAXVALUE),
                PARTITION p3 VALUES LESS THAN (65,MAXVALUE),
                PARTITION p4 VALUES LESS THAN (MAXVALUE,MAXVALUE)
             );",
            "CREATE TABLE client_firms (id INT, name VARCHAR(35)) PARTITION BY LIST (id) (
                PARTITION r0 VALUES IN (1, 5, 9, 13, 17, 21),
                PARTITION r1 VALUES IN (2, 6, 10, 14, 18, 22),
                PARTITION r2 VALUES IN (3, 7, 11, 15, 19, 23),
                PARTITION r3 VALUES IN (4, 8, 12, 16, 20, 24)
             );",
            "CREATE TABLE lc (a INT NULL, b INT NULL) PARTITION BY LIST COLUMNS(a,b) (
                PARTITION p0 VALUES IN( (0,0), (NULL,NULL) ),
                PARTITION p1 VALUES IN( (0,1), (0,2), (0,3), (1,1), (1,2) ),
                PARTITION p2 VALUES IN( (1,0), (2,0), (2,1), (3,0), (3,1) ),
                PARTITION p3 VALUES IN( (1,3), (2,2), (2,3), (3,2), (3,3) )
             );",
            "CREATE TABLE th (id INT, name VARCHAR(30), adate DATE) PARTITION BY LIST(YEAR(adate)) (
                PARTITION p1999 VALUES IN (1995, 1999, 2003) DATA DIRECTORY = '/var/appdata/95/data' INDEX DIRECTORY = '/var/appdata/95/idx',
                PARTITION p2000 VALUES IN (1996, 2000, 2004) DATA DIRECTORY = '/var/appdata/96/data' INDEX DIRECTORY = '/var/appdata/96/idx',
                PARTITION p2001 VALUES IN (1997, 2001, 2005) DATA DIRECTORY = '/var/appdata/97/data' INDEX DIRECTORY = '/var/appdata/97/idx',
                PARTITION p2002 VALUES IN (1998, 2002, 2006) DATA DIRECTORY = '/var/appdata/98/data' INDEX DIRECTORY = '/var/appdata/98/idx'
             );",
            "CREATE TABLE t1 (s1 INT, s2 INT AS (EXP(s1)) STORED) PARTITION BY LIST (s2) (PARTITION p1 VALUES IN (1));",
        ];

        parser(inputs);
    }

    #[test]
    fn test_dal_stmt() {
        let inputs = vec![
            "CREATE USER 'root' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin';",
            "CREATE USER IF NOT EXISTS 'jeffrey'@'localhost' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin';",
            "CREATE USER 'jeffrey'@'localhost' IDENTIFIED WITH auth_plugin DEFAULT ROLE 'admin';",
            "CREATE USER 'jeffrey'@'localhost' IDENTIFIED WITH auth_plugin BY RANDOM PASSWORD DEFAULT ROLE 'admin';",
            "CREATE USER 'jeffrey'@'localhost' IDENTIFIED WITH auth_plugin INITIAL AUTHENTICATION IDENTIFIED BY RANDOM PASSWORD DEFAULT ROLE 'admin';",
            "CREATE USER 'jeffrey'@'localhost' IDENTIFIED BY 'auth_string' AND IDENTIFIED BY RANDOM PASSWORD AND IDENTIFIED WITH auth_plugin BY 'auth_string' DEFAULT ROLE 'admin';",
            "CREATE USER 'jeffrey'@'localhost' IDENTIFIED BY 'auth_string' AND IDENTIFIED BY RANDOM PASSWORD AND IDENTIFIED WITH auth_plugin BY 'auth_string' DEFAULT ROLE 'admin';",
            "CREATE USER 'root' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin' REQUIRE X509;",
            "CREATE USER 'root' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin' REQUIRE ISSUER 'issuer';",
            "CREATE USER 'root' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin' WITH MAX_QUERIES_PER_HOUR 100 MAX_CONNECTIONS_PER_HOUR 10;",
            "CREATE USER 'root' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin' PASSWORD HISTORY DEFAULT;",
            "CREATE USER 'root' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin' PASSWORD EXPIRE INTERVAL 10 DAY;",
            "CREATE USER 'root' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin' ACCOUNT LOCK;",
            "CREATE USER 'root' IDENTIFIED BY 'auth_string' DEFAULT ROLE 'admin' COMMENT 'comment_string';",
        ];

        parser(inputs);
    }

    fn parser(inputs: Vec<&str>) {
        let p = Parser::new();
        for input in inputs {
            let res = p.parse(input);
            assert!(res.is_ok());
            match res {
                Err(e) => {
                    println!("sql={:?} {:?}", input, e)
                }
                Ok(_stmt) => {
                    // println!("sql={:?} {:#?}", input, _stmt)
                }
            }
        }
    }
}
