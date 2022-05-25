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

use iota::iota;

pub const MIN_PROTOCOL_VERSION: u8 = 10;
pub const MAX_PAYLOAD_LEN: usize = (1 << 24) - 1;

pub const OK_HEADER: u8 = 0x00;
pub const ERR_HEADER: u8 = 0xff;
pub const EOF_HEADER: u8 = 0xfe;
pub const LOCALINFILE_HEADER: u8 = 0xfb;
pub const MORE_DATA_HEADER: u8 = 0x01;
pub const LOCAL_IN_FILE_HEADER: u8 = 0xfb;
pub const CACHE_SHA2_FAST_AUTH: u8 = 0x03;
pub const CACHE_SHA2_FULL_AUTH: u8 = 0x04;

pub const SERVER_STATUS_IN_TRANS: u16 = 0x0001;
pub const SERVER_STATUS_AUTOCOMMIT: u16 = 0x0002;
pub const SERVER_MORE_RESULTS_EXISTS: u16 = 0x0008;
pub const SERVER_STATUS_NO_GOOD_INDEX_USED: u16 = 0x0010;
pub const SERVER_STATUS_NO_INDEX_USED: u16 = 0x0020;
pub const SERVER_STATUS_CURSOR_EXISTS: u16 = 0x0040;
pub const SERVER_STATUS_LAST_ROW_SEND: u16 = 0x0080;
pub const SERVER_STATUS_DB_DROPPED: u16 = 0x0100;
pub const SERVER_STATUS_NO_BACKSLASH_ESCAPED: u16 = 0x0200;
pub const SERVER_STATUS_METADATA_CHANGED: u16 = 0x0400;
pub const SERVER_QUERY_WAS_SLOW: u16 = 0x0800;
pub const SERVER_PS_OUT_PARAMS: u16 = 0x1000;
pub const SERVER_SESSION_STATE_CHANGED: u16 = 0x4000;

//TODO: change to enum
pub const AUTH_MYSQL_OLD_PASSWORD: &str = "mysql_old_password";
pub const AUTH_CACHING_SHA2_PASSWORD: &str = "caching_sha2_password";
pub const AUTH_SHA256_PASSWORD: &str = "sha256_password";
pub const AUTH_NATIVE_PASSWORD: &str = "mysql_native_password";

iota! {
    pub const COM_SLEEP :u8 = iota;
         ,COM_QUIT
         ,COM_INIT_DB
         ,COM_QUERY
         ,COM_FIELD_LIST
         ,COM_CREATE_DB
         ,COM_DROP_DB
         ,COM_REFRESH
         ,COM_SHUTDOWN
         ,COM_STATISTICS
         ,COM_PROCESS_INFO
         ,COM_CONNECT
         ,COM_PROCESS_KILL
         ,COM_DEBUG
         ,COM_PING
         ,COM_TIME
         ,COM_DELAYED_INSERT
         ,COM_CHANGE_USER
         ,COM_BINLOG_DUMP
         ,COM_TABLE_DUMP
         ,COM_CONNECT_OUT
         ,COM_REGISTER_SLAVE
         ,COM_STMT_PREPARE
         ,COM_STMT_EXECUTE
         ,COM_STMT_SEND_LONG_DATA
         ,COM_STMT_CLOSE
         ,COM_STMT_RESET
         ,COM_SET_OPTION
         ,COM_STMT_FETCH
         ,COM_DAEMON
         ,COM_BINLOG_DUMP_GTID
         ,COM_RESET_CONNECTION
}

iota! {
    pub const CLIENT_LONG_PASSWORD: u32 = 1 << iota;
         ,CLIENT_FOUND_ROWS
         ,CLIENT_LONG_FLAG
         ,CLIENT_CONNECT_WITH_DB
         ,CLIENT_NO_SCHEMA
         ,CLIENT_COMPRESS
         ,CLIENT_ODBC
         ,CLIENT_LOCAL_FILES
         ,CLIENT_IGNORE_SPACE
         ,CLIENT_PROTOCOL_41
         ,CLIENT_INTERACTIVE
         ,CLIENT_SSL
         ,CLIENT_IGNORE_SIGPIPE
         ,CLIENT_TRANSACTIONS
         ,CLIENT_RESERVED
         ,CLIENT_SECURE_CONNECTION
         ,CLIENT_MULTI_STATEMENTS
         ,CLIENT_MULTI_RESULTS
         ,CLIENT_PS_MULTI_RESULTS
         ,CLIENT_PLUGIN_AUTH
         ,CLIENT_CONNECT_ATTRS
         ,CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA
         ,CLIENT_CAN_HANDLE_EXPIRED_PASSWORDS
         ,CLIENT_SESSION_TRACK
}

iota! {
    pub const MYSQL_TYPE_DECIMAL: u8 = iota;
         ,MYSQL_TYPE_TINY
         ,MYSQL_TYPE_SHORT
         ,MYSQL_TYPE_LONG
         ,MYSQL_TYPE_FLOAT
         ,MYSQL_TYPE_DOUBLE
         ,MYSQL_TYPE_NULL
         ,MYSQL_TYPE_TIMESTAMP
         ,MYSQL_TYPE_LONGLONG
         ,MYSQL_TYPE_INT24
         ,MYSQL_TYPE_DATE
         ,MYSQL_TYPE_TIME
         ,MYSQL_TYPE_DATETIME
         ,MYSQL_TYPE_YEAR
         ,MYSQL_TYPE_NEWDATE
         ,MYSQL_TYPE_VARCHAR
         ,MYSQL_TYPE_BIT
}

iota! {
    const MYSQL_TYPE_NEWDECIMAL: u8 = iota + 0xf6;
         ,MYSQL_TYPE_ENUM
         ,MYSQL_TYPE_SET
         ,MYSQL_TYPE_TINY_BLOB
         ,MYSQL_TYPE_MEDIUM_BLOB
         ,MYSQL_TYPE_LONG_BLOB
         ,MYSQL_TYPE_BLOB
         ,MYSQL_TYPE_VAR_STRING
         ,MYSQL_TYPE_STRING
         ,MYSQL_TYPE_GEOMETRY
}

#[allow(dead_code)]
const NOT_NULL_FLAG: i64 = 1;
#[allow(dead_code)]
const PRI_KEY_FLAG: i64 = 2;
#[allow(dead_code)]
const UNIQUE_KEY_FLAG: i64 = 4;
#[allow(dead_code)]
const BLOB_FLAG: i64 = 16;
#[allow(dead_code)]
const UNSIGNED_FLAG: i64 = 32;
#[allow(dead_code)]
const ZEROFILL_FLAG: i64 = 64;
#[allow(dead_code)]
const BINARY_FLAG: i64 = 128;
#[allow(dead_code)]
const ENUM_FLAG: i64 = 256;
#[allow(dead_code)]
const AUTO_INCREMENT_FLAG: i64 = 512;
#[allow(dead_code)]
const TIMESTAMP_FLAG: i64 = 1024;
#[allow(dead_code)]
const SET_FLAG: i64 = 2048;
#[allow(dead_code)]
const NUM_FLAG: i64 = 32768;
#[allow(dead_code)]
const PART_KEY_FLAG: i64 = 16384;
#[allow(dead_code)]
const GROUP_FLAG: i64 = 32768;
#[allow(dead_code)]
const UNIQUE_FLAG: i64 = 65536;
#[allow(dead_code)]
const AUTH_NAME: &str = "mysql_native_password";

pub const CACHING_SHA2PASSORD_REQUEST_PUBLIC_KEY: i64 = 2;
pub const CACHING_SHA2PASSWORD_FAST_AUTH_SUCCESS: i64 = 3;
pub const CACHING_SHA2PASSWORD_PERFORM_FULL_AUTHENTICATION: i64 = 4;

use num_derive::FromPrimitive;

#[derive(Debug, Eq, PartialEq, FromPrimitive)]
#[repr(u8)]
pub enum Com {
    Sleep = 0,
    Quit,
    InitDb,
    Query,
    FieldList,
    CreateDb,
    DropDb,
    Refresh,
    Shutdown,
    Statistics,
    ProcessInfo,
    Connect,
    ProcessKill,
    Debug,
    Ping,
    Time,
    DelayedInsert,
    ChangeUser,
    BinlogDump,
    TableDump,
    ConnectOut,
    RegisterSlave,
    StmtPrepare,
    StmtExecute,
    StmtSendLongData,
    StmtClose,
    StmtReset,
    SetOption,
    StmtFetch,
    Daemon,
    BinlogDumpGtid,
    ResetConnection,
}

#[test]
fn test_const() {
    assert_eq!(NOT_NULL_FLAG, 1);
}
