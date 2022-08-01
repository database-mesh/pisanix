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
use crate::ast::User;

#[derive(Debug, Clone)]
pub struct CreateUser {
    pub span: Span,
    pub is_not_exists: bool,
    pub create_user_list: Vec<UserAndAuthOption>,
    pub default_role_clause: Option<DefaultRoleClause>,
    pub require_clause: Option<RequireClause>,
    pub connect_options: Option<ConnectOptions>,
    pub opt_account_lock_password_expire_options: Option<AccountLockPasswordExpireOptions>,
    pub opt_user_attribute: Option<UserAttribute>,
}

#[derive(Debug, Clone)]
pub struct UserAndAuthOption {
    pub span: Span,
    pub user: User,
    pub identification: Option<Identification>,
    pub opt_create_user_with_mfa: Option<CreateUserWithMFA>,
    pub identified_with_plugin: Option<Identification>,
    pub opt_initial_auth: Option<InitialAuth>,
}

#[derive(Debug, Clone)]
pub enum Identification {
    IdentifiedByPassword(IdentifiedByPassword),
    IdentifiedByRandomPassword(IdentifiedByRandomPassword),
    IdentifiedWithPlugin(IdentifiedWithPlugin),
    IdentifiedWithPluginAsAuth(IdentifiedWithPluginAsAuth),
    IdentifiedWithPluginByPassword(IdentifiedWithPluginByPassword),
    IdentifiedWithPluginByRandomPassword(IdentifiedWithPluginByRandomPassword),
}

#[derive(Debug, Clone)]
pub enum CreateUserWithMFA {
    CreateUserWith2FA(CreateUserWith2FA),
    CreateUserWith3FA(CreateUserWith3FA),
}

#[derive(Debug, Clone)]
pub struct CreateUserWith2FA {
    pub span: Span,
    pub auth_2fa_option: Identification,
}

#[derive(Debug, Clone)]
pub struct CreateUserWith3FA {
    pub span: Span,
    pub auth_2fa_option: Identification,
    pub auth_3fa_option: Identification,
}

#[derive(Debug, Clone)]
pub struct IdentifiedByPassword {
    pub span: Span,
    pub auth_string: String,
}

#[derive(Debug, Clone)]
pub struct IdentifiedByRandomPassword {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IdentifiedWithPlugin {
    pub span: Span,
    pub auth_plugin: String,
}

#[derive(Debug, Clone)]
pub struct IdentifiedWithPluginAsAuth {
    pub span: Span,
    pub auth_plugin: String,
    pub auth_string: String,
}

#[derive(Debug, Clone)]
pub struct IdentifiedWithPluginByPassword {
    pub span: Span,
    pub auth_plugin: String,
    pub auth_string: String,
}

#[derive(Debug, Clone)]
pub struct IdentifiedWithPluginByRandomPassword {
    pub span: Span,
    pub auth_plugin: String,
}

#[derive(Debug, Clone)]
pub struct InitialAuth {
    pub span: Span,
    pub identification: Identification,
}

#[derive(Debug, Clone)]
pub struct DefaultRoleClause {
    pub span: Span,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone)]
pub struct Role {
    pub span: Span,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct RequireClause {
    pub span: Span,
    pub requires: Option<Vec<RequireElement>>,
}

#[derive(Debug, Clone)]
pub struct RequireElement {
    pub span: Span,
    pub require: String,
}

#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub span: Span,
    pub options: Vec<ConnectOption>,
}

#[derive(Debug, Clone)]
pub struct ConnectOption {
    pub span: Span,
    pub num: String,
}

#[derive(Debug, Clone)]
pub struct AccountLockPasswordExpireOptions {
    pub span: Span,
    pub options: Vec<AccountLockPasswordExpireOption>,
}

#[derive(Debug, Clone)]
pub enum AccountLockPasswordExpireOption {
    AccountLock(AccountLock),
    PasswordExpire(PasswordExpire),
}

#[derive(Debug, Clone)]
pub struct AccountLock {
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct  PasswordExpire {
    pub span: Span,
    pub num: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserAttribute {
    pub span: Span,
    pub content: String,
}
