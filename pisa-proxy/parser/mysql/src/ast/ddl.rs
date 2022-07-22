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

use crate::ast::{base::*, FieldType, SelectStmt};

#[derive(Debug, Clone)]
pub enum Create {
    CreateDatabase(Box<CreateDatabase>),
    CreateViewOrTriggerOrSpOrEvent(Box<ViewOrTriggerOrSpOrEvent>),
    CreateLogFileGroup(Box<CreateLogFileGroup>),
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
    pub view_name: String,
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
    pub table_name: String,
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
    SizeOption(SizeOption),
    NodeGroupOption(NodeGroupOption),
    CommentOption(CommentOption),
    EngineOption(EngineOption),
    WaitOption(WaitOption),
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
