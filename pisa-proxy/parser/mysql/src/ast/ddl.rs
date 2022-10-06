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

use crate::ast::{base::*, dml::TableIdent, CreateUser, FieldType, SelectStmt, dml};

#[derive(Debug, Clone)]
pub enum Create {
    CreateDatabase(Box<CreateDatabase>),
    CreateViewOrTriggerOrSpOrEvent(Box<ViewOrTriggerOrSpOrEvent>),
    CreateLogFileGroup(Box<CreateLogFileGroup>),
    CreateUser(Box<CreateUser>),
    CreateTablespace(Box<CreateTablespace>),
    CreateUndoTablespace(Box<CreateUndoTablespace>),
    CreateServer(Box<CreateServer>),
}

#[derive(Debug, Clone)]
pub struct CreateDatabase {
    pub is_not_exists: bool,
    pub database_name: String,
    pub opt_create_database_options: Vec<CreateDatabaseOption>,
}

#[derive(Debug, Clone)]
pub enum CreateDatabaseOption {
    DefaultCollation(DefaultCollation),
    DefaultCharset(DefaultCharset),
    DefaultEncryption(DefaultEncryption),
}

#[derive(Debug, Clone)]
pub struct DefaultCollation {
    pub is_default: bool,
    pub is_equal: bool,
    pub collation_name: String,
}

#[derive(Debug, Clone)]
pub struct DefaultCharset {
    pub is_default: bool,
    pub is_equal: bool,
    pub charset_name: String,
}

#[derive(Debug, Clone)]
pub struct DefaultEncryption {
    pub is_default: bool,
    pub is_equal: bool,
    pub encryption: String,
}

#[derive(Debug, Clone)]
pub struct Definer {
    pub user: User,
}

#[derive(Debug, Clone)]
pub enum ViewSuid {
    ViewSuidDefault,
    ViewSuidDefiner,
    ViewSuidInvoker,
}

#[derive(Debug, Clone)]
pub enum ViewCheckOption {
    ViewCheckNone,
    ViewCheckCascade,
    ViewCheckLocal,
}

#[derive(Debug, Clone)]
pub struct ViewQueryBlock {
    pub select_stmt: SelectStmt,
    pub view_check_option: ViewCheckOption,
}

#[derive(Debug, Clone)]
pub struct ViewTail {
    pub span: Span,
    pub view_suid: ViewSuid,
    pub view_name: TableIdent,
    pub columns: Vec<Value>,
    pub view_query_block: ViewQueryBlock,
}

#[derive(Debug, Clone)]
pub enum DefinerTail {
    ViewTail(ViewTail),
    TriggerTail(TriggerTail),
    SpTail(SpTail),
}

#[derive(Debug, Clone)]
pub struct ViewOrTriggerOrSpOrEvent {
    pub definer: Definer,
    pub definer_tail: DefinerTail,
}

#[derive(Debug, Clone)]
pub enum TrgActionTime {
    Before,
    After,
}

#[derive(Debug, Clone)]
pub enum TrgEvent {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone)]
pub enum TrgActionOrder {
    None,
    Follows,
    Precedes,
}

#[derive(Debug, Clone)]
pub struct TriggerFollowsPrecedesClause {
    pub ordering_clause: TrgActionOrder,
    pub anchor_trigger_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TriggerTail {
    pub span: Span,
    pub sp_name: String,
    pub trg_action_time: TrgActionTime,
    pub trg_event: TrgEvent,
    pub table_name: TableIdent,
    pub trigger_follows_precedes_clause: TriggerFollowsPrecedesClause,
    pub sp_proc_stmt: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SpOptInout {
    In,
    Out,
    Inout,
}

#[derive(Debug, Clone)]
pub struct SpPdparam {
    pub sp_opt_inout: SpOptInout,
    pub ident: String,
    pub sp_type: FieldType,
    pub opt_collate: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SpTail {
    pub span: Span,
    pub sp_name: String,
    pub sp_pdparam_list: Vec<SpPdparam>,
    pub sp_c_chistics: Vec<SpCChistic>,
    pub sp_proc_stmt: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SpSuid {
    SpIsSuid,
    SpIsNotSuid,
}

#[derive(Debug, Clone)]
pub enum SpChistic {
    Comment(String),
    LanguageSql,
    NoSql,
    ContainsSql,
    ReadsSqlData,
    ModifiesSqlData,
    SpSuid(SpSuid),
}

#[derive(Debug, Clone)]
pub enum SpCChistic {
    SpChistic(SpChistic),
    Deterministic,
    NotDeterministic,
}

#[derive(Debug, Clone)]
pub enum CreateIndexStmt {
    CommonIndex(CreateCommonIndexStmt),
    FullTextIndex(CreateFullTextIndexStmt),
    SpatialIndex(CreateSpatialIndexStmt),
}

#[derive(Debug, Clone)]
pub struct CreateCommonIndexStmt {
    pub span: Span,
    pub opt_unique: bool,
    pub index_name: String,
    pub opt_index_type_clause: Option<IndexTypeClause>,
    pub table_name: TableIdent,
    pub key_list_with_expression: Vec<KeyPartWithExpression>,
    pub opt_index_options: Option<Vec<IndexOption>>,
    pub opt_index_lock_and_algorithm: Option<IndexLockAndAlgorithm>,
}

#[derive(Debug, Clone)]
pub struct CreateFullTextIndexStmt {
    pub span: Span,
    pub index_name: String,
    pub table_name: TableIdent,
    pub key_list_with_expression: Vec<KeyPartWithExpression>,
    pub opt_fulltext_index_options: Option<Vec<FullTextIndexOption>>,
    pub opt_index_lock_and_algorithm: Option<IndexLockAndAlgorithm>,
}

#[derive(Debug, Clone)]
pub struct CreateSpatialIndexStmt {
    pub span: Span,
    pub index_name: String,
    pub table_name: TableIdent,
    pub key_list_with_expression: Vec<KeyPartWithExpression>,
    pub opt_spatial_index_options: Option<Vec<SpatialIndexOption>>,
    pub opt_index_lock_and_algorithm: Option<IndexLockAndAlgorithm>,
}

#[derive(Debug, Clone)]
pub struct IndexTypeClause {
    pub span: Span,
    pub index_type: IndexType,
}

#[derive(Debug, Clone)]
pub enum IndexType {
    Btree,
    Rtree,
    Hash,
}

#[derive(Debug, Clone)]
pub enum IndexOption {
    CommonIndexOption(CommonIndexOption),
    IndexTypeClause(IndexTypeClause),
}

#[derive(Debug, Clone)]
pub enum FullTextIndexOption {
    CommonIndexOption(CommonIndexOption),
    WithParserIdent(String),
}

#[derive(Debug, Clone)]
pub enum SpatialIndexOption {
    CommonIndexOption(CommonIndexOption),
}

#[derive(Debug, Clone)]
pub enum CommonIndexOption {
    KeyBlockSizeOption(KeyBlockSizeOption),
    CommentOption(IndexCommentOption),
    Visibility(Visibility),
    AttributeOption(AttributeOption),
}

#[derive(Debug, Clone)]
pub struct KeyBlockSizeOption {
    pub span: Span,
    pub is_equal: bool,
    pub ulong_num: String,
}

#[derive(Debug, Clone)]
pub struct IndexCommentOption {
    pub span: Span,
    pub comment: String,
}

#[derive(Debug, Clone)]
pub enum Visibility {
    Visible,
    Invisible,
}

#[derive(Debug, Clone)]
pub struct AttributeOption {
    pub span: Span,
    pub is_equal: bool,
    pub attribute: String,
}

#[derive(Debug, Clone)]
pub struct IndexLockAndAlgorithm {
    pub alter_lock_option: Option<AlterLockOption>,
    pub alter_algorithm_option: Option<AlterAlgorithmOption>,
}

#[derive(Debug, Clone)]
pub struct AlterLockOption {
    pub span: Span,
    pub is_equal: bool,
    pub alter_lock_option_value: String,
}

#[derive(Debug, Clone)]
pub struct AlterAlgorithmOption {
    pub span: Span,
    pub is_equal: bool,
    pub alter_algorithm_option_value: String,
}

#[derive(Debug, Clone)]
pub enum KeyPartWithExpression {
    KeyPart(KeyPart),
    OrderingDirection(OrderExpr),
}

#[derive(Debug, Clone)]
pub struct KeyPart {
    pub span: Span,
    pub ident: String,
    pub length: Option<u32>,
    pub direction: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateLogFileGroup {
    pub span: Span,
    pub logfile_group: String,
    pub undo_file: UndoFile,
    pub opt_logfile_group_options: Option<Vec<LogFileGroupOption>>,
}

#[derive(Debug, Clone)]
pub struct UndoFile {
    pub span: Span,
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub enum LogFileGroupOption {
    InitialSize(SizeOption),
    UndoBufferSize(SizeOption),
    RedoBufferSize(SizeOption),
    NodeGroup(NodeGroupOption),
    Comment(CommentOption),
    Engine(EngineOption),
    Wait(WaitOption),
}

#[derive(Debug, Clone)]
pub struct SizeOption {
    pub span: Span,
    pub is_equal: bool,
    pub size: String,
}

#[derive(Debug, Clone)]
pub struct NodeGroupOption {
    pub span: Span,
    pub is_equal: bool,
    pub nodegroup_id: String,
}

#[derive(Debug, Clone)]
pub struct CommentOption {
    pub span: Span,
    pub is_equal: bool,
    pub comment: String,
}

#[derive(Debug, Clone)]
pub struct EngineOption {
    pub span: Span,
    pub opt_storage: bool,
    pub is_equal: bool,
    pub engine_name: String,
}

#[derive(Debug, Clone)]
pub enum WaitOption {
    Wait,
    NoWait,
}

#[derive(Debug, Clone)]
pub struct CreateTablespace {
    pub span: Span,
    pub tablespace_name: String,
    pub opt_ts_datafile: Option<AddTsDataFile>,
    pub opt_logfile_group: Option<LogFileGroup>,
    pub opt_tablespace_options: Option<Vec<TablespaceOption>>,
}

#[derive(Debug, Clone)]
pub struct AddTsDataFile {
    pub span: Span,
    pub ts_datafile: TsDataFile,
}

#[derive(Debug, Clone)]
pub struct TsDataFile {
    pub span: Span,
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub struct LogFileGroup {
    pub span: Span,
    pub logfile_group: String,
}

#[derive(Debug, Clone)]
pub enum TablespaceOption {
    InitialSize(SizeOption),
    AutoextendSize(SizeOption),
    MaxSize(SizeOption),
    ExtentSize(SizeOption),
    NodeGroup(NodeGroupOption),
    Engine(EngineOption),
    Wait(WaitOption),
    Comment(CommentOption),
    FileBlockSize(SizeOption),
    Encryption(EncryptionOption),
    EngineAttribute(EngineAttributeOption),
}

#[derive(Debug, Clone)]
pub struct EncryptionOption {
    pub span: Span,
    pub is_equal: bool,
    pub encryption: String,
}

#[derive(Debug, Clone)]
pub struct EngineAttributeOption {
    pub span: Span,
    pub is_equal: bool,
    pub attribute: String,
}

#[derive(Debug, Clone)]
pub enum AlterTablespaceOption {
    InitialSize(SizeOption),
    AutoextendSize(SizeOption),
    MaxSize(SizeOption),
    Engine(EngineOption),
    Wait(WaitOption),
    Encryption(EncryptionOption),
    EngineAttribute(EngineAttributeOption),
}

#[derive(Debug, Clone)]
pub enum AlterLogFileGroupOption {
    InitialSize(SizeOption),
    Engine(EngineOption),
    Wait(WaitOption),
}

#[derive(Debug, Clone)]
pub enum UndoTablespaceState {
    Active,
    Inactive,
}

#[derive(Debug, Clone)]
pub struct CreateUndoTablespace {
    pub span: Span,
    pub tablespace_name: String,
    pub ts_datafile: TsDataFile,
    pub opt_undo_tablespace_options: Option<Vec<UndoTablespaceOption>>,
}

#[derive(Debug, Clone)]
pub enum UndoTablespaceOption {
    Engine(EngineOption),
}

#[derive(Debug, Clone)]
pub struct CreateServer {
    pub span: Span,
    pub server_name: String,
    pub wrapper_name: String,
    pub server_options_list: Vec<ServerOption>,
}

#[derive(Debug, Clone)]
pub enum ServerOption {
    User(String),
    Host(String),
    Database(String),
    Owner(String),
    Password(String),
    Socket(String),
    Port(String),
}

#[derive(Debug, Clone)]
pub struct StringOption {
    pub span: Span,
    pub is_equal: bool,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct TernaryOption {
    pub span: Span,
    pub is_equal: bool,
    pub option: Ternary,
}

#[derive(Debug, Clone)]
pub struct CreateTableStmt {
    pub span: Span,
    pub is_temporary: bool,
    pub is_not_exists: bool,
    pub table_ident: TableIdent,
    pub table_element_list: Option<Vec<TableElement>>,
    pub opt_create_table_options_etc: Option<CreateTableOptions>,
    pub like_table_ident: Option<TableIdent>,
}

#[derive(Debug, Clone)]
pub struct CreateTableOptions {
    pub opt_create_table_options: Option<Vec<CreateTableOption>>,
    pub opt_partitioning: Option<Partition>,
    pub on_duplicate: OnDuplicate,
    pub opt_query_expression: Option<SelectStmt>,
}

#[derive(Debug, Clone)]
pub struct Partition {
    pub span: Span,
    pub part_type_def: PartTypeDef,
    pub opt_num_parts: Option<String>,
    pub opt_sub_part: Option<SubPartition>,
    pub opt_part_defs: Option<Vec<PartDefinition>>,
}

#[derive(Debug, Clone)]
pub enum TableElement {
    ColumnDef(ColumnDef),
    TableConstraintDef(TableConstraintDef),
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub span: Span,
    pub column_name: String,
    pub field_def: FieldDef,
    pub opt_references: Option<References>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub span: Span,
    pub data_type: FieldType,
    pub opt_collate: Option<String>,
    pub is_generated_always: bool,
    pub expr: Option<Expr>,
    pub opt_stored_attribute: Option<StoredAttribute>,
    pub opt_column_attribute_list: Option<Vec<ColumnAttribute>>,
}

#[derive(Debug, Clone)]
pub enum StoredAttribute {
    Virtual,
    Stored,
}

#[derive(Debug, Clone)]
pub struct References {
    pub span: Span,
    pub table_ident: TableIdent,
    pub opt_ref_list: Option<Vec<String>>,
    pub opt_match_clause: Option<MatchClause>,
    pub opt_on_update_delete: Option<OnUpdateDelete>,
}

#[derive(Debug, Clone)]
pub enum MatchClause {
    Full,
    Partial,
    Simple,
}

#[derive(Debug, Clone)]
pub struct OnUpdateDelete {
    pub span: Span,
    pub update_option: Option<ReferenceOption>,
    pub delete_option: Option<ReferenceOption>,
}

#[derive(Debug, Clone)]
pub enum ReferenceOption {
    Restrict,
    Cascade,
    SetNull,
    NoAction,
    SetDefault,
}

#[derive(Debug, Clone)]
pub enum TableConstraintDef {
    InlineIndex(InlineIndexDefinition),
    ForeignKey(ForeignKeyDefinition),
    Check(CheckDefinition),
}

#[derive(Debug, Clone)]
pub struct InlineIndexDefinition {
    pub span: Span,
    pub key_type: KeyType,
    pub index_name: Option<String>,
    pub index_type: Option<IndexType>,
    pub key_list_with_expression: Vec<KeyPartWithExpression>,
    pub opt_index_options: Option<Vec<IndexOption>>,
    pub opt_fulltext_index_options: Option<Vec<FullTextIndexOption>>,
    pub opt_spatial_index_options: Option<Vec<SpatialIndexOption>>,
}

#[derive(Debug, Clone)]
pub struct ForeignKeyDefinition {
    pub span: Span,
    pub constraint_name: Option<String>,
    pub key_name: Option<String>,
    pub key_list: Vec<KeyPart>,
    pub references: References,
}

#[derive(Debug, Clone)]
pub struct CheckDefinition {
    pub span: Span,
    pub name: Option<String>,
    pub check_expr: Expr,
    pub is_enforced: bool,
}

#[derive(Debug, Clone)]
pub enum KeyType {
    Primary,
    Unique,
    Multiple,
    FullText,
    Spatial,
    Foreign,
}

#[derive(Debug, Clone)]
pub enum ColumnAttribute {
    Null,
    NotNull,
    NotSecondary,
    DefaultLiteral(Value),
    DefaultExpr(Expr),
    OnUpdate(String),
    AutoInc,
    SerialDefaultValue,
    PrimaryKey,
    Unique,
    Comment(String),
    Collate(String),
    ColumnFormat(ColumnFormat),
    Storage(StorageMedia),
    Srid(String),
    Check(CheckDefinition),
    Enforcement(bool),
    Engine(AttributeOption),
    SecondaryEngine(AttributeOption),
    Visibility,
}

#[derive(Debug, Clone)]
pub enum ColumnFormat {
    Default,
    Fixed,
    Dynamic,
}

#[derive(Debug, Clone)]
pub enum StorageMedia {
    Default,
    Disk,
    Memory,
}

#[derive(Debug, Clone)]
pub enum CreateTableOption {
    Engine {
        span: Span,
        is_equal: bool,
        engine_name: String,
    },
    SecondaryEngine {
        span: Span,
        is_equal: bool,
        engine_name: Option<String>,
    },
    MaxRows {
        span: Span,
        is_equal: bool,
        value: String,
    },
    MinRows {
        span: Span,
        is_equal: bool,
        value: String,
    },
    AvgRowLength {
        span: Span,
        is_equal: bool,
        value: String,
    },
    Password {
        span: Span,
        is_equal: bool,
        content: String,
    },
    Comment {
        span: Span,
        is_equal: bool,
        content: String,
    },
    Compression {
        span: Span,
        is_equal: bool,
        value: String,
    },
    Encryption {
        span: Span,
        is_equal: bool,
        value: String,
    },
    AutoInc {
        span: Span,
        is_equal: bool,
        value: String,
    },
    PackKeys {
        span: Span,
        is_equal: bool,
        option: Ternary,
    },
    StatsAutoRecalc {
        span: Span,
        is_equal: bool,
        option: Ternary,
    },
    StatsPersistent {
        span: Span,
        is_equal: bool,
        option: Ternary,
    },
    StatsSamplePages {
        span: Span,
        is_equal: bool,
        is_default: bool,
        value: Option<String>,
    },
    Checksum {
        span: Span,
        is_equal: bool,
        value: String,
    },
    TableChecksum {
        span: Span,
        is_equal: bool,
        value: String,
    },
    DelayKeyWrite {
        span: Span,
        is_equal: bool,
        value: String,
    },
    RowFormat {
        span: Span,
        is_equal: bool,
        row_types: RowTypes,
    },
    Union {
        span: Span,
        is_equal: bool,
        opt_table_list: Option<Vec<TableIdent>>,
    },
    DefaultCharset(DefaultCharset),
    DefaultCollation(DefaultCollation),
    InsertMethod {
        span: Span,
        is_equal: bool,
        merge_insert_types: MergeInsertTypes,
    },
    DataDirectory {
        span: Span,
        is_equal: bool,
        value: String,
    },
    IndexDirectory {
        span: Span,
        is_equal: bool,
        value: String,
    },
    Tablespace {
        span: Span,
        is_equal: bool,
        value: String,
    },
    StorageDisk,
    StorageMemory,
    Connection {
        span: Span,
        is_equal: bool,
        value: String,
    },
    KeyBlockSize {
        span: Span,
        is_equal: bool,
        value: String,
    },
    StartTransaction,
    EngineAttribute {
        span: Span,
        is_equal: bool,
        json_attribute: String,
    },
    SecondaryEngineAttribute {
        span: Span,
        is_equal: bool,
        json_attribute: String,
    },
    OptionAutoextendSize(SizeOption),
}

#[derive(Debug, Clone)]
pub enum Ternary {
    Default,
    On,
    Off,
}

#[derive(Debug, Clone)]
pub enum RowTypes {
    Default,
    Fixed,
    Dynamic,
    Compressed,
    Redundant,
    Compact,
}

#[derive(Debug, Clone)]
pub enum MergeInsertTypes {
    Disabled,
    First,
    Last,
}

#[derive(Debug, Clone)]
pub enum OnDuplicate {
    Error,
    Replace,
    Ignore,
}

#[derive(Debug, Clone)]
pub enum PartTypeDef {
    Key {
        span: Span,
        is_linear: bool,
        opt_key_algo: Option<KeyAlgorithm>,
        opt_name_list: Option<Vec<String>>,
    },

    Hash {
        span: Span,
        is_linear: bool,
        bit_expr: Expr,
    },

    Range {
        span: Span,
        bit_expr: Expr,
    },

    RangeColumns {
        span: Span,
        name_list: Vec<String>,
    },

    List {
        span: Span,
        bit_expr: Expr,
    },

    ListColumns {
        span: Span,
        name_list: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct KeyAlgorithm {
    pub span: Span,
    pub eq: Op,
    pub num: String,
}

#[derive(Debug, Clone)]
pub enum SubPartition {
    Hash {
        span: Span,
        is_linear: bool,
        bit_expr: Expr,
        opt_num_subparts: String,
    },
    Key {
        span: Span,
        is_linear: bool,
        opt_key_algo: Option<KeyAlgorithm>,
        name_list: Vec<String>,
        opt_num_subparts: String,
    },
}

#[derive(Debug, Clone)]
pub struct PartDefinition {
    pub span: Span,
    pub partition_name: String,
    pub partition_type: PartitionType,
    pub partition_value: Option<Vec<Vec<PartValueItem>>>,
    pub opt_part_options: Option<Vec<PartitionOption>>,
    pub opt_sub_partition: Option<Vec<SubPartDefinition>>,
}

#[derive(Debug, Clone)]
pub enum PartitionType {
    Hash,
    Range,
    List,
}

#[derive(Debug, Clone)]
pub enum PartValueItem {
    MaxValue,
    BitExpr(Expr),
}

#[derive(Debug, Clone)]
pub enum PartitionOption {
    Tablespace {
        span: Span,
        is_equal: bool,
        tablespace_name: String,
    },
    Engine {
        span: Span,
        is_storage: bool,
        is_equal: bool,
        engine_name: String,
    },
    NodeGroup {
        span: Span,
        is_equal: bool,
        group_num: String,
    },
    MaxRows {
        span: Span,
        is_equal: bool,
        rows_num: String,
    },
    MinRows {
        span: Span,
        is_equal: bool,
        rows_num: String,
    },
    DataDirectory {
        span: Span,
        is_equal: bool,
        data_dir: String,
    },
    IndexDirectory {
        span: Span,
        is_equal: bool,
        index_dir: String,
    },
    Comment {
        span: Span,
        is_equal: bool,
        comment: String,
    }
}

#[derive(Debug, Clone)]
pub struct SubPartDefinition {
    pub span: Span,
    pub name: String,
    pub opt_part_options: Option<Vec<PartitionOption>>,
}
