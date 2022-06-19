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
pub enum Create {
    CreateDatabase(Box<CreateDatabase>),
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

