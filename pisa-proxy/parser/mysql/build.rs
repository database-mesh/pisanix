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

use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

use cfgrammar::yacc::YaccKind;
use lrlex::{ct_token_map, DefaultLexeme};
use lrpar::CTParserBuilder;
use syn::{Item, Visibility};

const TOKENS_MAP: &[(&str, &str)] = &[
    ("(", "LPAREN"),
    (")", "RPAREN"),
    ("[", "LBRACK"),
    ("]", "RBRACK"),
    ("{", "LBRACE"),
    ("}", "RBRACE"),
    (">", "GT"),
    ("<", "LT"),
    ("=", "EQ"),
    (":=", "ASSIGN_EQ"),
    ("!", "NOT"),
    ("|", "OR_OP"),
    ("&", "AND_OP"),
    ("+", "PLUS"),
    ("-", "DASH"),
    ("*", "ASTERISK"),
    ("%", "PERCENT"),
    ("~", "TILDE"),
    ("!", "EXCLAMARK"),
    ("?", "PARAMMARK"),
    (":", "COLON"),
    (";", "SEMICOLON"),
    (",", "COMMA"),
    ("/", "SLASH"),
    ("^", "CARET"),
    (".", "DOT"),
    ("\\", "BACKSLASH"),
    ("&&", "AND_AND"),
    ("<=", "LE"),
    ("!=", "NE"),
    ("<>", "NE_"),
    (">=", "GE"),
    ("<<", "SHIFT_LEFT"),
    (">>", "SHIFT_RIGHT"),
    ("<=>", "EQ_"),
    ("@", "AT"),
    ("~", "NEG"),
    ("ACCESSIBLE", "ACCESSIBLE"),
    ("ACCOUNT", "ACCOUNT"),
    ("ACTION", "ACTION"),
    ("ACTIVE", "ACTIVE"),
    ("ADD", "ADD"),
    ("ADMIN", "ADMIN"),
    ("AFTER", "AFTER"),
    ("AGAINST", "AGAINST"),
    ("AGGREGATE", "AGGREGATE"),
    ("ALL", "ALL"),
    ("ALGORITHM", "ALGORITHM"),
    ("ALTER", "ALTER"),
    ("ALWAYS", "ALWAYS"),
    ("ANALYZE", "ANALYZE"),
    ("AND", "AND"),
    ("ANY", "ANY"),
    ("ARRAY", "ARRAY"),
    ("AS", "AS"),
    ("ASC", "ASC"),
    ("ASCII", "ASCII"),
    ("ASENSITIVE", "ASENSITIVE"),
    ("AT", "AT_SYM"),
    ("ATTRIBUTE", "ATTRIBUTE"),
    ("AUTHENTICATION", "AUTHENTICATION"),
    ("AUTO_INCREMENT", "AUTO_INC"),
    ("AUTOEXTEND_SIZE", "AUTOEXTEND_SIZE"),
    ("AVG", "AVG"),
    ("AVG_ROW_LENGTH", "AVG_ROW_LENGTH"),
    ("BACKUP", "BACKUP"),
    ("BEFORE", "BEFORE"),
    ("BEGIN", "BEGIN"),
    ("BIGINT", "BIGINT"),
    ("BINARY", "BINARY"),
    ("BINLOG", "BINLOG"),
    ("BIT", "BIT"),
    ("BLOB", "BLOB"),
    ("BLOCK", "BLOCK"),
    ("BOOL", "BOOL"),
    ("BOOLEAN", "BOOLEAN"),
    ("BOTH", "BOTH"),
    ("BTREE", "BTREE"),
    ("BUCKETS", "BUCKETS"),
    ("BY", "BY"),
    ("BYTE", "BYTE"),
    ("CACHE", "CACHE"),
    ("CALL", "CALL"),
    ("CASCADE", "CASCADE"),
    ("CASCADED", "CASCADED"),
    ("CASE", "CASE"),
    ("CATALOG_NAME", "CATALOG_NAME"),
    ("CHAIN", "CHAIN"),
    ("CHALLENGE_RESPONSE", "CHALLENGE_RESPONSE"),
    ("CHANGE", "CHANGE"),
    ("CHANGED", "CHANGED"),
    ("CHANNEL", "CHANNEL"),
    ("CHAR", "CHAR"),
    ("CHARACTER", "CHARACTER"),
    ("CHARSET", "CHARSET"),
    ("CHECK", "CHECK"),
    ("CHECKSUM", "CHECKSUM"),
    ("CIPHER", "CIPHER"),
    ("CLASS_ORIGIN", "CLASS_ORIGIN"),
    ("CLIENT", "CLIENT"),
    ("CLONE", "CLONE"),
    ("CLOSE", "CLOSE"),
    ("COALESCE", "COALESCE"),
    ("CODE", "CODE"),
    ("COLLATE", "COLLATE"),
    ("COLLATION", "COLLATION"),
    ("COLUMN", "COLUMN"),
    ("COLUMN_FORMAT", "COLUMN_FORMAT"),
    ("COLUMN_NAME", "COLUMN_NAME"),
    ("COLUMNS", "COLUMNS"),
    ("COMMENT", "COMMENT"),
    ("COMMIT", "COMMIT"),
    ("COMMITTED", "COMMITTED"),
    ("COMPACT", "COMPACT"),
    ("COMPLETION", "COMPLETION"),
    ("COMPONENT", "COMPONENT"),
    ("COMPRESSION", "COMPRESSION"),
    ("COMPRESSED", "COMPRESSED"),
    ("ENCRYPTION", "ENCRYPTION"),
    ("CONCURRENT", "CONCURRENT"),
    ("CONDITION", "CONDITION"),
    ("CONNECTION", "CONNECTION"),
    ("CONSISTENT", "CONSISTENT"),
    ("CONSTRAINT", "CONSTRAINT"),
    ("CONSTRAINT_CATALOG", "CONSTRAINT_CATALOG"),
    ("CONSTRAINT_NAME", "CONSTRAINT_NAME"),
    ("CONSTRAINT_SCHEMA", "CONSTRAINT_SCHEMA"),
    ("CONTAINS", "CONTAINS"),
    ("CONTEXT", "CONTEXT"),
    ("CONTINUE", "CONTINUE"),
    ("CONVERT", "CONVERT"),
    ("CPU", "CPU"),
    ("CREATE", "CREATE"),
    ("CROSS", "CROSS"),
    ("CUBE", "CUBE"),
    ("CUME_DIST", "CUME_DIST"),
    ("CURRENT", "CURRENT"),
    ("CURRENT_DATE", "CURDATE"),
    ("CURRENT_TIME", "CURTIME"),
    ("CURRENT_TIMESTAMP", "NOW_CURRENT"),
    ("CURRENT_USER", "CURRENT_USER"),
    ("CURSOR", "CURSOR"),
    ("CURSOR_NAME", "CURSOR_NAME"),
    ("DATA", "DATA"),
    ("DATABASE", "DATABASE"),
    ("DATABASES", "DATABASES"),
    ("DATAFILE", "DATAFILE"),
    ("DATE", "DATE"),
    ("DATETIME", "DATETIME"),
    ("DAY", "DAY"),
    ("DAY_HOUR", "DAY_HOUR"),
    ("DAY_MICROSECOND", "DAY_MICROSECOND"),
    ("DAY_MINUTE", "DAY_MINUTE"),
    ("DAY_SECOND", "DAY_SECOND"),
    ("DEALLOCATE", "DEALLOCATE"),
    ("DEC", "DECIMAL"),
    ("DECIMAL", "DECIMAL_"),
    ("DECLARE", "DECLARE"),
    ("DEFAULT", "DEFAULT"),
    ("DEFAULT_AUTH", "DEFAULT_AUTH"),
    ("DEFINER", "DEFINER"),
    ("DEFINITION", "DEFINITION"),
    ("DELAYED", "DELAYED"),
    ("DELAY_KEY_WRITE", "DELAY_KEY_WRITE"),
    ("DENSE_RANK", "DENSE_RANK"),
    ("DESC", "DESC"),
    ("DESCRIBE", "DESCRIBE"),
    ("DESCRIPTION", "DESCRIPTION"),
    ("DETERMINISTIC", "DETERMINISTIC"),
    ("DIAGNOSTICS", "DIAGNOSTICS"),
    ("DIRECTORY", "DIRECTORY"),
    ("DISABLE", "DISABLE"),
    ("DISCARD", "DISCARD"),
    ("DISK", "DISK"),
    ("DISTINCT", "DISTINCT"),
    ("DISTINCTROW", "DISTINCTROW"),
    ("DIV", "DIV"),
    ("DO", "DO"),
    ("DOUBLE", "DOUBLE"),
    ("DROP", "DROP"),
    ("DUAL", "DUAL"),
    ("DUMPFILE", "DUMPFILE"),
    ("DUPLICATE", "DUPLICATE"),
    ("DYNAMIC", "DYNAMIC"),
    ("EACH", "EACH"),
    ("ELSE", "ELSE"),
    ("ELSEIF", "ELSEIF"),
    ("EMPTY", "EMPTY"),
    ("ENABLE", "ENABLE"),
    ("ENCLOSED", "ENCLOSED"),
    ("END", "END"),
    ("ENDS", "ENDS"),
    ("ENFORCED", "ENFORCED"),
    ("ENGINE", "ENGINE"),
    ("ENGINE_ATTRIBUTE", "ENGINE_ATTRIBUTE"),
    ("ENGINES", "ENGINES"),
    ("ENUM", "ENUM"),
    ("ERROR", "ERROR"),
    ("ERRORS", "ERRORS"),
    ("ESCAPE", "ESCAPE"),
    ("ESCAPED", "ESCAPED"),
    ("EVENT", "EVENT"),
    ("EVENTS", "EVENTS"),
    ("EVERY", "EVERY"),
    ("EXCEPT", "EXCEPT"),
    ("EXCHANGE", "EXCHANGE"),
    ("EXCLUDE", "EXCLUDE"),
    ("EXECUTE", "EXECUTE"),
    ("EXISTS", "EXISTS"),
    ("EXIT", "EXIT"),
    ("EXPANSION", "EXPANSION"),
    ("EXPORT", "EXPORT"),
    ("EXPIRE", "EXPIRE"),
    ("EXPLAIN", "DESCRIBE"),
    ("EXTENDED", "EXTENDED"),
    ("EXTENT_SIZE", "EXTENT_SIZE"),
    ("FACTOR", "FACTOR"),
    ("FAILED_LOGIN_ATTEMPTS", "FAILED_LOGIN_ATTEMPTS"),
    ("FALSE", "FALSE"),
    ("FAST", "FAST"),
    ("FAULTS", "FAULTS"),
    ("FETCH", "FETCH"),
    ("FIELDS", "FIELDS"),
    ("FILE", "FILE"),
    ("FILE_BLOCK_SIZE", "FILE_BLOCK_SIZE"),
    ("FILTER", "FILTER"),
    ("FINISH", "FINISH"),
    ("FIRST", "FIRST"),
    ("FIRST_VALUE", "FIRST_VALUE"),
    ("FIXED", "FIXED"),
    ("FLOAT", "FLOAT"),
    ("FLOAT4", "FLOAT_"),
    ("FLOAT8", "DOUBLE"),
    ("FLUSH", "FLUSH"),
    ("FOLLOWS", "FOLLOWS"),
    ("FOLLOWING", "FOLLOWING"),
    ("FOR", "FOR"),
    ("FORCE", "FORCE"),
    ("FOREIGN", "FOREIGN"),
    ("FORMAT", "FORMAT"),
    ("FOUND", "FOUND"),
    ("FROM", "FROM"),
    ("FULL", "FULL"),
    ("FULLTEXT", "FULLTEXT"),
    ("FUNCTION", "FUNCTION"),
    ("GENERAL", "GENERAL"),
    ("GROUP_REPLICATION", "GROUP_REPLICATION"),
    ("GEOMCOLLECTION", "GEOMETRYCOLLECTION"),
    ("GEOMETRY", "GEOMETRY"),
    ("GEOMETRYCOLLECTION", "GEOMETRYCOLLECTION"),
    ("GET_FORMAT", "GET_FORMAT"),
    ("GET_MASTER_PUBLIC_KEY", "GET_MASTER_PUBLIC_KEY"),
    ("GET_SOURCE_PUBLIC_KEY", "GET_SOURCE_PUBLIC_KEY"),
    ("GET", "GET"),
    ("GENERATED", "GENERATED"),
    ("GLOBAL", "GLOBAL"),
    ("GRANT", "GRANT"),
    ("GRANTS", "GRANTS"),
    ("GROUP", "GROUP"),
    ("GROUPING", "GROUPING"),
    ("GROUPS", "GROUPS"),
    ("GTID_ONLY", "GTID_ONLY"),
    ("HANDLER", "HANDLER"),
    ("HASH", "HASH"),
    ("HAVING", "HAVING"),
    ("HELP", "HELP"),
    ("HIGH_PRIORITY", "HIGH_PRIORITY"),
    ("HISTOGRAM", "HISTOGRAM"),
    ("HISTORY", "HISTORY"),
    ("HOST", "HOST"),
    ("HOSTS", "HOSTS"),
    ("HOUR", "HOUR"),
    ("HOUR_MICROSECOND", "HOUR_MICROSECOND"),
    ("HOUR_MINUTE", "HOUR_MINUTE"),
    ("HOUR_SECOND", "HOUR_SECOND"),
    ("IDENTIFIED", "IDENTIFIED"),
    ("IF", "IF"),
    ("IGNORE", "IGNORE"),
    ("IGNORE_SERVER_IDS", "IGNORE_SERVER_IDS"),
    ("IMPORT", "IMPORT"),
    ("IN", "IN"),
    ("INACTIVE", "INACTIVE"),
    ("INDEX", "INDEX"),
    ("INDEXES", "INDEXES"),
    ("INFILE", "INFILE"),
    ("INITIAL", "INITIAL"),
    ("INITIAL_SIZE", "INITIAL_SIZE"),
    ("INITIATE", "INITIATE"),
    ("INNER", "INNER"),
    ("INOUT", "INOUT"),
    ("INSENSITIVE", "INSENSITIVE"),
    ("INSERT_METHOD", "INSERT_METHOD"),
    ("INSTALL", "INSTALL"),
    ("INSTANCE", "INSTANCE"),
    ("INT", "INT"),
    ("INT1", "TINYINT"),
    ("INT2", "SMALLINT"),
    ("INT3", "MEDIUMINT"),
    ("INT4", "INT"),
    ("INT8", "BIGINT"),
    ("INTEGER", "INT"),
    ("INTERVAL", "INTERVAL"),
    ("INTO", "INTO"),
    ("IO", "IO"),
    ("IO_AFTER_GTIDS", "IO_AFTER_GTIDS"),
    ("IO_BEFORE_GTIDS", "IO_BEFORE_GTIDS"),
    ("IO_THREAD", "RELAY_THREAD"),
    ("IPC", "IPC"),
    ("IS", "IS"),
    ("ISOLATION", "ISOLATION"),
    ("ISSUER", "ISSUER"),
    ("ITERATE", "ITERATE"),
    ("INVISIBLE", "INVISIBLE"),
    ("INVOKER", "INVOKER"),
    ("JOIN", "JOIN"),
    ("JSON", "JSON"),
    ("JSON_TABLE", "JSON_TABLE"),
    ("JSON_VALUE", "JSON_VALUE"),
    ("KEY", "KEY"),
    ("KEYRING", "KEYRING"),
    ("KEYS", "KEYS"),
    ("KEY_BLOCK_SIZE", "KEY_BLOCK_SIZE"),
    ("KILL", "KILL"),
    ("LAG", "LAG"),
    ("LANGUAGE", "LANGUAGE"),
    ("LAST", "LAST"),
    ("LAST_VALUE", "LAST_VALUE"),
    ("LATERAL", "LATERAL"),
    ("LEAD", "LEAD"),
    ("LEADING", "LEADING"),
    ("LEAVE", "LEAVE"),
    ("LEAVES", "LEAVES"),
    ("LEFT", "LEFT"),
    ("LESS", "LESS"),
    ("LEVEL", "LEVEL"),
    ("LIKE", "LIKE"),
    ("LIMIT", "LIMIT"),
    ("LINEAR", "LINEAR"),
    ("LINES", "LINES"),
    ("LINESTRING", "LINESTRING"),
    ("LIST", "LIST"),
    ("LOAD", "LOAD"),
    ("LOCAL", "LOCAL"),
    ("LOCALTIME", "NOW_LOCAL1"),
    ("LOCALTIMESTAMP", "NOW_LOCAL2"),
    ("LOCK", "LOCK"),
    ("LOCKED", "LOCKED"),
    ("LOCKS", "LOCKS"),
    ("LOGFILE", "LOGFILE"),
    ("LOGS", "LOGS"),
    ("LONG", "LONG"),
    ("LONGBLOB", "LONGBLOB"),
    ("LONGTEXT", "LONGTEXT"),
    ("LOOP", "LOOP"),
    ("LOW_PRIORITY", "LOW_PRIORITY"),
    ("MASTER", "MASTER"),
    ("MASTER_AUTO_POSITION", "MASTER_AUTO_POSITION"),
    ("MASTER_BIND", "MASTER_BIND"),
    ("MASTER_CONNECT_RETRY", "MASTER_CONNECT_RETRY"),
    ("MASTER_COMPRESSION_ALGORITHMS", "MASTER_COMPRESSION_ALGORITHM"),
    ("MASTER_DELAY", "MASTER_DELAY"),
    ("MASTER_HEARTBEAT_PERIOD", "MASTER_HEARTBEAT_PERIOD"),
    ("MASTER_HOST", "MASTER_HOST"),
    ("MASTER_LOG_FILE", "MASTER_LOG_FILE"),
    ("MASTER_LOG_POS", "MASTER_LOG_POS"),
    ("MASTER_PASSWORD", "MASTER_PASSWORD"),
    ("MASTER_PORT", "MASTER_PORT"),
    ("MASTER_PUBLIC_KEY_PATH", "MASTER_PUBLIC_KEY_PATH"),
    ("MASTER_RETRY_COUNT", "MASTER_RETRY_COUNT"),
    ("MASTER_SSL", "MASTER_SSL"),
    ("MASTER_SSL_CA", "MASTER_SSL_CA"),
    ("MASTER_SSL_CAPATH", "MASTER_SSL_CAPATH"),
    ("MASTER_SSL_CERT", "MASTER_SSL_CERT"),
    ("MASTER_SSL_CIPHER", "MASTER_SSL_CIPHER"),
    ("MASTER_SSL_CRL", "MASTER_SSL_CRL"),
    ("MASTER_SSL_CRLPATH", "MASTER_SSL_CRLPATH"),
    ("MASTER_SSL_KEY", "MASTER_SSL_KEY"),
    ("MASTER_SSL_VERIFY_SERVER_CERT", "MASTER_SSL_VERIFY_SERVER_CERT"),
    ("MASTER_TLS_CIPHERSUITES", "MASTER_TLS_CIPHERSUITES"),
    ("MASTER_TLS_VERSION", "MASTER_TLS_VERSION"),
    ("MASTER_USER", "MASTER_USER"),
    ("MASTER_ZSTD_COMPRESSION_LEVEL", "MASTER_ZSTD_COMPRESSION_LEVEL"),
    ("MATCH", "MATCH"),
    ("MAX_CONNECTIONS_PER_HOUR", "MAX_CONNECTIONS_PER_HOUR"),
    ("MAX_QUERIES_PER_HOUR", "MAX_QUERIES_PER_HOUR"),
    ("MAX_ROWS", "MAX_ROWS"),
    ("MAX_SIZE", "MAX_SIZE"),
    ("MAX_UPDATES_PER_HOUR", "MAX_UPDATES_PER_HOUR"),
    ("MAX_USER_CONNECTIONS", "MAX_USER_CONNECTIONS"),
    ("MAXVALUE", "MAX_VALUE"),
    ("MEDIUM", "MEDIUM"),
    ("MEDIUMBLOB", "MEDIUMBLOB"),
    ("MEDIUMINT", "MEDIUMINT"),
    ("MEDIUMTEXT", "MEDIUMTEXT"),
    ("MEMBER", "MEMBER"),
    ("MEMORY", "MEMORY"),
    ("MERGE", "MERGE"),
    ("MESSAGE_TEXT", "MESSAGE_TEXT"),
    ("MICROSECOND", "MICROSECOND"),
    ("MIDDLEINT", "MEDIUMINT"),
    ("MIGRATE", "MIGRATE"),
    ("MINUTE", "MINUTE"),
    ("MINUTE_MICROSECOND", "MINUTE_MICROSECOND"),
    ("MINUTE_SECOND", "MINUTE_SECOND"),
    ("MIN_ROWS", "MIN_ROWS"),
    ("MOD", "MOD"),
    ("MODE", "MODE"),
    ("MODIFIES", "MODIFIES"),
    ("MODIFY", "MODIFY"),
    ("MONTH", "MONTH"),
    ("MULTILINESTRING", "MULTILINESTRING"),
    ("MULTIPOINT", "MULTIPOINT"),
    ("MULTIPOLYGON", "MULTIPOLYGON"),
    ("MUTEX", "MUTEX"),
    ("MYSQL_ERRNO", "MYSQL_ERRNO"),
    ("NAME", "NAME"),
    ("NAMES", "NAMES"),
    ("NATIONAL", "NATIONAL"),
    ("NATURAL", "NATURAL"),
    ("NDB", "NDBCLUSTER"),
    ("NDBCLUSTER", "NDBCLUSTER_"),
    ("NCHAR", "NCHAR"),
    ("NESTED", "NESTED"),
    ("NETWORK_NAMESPACE", "NETWORK_NAMESPACE"),
    ("NEVER", "NEVER"),
    ("NEW", "NEW"),
    ("NEXT", "NEXT"),
    ("NO", "NO"),
    ("NOWAIT", "NOWAIT"),
    ("NO_WAIT", "NO_WAIT"),
    ("NODEGROUP", "NODEGROUP"),
    ("NONE", "NONE"),
    ("NOT", "NOT"),
    ("NO_WRITE_TO_BINLOG", "NO_WRITE_TO_BINLOG"),
    ("NTH_VALUE", "NTH_VALUE"),
    ("NTILE", "NTILE"),
    ("NULL", "NULL"),
    ("NULLS", "NULLS"),
    ("NUMBER", "NUMBER"),
    ("NUMERIC", "NUMERIC"),
    ("NVARCHAR", "NVARCHAR"),
    ("OF", "OF"),
    ("OFF", "OFF"),
    ("OFFSET", "OFFSET"),
    ("OJ", "OJ"),
    ("OLD", "OLD"),
    ("ON", "ON"),
    ("ONE", "ONE"),
    ("ONLY", "ONLY"),
    ("OPEN", "OPEN"),
    ("OPTIMIZE", "OPTIMIZE"),
    ("OPTIMIZER_COSTS", "OPTIMIZER_COSTS"),
    ("OPTIONS", "OPTIONS"),
    ("OPTION", "OPTION"),
    ("OPTIONAL", "OPTIONAL"),
    ("OPTIONALLY", "OPTIONALLY"),
    ("OR", "OR"),
    ("ORGANIZATION", "ORGANIZATION"),
    ("OTHERS", "OTHERS"),
    ("ORDER", "ORDER"),
    ("ORDINALITY", "ORDINALITY"),
    ("OUT", "OUT"),
    ("OUTER", "OUTER"),
    ("OUTFILE", "OUTFILE"),
    ("OVER", "OVER"),
    ("OWNER", "OWNER"),
    ("PACK_KEYS", "PACK_KEYS"),
    ("PATH", "PATH"),
    ("PARSER", "PARSER"),
    ("PAGE", "PAGE"),
    ("PARTIAL", "PARTIAL"),
    ("PARTITION", "PARTITION"),
    ("PARTITIONING", "PARTITIONING"),
    ("PARTITIONS", "PARTITIONS"),
    ("PASSWORD", "PASSWORD"),
    ("PASSWORD_LOCK_TIME", "PASSWORD_LOCK_TIME"),
    ("PERCENT_RANK", "PERCENT_RANK"),
    ("PERSIST", "PERSIST"),
    ("PERSIST_ONLY", "PERSIST_ONLY"),
    ("PHASE", "PHASE"),
    ("PLUGIN", "PLUGIN"),
    ("PLUGINS", "PLUGINS"),
    ("PLUGIN_DIR", "PLUGIN_DIR"),
    ("POINT", "POINT"),
    ("POLYGON", "POLYGON"),
    ("PORT", "PORT"),
    ("PRECEDES", "PRECEDES"),
    ("PRECEDING", "PRECEDING"),
    ("PRECISION", "PRECISION"),
    ("PREPARE", "PREPARE"),
    ("PRESERVE", "PRESERVE"),
    ("PREV", "PREV"),
    ("PRIMARY", "PRIMARY"),
    ("PRIVILEGES", "PRIVILEGES"),
    ("PRIVILEGE_CHECKS_USER", "PRIVILEGE_CHECKS_USER"),
    ("PROCEDURE", "PROCEDURE"),
    ("PROCESS", "PROCESS"),
    ("PROCESSLIST", "PROCESSLIST"),
    ("PROFILE", "PROFILE"),
    ("PROFILES", "PROFILES"),
    ("PROXY", "PROXY"),
    ("PURGE", "PURGE"),
    ("QUARTER", "QUARTER"),
    ("QUERY", "QUERY"),
    ("QUICK", "QUICK"),
    ("RANDOM", "RANDOM"),
    ("RANK", "RANK"),
    ("RANGE", "RANGE"),
    ("READ", "READ"),
    ("READ_ONLY", "READ_ONLY"),
    ("READ_WRITE", "READ_WRITE"),
    ("READS", "READS"),
    ("REAL", "REAL"),
    ("REBUILD", "REBUILD"),
    ("RECOVER", "RECOVER"),
    ("RECURSIVE", "RECURSIVE"),
    ("REDO_BUFFER_SIZE", "REDO_BUFFER_SIZE"),
    ("REDUNDANT", "REDUNDANT"),
    ("REFERENCE", "REFERENCE"),
    ("REFERENCES", "REFERENCES"),
    ("REGEXP", "REGEXP"),
    ("REGISTRATION", "REGISTRATION"),
    ("RELAY", "RELAY"),
    ("RELAYLOG", "RELAYLOG"),
    ("RELAY_LOG_FILE", "RELAY_LOG_FILE"),
    ("RELAY_LOG_POS", "RELAY_LOG_POS"),
    ("RELAY_THREAD", "RELAY_THREAD"),
    ("RELEASE", "RELEASE"),
    ("RELOAD", "RELOAD"),
    ("REMOVE", "REMOVE"),
    ("RENAME", "RENAME"),
    ("ASSIGN_GTIDS_TO_ANONYMOUS_TRANSACTIONS", "ASSIGN_GTIDS_TO_ANONYMOUS_TRANSACTIONS"),
    ("REORGANIZE", "REORGANIZE"),
    ("REPAIR", "REPAIR"),
    ("REPEATABLE", "REPEATABLE"),
    ("REPLICA", "REPLICA"),
    ("REPLICAS", "REPLICAS"),
    ("REPLICATION", "REPLICATION"),
    ("REPLICATE_DO_DB", "REPLICATE_DO_DB"),
    ("REPLICATE_IGNORE_DB", "REPLICATE_IGNORE_DB"),
    ("REPLICATE_DO_TABLE", "REPLICATE_DO_TABLE"),
    ("REPLICATE_IGNORE_TABLE", "REPLICATE_IGNORE_TABLE"),
    ("REPLICATE_WILD_DO_TABLE", "REPLICATE_WILD_DO_TABLE"),
    ("REPLICATE_WILD_IGNORE_TABLE", "REPLICATE_WILD_IGNORE_TABLE"),
    ("REPLICATE_REWRITE_DB", "REPLICATE_REWRITE_DB"),
    ("REPEAT", "REPEAT"),
    ("REQUIRE", "REQUIRE"),
    ("REQUIRE_ROW_FORMAT", "REQUIRE_ROW_FORMAT"),
    ("REQUIRE_TABLE_PRIMARY_KEY_CHECK", "REQUIRE_TABLE_PRIMARY_KEY_CHECK"),
    ("RESET", "RESET"),
    ("RESPECT", "RESPECT"),
    ("RESIGNAL", "RESIGNAL"),
    ("RESOURCE", "RESOURCE"),
    ("RESTART", "RESTART"),
    ("RESTORE", "RESTORE"),
    ("RESTRICT", "RESTRICT"),
    ("RESUME", "RESUME"),
    ("RETAIN", "RETAIN"),
    ("RETURNED_SQLSTATE", "RETURNED_SQLSTATE"),
    ("RETURN", "RETURN"),
    ("RETURNING", "RETURNING"),
    ("RETURNS", "RETURNS"),
    ("REUSE", "REUSE"),
    ("REVERSE", "REVERSE"),
    ("REVOKE", "REVOKE"),
    ("RIGHT", "RIGHT"),
    ("RLIKE", "REGEXP"),
    ("ROLE", "ROLE"),
    ("ROLLBACK", "ROLLBACK"),
    ("ROLLUP", "ROLLUP"),
    ("ROUTINE", "ROUTINE"),
    ("ROTATE", "ROTATE"),
    //("ROW", "ROW"),
    ("ROW_COUNT", "ROW_COUNT"),
    ("ROW_NUMBER", "ROW_NUMBER"),
    //("ROWS", "ROWS"),
    ("ROW_FORMAT", "ROW_FORMAT"),
    ("RTREE", "RTREE"),
    ("SAVEPOINT", "SAVEPOINT"),
    ("SCHEDULE", "SCHEDULE"),
    ("SCHEMA", "DATABASE"),
    ("SCHEMA_NAME", "SCHEMA_NAME"),
    ("SCHEMAS", "DATABASES"),
    ("SECOND", "SECOND"),
    ("SECOND_MICROSECOND", "SECOND_MICROSECOND"),
    ("SECONDARY", "SECONDARY"),
    ("SECONDARY_ENGINE", "SECONDARY_ENGINE"),
    ("SECONDARY_ENGINE_ATTRIBUTE", "SECONDARY_ENGINE_ATTRIBUTE"),
    ("SECONDARY_LOAD", "SECONDARY_LOAD"),
    ("SECONDARY_UNLOAD", "SECONDARY_UNLOAD"),
    ("SECURITY", "SECURITY"),
    ("SENSITIVE", "SENSITIVE"),
    ("SEPARATOR", "SEPARATOR"),
    ("SERIAL", "SERIAL"),
    ("SERIALIZABLE", "SERIALIZABLE"),
    ("SESSION", "SESSION"),
    ("SERVER", "SERVER"),
    ("SET", "SET"),
    ("SHARE", "SHARE"),
    ("SHOW", "SHOW"),
    ("SHUTDOWN", "SHUTDOWN"),
    ("SIGNAL", "SIGNAL"),
    ("SIGNED", "SIGNED"),
    ("SIMPLE", "SIMPLE"),
    ("SKIP", "SKIP"),
    ("SLAVE", "SLAVE"),
    ("SLOW", "SLOW"),
    ("SNAPSHOT", "SNAPSHOT"),
    ("SMALLINT", "SMALLINT"),
    ("SOCKET", "SOCKET"),
    ("SOME", "SOME"),
    ("SONAME", "SONAME"),
    ("SOUNDS", "SOUNDS"),
    ("SOURCE", "SOURCE"),
    ("SOURCE_AUTO_POSITION", "SOURCE_AUTO_POSITION"),
    ("SOURCE_BIND", "SOURCE_BIND"),
    ("SOURCE_COMPRESSION_ALGORITHMS", "SOURCE_COMPRESSION_ALGORITHM"),
    ("SOURCE_CONNECT_RETRY", "SOURCE_CONNECT_RETRY"),
    ("SOURCE_CONNECTION_AUTO_FAILOVER", "SOURCE_CONNECTION_AUTO_FAILOVER"),
    ("SOURCE_DELAY", "SOURCE_DELAY"),
    ("SOURCE_HEARTBEAT_PERIOD", "SOURCE_HEARTBEAT_PERIOD"),
    ("SOURCE_HOST", "SOURCE_HOST"),
    ("SOURCE_LOG_FILE", "SOURCE_LOG_FILE"),
    ("SOURCE_LOG_POS", "SOURCE_LOG_POS"),
    ("SOURCE_PASSWORD", "SOURCE_PASSWORD"),
    ("SOURCE_PORT", "SOURCE_PORT"),
    ("SOURCE_PUBLIC_KEY_PATH", "SOURCE_PUBLIC_KEY_PATH"),
    ("SOURCE_RETRY_COUNT", "SOURCE_RETRY_COUNT"),
    ("SOURCE_SSL_CAPATH", "SOURCE_SSL_CAPATH"),
    ("SOURCE_SSL_CA", "SOURCE_SSL_CA"),
    ("SOURCE_SSL_CERT", "SOURCE_SSL_CERT"),
    ("SOURCE_SSL_CIPHER", "SOURCE_SSL_CIPHER"),
    ("SOURCE_SSL_CRL", "SOURCE_SSL_CRL"),
    ("SOURCE_SSL_CRLPATH", "SOURCE_SSL_CRLPATH"),
    ("SOURCE_SSL_KEY", "SOURCE_SSL_KEY"),
    ("SOURCE_SSL", "SOURCE_SSL"),
    ("SOURCE_SSL_VERIFY_SERVER_CERT", "SOURCE_SSL_VERIFY_SERVER_CERT"),
    ("SOURCE_TLS_CIPHERSUITES", "SOURCE_TLS_CIPHERSUITES"),
    ("SOURCE_TLS_VERSION", "SOURCE_TLS_VERSION"),
    ("SOURCE_USER", "SOURCE_USER"),
    ("SOURCE_ZSTD_COMPRESSION_LEVEL", "SOURCE_ZSTD_COMPRESSION_LEVEL"),
    ("SPATIAL", "SPATIAL"),
    ("SPECIFIC", "SPECIFIC"),
    ("SQL", "SQL"),
    ("SQLEXCEPTION", "SQLEXCEPTION"),
    ("SQLSTATE", "SQLSTATE"),
    ("SQLWARNING", "SQLWARNING"),
    ("SQL_AFTER_GTIDS", "SQL_AFTER_GTIDS"),
    ("SQL_AFTER_MTS_GAPS", "SQL_AFTER_MTS_GAPS"),
    ("SQL_BEFORE_GTIDS", "SQL_BEFORE_GTIDS"),
    ("SQL_BIG_RESULT", "SQL_BIG_RESULT"),
    ("SQL_BUFFER_RESULT", "SQL_BUFFER_RESULT"),
    ("SQL_CALC_FOUND_ROWS", "SQL_CALC_FOUND_ROWS"),
    ("SQL_NO_CACHE", "SQL_NO_CACHE"),
    ("SQL_SMALL_RESULT", "SQL_SMALL_RESULT"),
    ("SQL_THREAD", "SQL_THREAD"),
    ("SQL_TSI_SECOND", "SECOND"),
    ("SQL_TSI_MINUTE", "MINUTE"),
    ("SQL_TSI_HOUR", "HOUR"),
    ("SQL_TSI_DAY", "DAY"),
    ("SQL_TSI_WEEK", "WEEK"),
    ("SQL_TSI_MONTH", "MONTH"),
    ("SQL_TSI_QUARTER", "QUARTER"),
    ("SQL_TSI_YEAR", "YEAR"),
    ("SRID", "SRID"),
    ("SSL", "SSL"),
    ("STACKED", "STACKED"),
    ("START", "START"),
    ("STARTING", "STARTING"),
    ("STARTS", "STARTS"),
    ("STATS_AUTO_RECALC", "STATS_AUTO_RECALC"),
    ("STATS_PERSISTENT", "STATS_PERSISTENT"),
    ("STATS_SAMPLE_PAGES", "STATS_SAMPLE_PAGES"),
    ("STATUS", "STATUS"),
    ("STOP", "STOP"),
    ("STORAGE", "STORAGE"),
    ("STORED", "STORED"),
    ("STRAIGHT_JOIN", "STRAIGHT_JOIN"),
    ("STREAM", "STREAM"),
    ("STRING", "STRING"),
    ("SUBCLASS_ORIGIN", "SUBCLASS_ORIGIN"),
    ("SUBJECT", "SUBJECT"),
    ("SUBPARTITION", "SUBPARTITION"),
    ("SUBPARTITIONS", "SUBPARTITIONS"),
    ("SUPER", "SUPER"),
    ("SUSPEND", "SUSPEND"),
    ("SWAPS", "SWAPS"),
    ("SWITCHES", "SWITCHES"),
    ("SYSTEM", "SYSTEM"),
    ("TABLE", "TABLE"),
    ("TABLE_NAME", "TABLE_NAME"),
    ("TABLES", "TABLES"),
    ("TABLESPACE", "TABLESPACE"),
    ("TABLE_CHECKSUM", "TABLE_CHECKSUM"),
    ("TEMPORARY", "TEMPORARY"),
    ("TEMPTABLE", "TEMPTABLE"),
    ("TERMINATED", "TERMINATED"),
    ("TEXT", "TEXT"),
    ("THAN", "THAN"),
    ("THEN", "THEN"),
    ("THREAD_PRIORITY", "THREAD_PRIORITY"),
    ("TIES", "TIES"),
    ("TIME", "TIME"),
    ("TIMESTAMP", "TIMESTAMP"),
    ("TIMESTAMPADD", "TIMESTAMP_ADD"),
    ("TIMESTAMPDIFF", "TIMESTAMP_DIFF"),
    ("TINYBLOB", "TINYBLOB"),
    ("TINYINT", "TINYINT"),
    ("TINYTEXT", "TINYTEXT_SYN"),
    ("TLS", "TLS"),
    ("TO", "TO"),
    ("TRAILING", "TRAILING"),
    ("TRANSACTION", "TRANSACTION"),
    ("TRIGGER", "TRIGGER"),
    ("TRIGGERS", "TRIGGERS"),
    ("TRUE", "TRUE"),
    ("TRUNCATE", "TRUNCATE"),
    ("TYPE", "TYPE"),
    ("TYPES", "TYPES"),
    ("UNBOUNDED", "UNBOUNDED"),
    ("UNCOMMITTED", "UNCOMMITTED"),
    ("UNDEFINED", "UNDEFINED"),
    ("UNDO_BUFFER_SIZE", "UNDO_BUFFER_SIZE"),
    ("UNDOFILE", "UNDOFILE"),
    ("UNDO", "UNDO"),
    ("UNICODE", "UNICODE"),
    ("UNION", "UNION"),
    ("UNIQUE", "UNIQUE"),
    ("UNKNOWN", "UNKNOWN"),
    ("UNLOCK", "UNLOCK"),
    ("UNINSTALL", "UNINSTALL"),
    ("UNREGISTER", "UNREGISTER"),
    ("UNSIGNED", "UNSIGNED"),
    ("UNTIL", "UNTIL"),
    ("UPGRADE", "UPGRADE"),
    ("USAGE", "USAGE"),
    ("USE", "USE"),
    ("USER", "USER"),
    ("USER_RESOURCES", "RESOURCES"),
    ("USE_FRM", "USE_FRM"),
    ("USING", "USING"),
    ("UTC_DATE", "UTC_DATE"),
    ("UTC_TIME", "UTC_TIME"),
    ("UTC_TIMESTAMP", "UTC_TIMESTAMP"),
    ("VALIDATION", "VALIDATION"),
    ("VALUE", "VALUE"),
    ("VALUES", "VALUES"),
    ("VARBINARY", "VARBINARY"),
    ("VARCHAR", "VARCHAR"),
    ("VARCHARACTER", "VARCHAR_"),
    ("VARIABLES", "VARIABLES"),
    ("VARYING", "VARYING"),
    ("WAIT", "WAIT"),
    ("WARNINGS", "WARNINGS"),
    ("WEEK", "WEEK"),
    ("WEIGHT_STRING", "WEIGHT_STRING"),
    ("WHEN", "WHEN"),
    ("WHERE", "WHERE"),
    ("WHILE", "WHILE"),
    ("WINDOW", "WINDOW"),
    ("VCPU", "VCPU"),
    ("VIEW", "VIEW"),
    ("VIRTUAL", "VIRTUAL"),
    ("VISIBLE", "VISIBLE"),
    ("WITH", "WITH"),
    ("WITHOUT", "WITHOUT"),
    ("WORK", "WORK"),
    ("WRAPPER", "WRAPPER"),
    ("WRITE", "WRITE"),
    ("X509", "X509"),
    ("XOR", "XOR"),
    ("XA", "XA"),
    ("XID", "XID"),
    ("XML", "XML"),
    ("YEAR", "YEAR"),
    ("YEAR_MONTH", "YEAR_MONTH"),
    ("ZEROFILL", "ZEROFILL"),
    ("ZONE", "ZONE"),
    ("DELETE", "DELETE"),
    ("INSERT", "INSERT"),
    ("REPLACE", "REPLACE"),
    ("SELECT", "SELECT"),
    ("UPDATE", "UPDATE"),
    ("ADDDATE", "ADDDATE"),
    ("BIT_AND", "BIT_AND"),
    ("BIT_OR", "BIT_OR"),
    ("BIT_XOR", "BIT_XOR"),
    ("CAST", "CAST"),
    ("COUNT", "COUNT"),
    ("CURDATE", "CURDATE"),
    ("CURTIME", "CURTIME"),
    ("DATE_ADD", "DATE_ADD"),
    ("DATE_SUB", "DATE_SUB"),
    ("EXTRACT", "EXTRACT"),
    ("GROUP_CONCAT", "GROUP_CONCAT"),
    ("JSON_OBJECTAGG", "JSON_OBJECTAGG"),
    ("JSON_ARRAYAGG", "JSON_ARRAYAGG"),
    ("MAX", "MAX"),
    ("MID", "SUBSTRING"),
    ("MIN", "MIN"),
    ("NOW", "NOW"),
    ("POSITION", "POSITION"),
    ("SESSION_USER", "USER"),
    ("STD", "STD"),
    ("STDDEV", "STDDEV"),
    ("STDDEV_POP", "STDDEV_POP"),
    ("STDDEV_SAMP", "STDDEV_SAMP"),
    ("ST_COLLECT", "ST_COLLECT"),
    ("SUBDATE", "SUBDATE"),
    ("SUBSTR", "SUBSTRING"),
    ("SUBSTRING", "SUBSTRING_"),
    ("SUM", "SUM"),
    ("SYSDATE", "SYSDATE"),
    ("SYSTEM_USER", "USER"),
    ("TRIM", "TRIM"),
    ("VARIANCE", "VARIANCE"),
    ("VAR_POP", "VAR_POP"),
    ("VAR_SAMP", "VAR_SAMP"),
    ("BKA", "BKA_HINT"),
    ("BNL", "BNL_HINT"),
    ("DUPSWEEDOUT", "DUPSWEEDOUT_HINT"),
    ("FIRSTMATCH", "FIRSTMATCH_HINT"),
    ("INTOEXISTS", "INTOEXISTS_HINT"),
    ("LOOSESCAN", "LOOSESCAN_HINT"),
    ("MATERIALIZATION", "MATERIALIZATION_HINT"),
    ("MAX_EXECUTION_TIME", "MAX_EXECUTION_TIME_HINT"),
    ("NO_BKA", "NO_BKA_HINT"),
    ("NO_BNL", "NO_BNL_HINT"),
    ("NO_ICP", "NO_ICP_HINT"),
    ("NO_MRR", "NO_MRR_HINT"),
    ("NO_RANGE_OPTIMIZATION", "NO_RANGE_OPTIMIZATION_HINT"),
    ("NO_SEMIJOIN", "NO_SEMIJOIN_HINT"),
    ("MRR", "MRR_HINT"),
    ("QB_NAME", "QB_NAME_HINT"),
    ("SEMIJOIN", "SEMIJOIN_HINT"),
    ("SET_VAR", "SET_VAR_HINT"),
    ("SUBQUERY", "SUBQUERY_HINT"),
    ("MERGE", "DERIVED_MERGE_HINT"),
    ("NO_MERGE", "NO_DERIVED_MERGE_HINT"),
    ("JOIN_PREFIX", "JOIN_PREFIX_HINT"),
    ("JOIN_SUFFIX", "JOIN_SUFFIX_HINT"),
    ("JOIN_ORDER", "JOIN_ORDER_HINT"),
    ("JOIN_FIXED_ORDER", "JOIN_FIXED_ORDER_HINT"),
    ("INDEX_MERGE", "INDEX_MERGE_HINT"),
    ("NO_INDEX_MERGE", "NO_INDEX_MERGE_HINT"),
    ("RESOURCE_GROUP", "RESOURCE_GROUP_HINT"),
    ("SKIP_SCAN", "SKIP_SCAN_HINT"),
    ("NO_SKIP_SCAN", "NO_SKIP_SCAN_HINT"),
    ("HASH_JOIN", "HASH_JOIN_HINT"),
    ("NO_HASH_JOIN", "NO_HASH_JOIN_HINT"),
    ("INDEX_HINT", "INDEX_HINT"),
    ("NO_INDEX", "NO_INDEX_HINT"),
    ("JOIN_INDEX", "JOIN_INDEX_HINT"),
    ("NO_JOIN_INDEX", "NO_JOIN_INDEX_HINT"),
    ("GROUP_INDEX", "GROUP_INDEX_HINT"),
    ("NO_GROUP_INDEX", "NO_GROUP_INDEX_HINT"),
    ("ORDER_INDEX", "ORDER_INDEX_HINT"),
    ("NO_ORDER_INDEX", "NO_ORDER_INDEX_HINT"),
    ("DERIVED_CONDITION_PUSHDOWN", "DERIVED_CONDITION_PUSHDOWN_HINT"),
    ("NO_DERIVED_CONDITION_PUSHDOWN", "NO_DERIVED_CONDITION_PUSHDOWN_HINT"),
    ("BIN_NUM", "BIN_NUM"),
    ("HEX_NUM", "HEX_NUM"),
];

fn main() -> Result<(), Box<dyn Error>> {
    let cp = CTParserBuilder::<DefaultLexeme<u32>, u32>::new()
        .yacckind(YaccKind::Grmtools)
        .grammar_in_src_dir("grammar.y")?
        .build()?;

    ct_token_map::<u32>("token_map", cp.token_map(), Some(&TOKENS_MAP.iter().cloned().collect()))?;
    gen_node_enum()
}

fn gen_node_enum() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(env::var("OUT_DIR")?);
    let mut f = File::create(&out.join("ast_api.rs"))?;

    writeln!(f, "// Don't edit, the file generated by cargo build")?;
    writeln!(f, "use enum_as_inner::EnumAsInner;\n")?;
    writeln!(f, "use crate::ast::*;\n")?;

    writeln!(f, "#[derive(EnumAsInner, Debug, Clone)]")?;
    writeln!(f, "pub enum Node {{")?;

    let mut variants = list_rs_file("src/ast")
        .into_iter()
        .map(|path| do_gen_node_enum(path, &mut f))
        .flatten()
        .flatten()
        .collect::<Vec<(String, String)>>();

    variants.sort();
    for item in variants {
        writeln!(f, "  {}({}),", item.0, item.1)?
    }

    writeln!(f, "}}")?;

    Ok(())
}

fn do_gen_node_enum(path: PathBuf, _f: &mut File) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut file = File::open(&path)?;

    let mut src = String::new();
    file.read_to_string(&mut src)?;

    let ast = syn::parse_file(&src)?;

    let mut variants: Vec<(String, String)> = Vec::new();

    for item in ast.items {
        match item {
            Item::Struct(item) => {
                let mut t = String::new();
                match item.vis {
                    Visibility::Crate(_) | Visibility::Inherited | Visibility::Restricted(_) => {
                        continue
                    }
                    _ => {}
                }

                t.push_str(&item.ident.to_string());

                if item.generics.lifetimes().count() > 0 {
                    t.push_str("<'a>");
                }

                variants.push((item.ident.to_string(), t))
            }

            Item::Enum(item) => {
                let mut t = String::new();
                match item.vis {
                    Visibility::Crate(_) | Visibility::Inherited | Visibility::Restricted(_) => {
                        continue
                    }
                    _ => {}
                }

                t.push_str(&item.ident.to_string());

                if item.generics.lifetimes().count() > 0 {
                    t.push_str("<'a>");
                }

                variants.push((item.ident.to_string(), t))
            }
            _ => {}
        }
    }

    Ok(variants)
}

fn list_rs_file(path: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(paths) = fs::read_dir(path) {
        for entry in paths.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        files.push(path);
                    }
                }
            }
        }
    }

    files
}