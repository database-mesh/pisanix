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

use loadbalance::balance::AlgorithmName;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ReadWriteSplitting {
    #[serde(rename = "static")]
    pub statics: Option<ReadWriteSplittingStatic>,
    pub dynamic: Option<ReadWriteSplittingDynamic>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ReadWriteSplittingStatic {
    pub default_target: TargetRole,
    #[serde(rename = "rule")]
    pub rules: Vec<ReadWriteSplittingRule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ReadWriteSplittingRule {
    Regex(RegexRule),
    Generic(GenericRule),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadWriteSplittingDynamic {
    pub default_target: TargetRole,
    #[serde(rename = "rule")]
    pub rules: Vec<ReadWriteSplittingRule>,
    pub discovery: Discovery,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Discovery {
    Mha(MasterHighAvailability),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct MasterHighAvailability {
    pub user: String,
    pub password: String,
    #[serde(default = "default_monitor_period")]
    pub monitor_period: u64,
    #[serde(default = "default_connect_period")]
    pub connect_period: u64,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,
    #[serde(default = "default_connect_failure_threshold")]
    pub connect_failure_threshold: u64,
    #[serde(default = "default_ping_period")]
    pub ping_period: u64,
    #[serde(default = "default_ping_timeout")]
    pub ping_timeout: u64,
    #[serde(default = "default_ping_failure_threshold")]
    pub ping_failure_threshold: u64,
    #[serde(default = "default_replication_lag_period")]
    pub replication_lag_period: u64,
    #[serde(default = "default_replication_lag_timeout")]
    pub replication_lag_timeout: u64,
    #[serde(default = "default_replication_lag_failure_threshold")]
    pub replication_lag_failure_threshold: u64,
    #[serde(default = "default_max_replication_lag")]
    pub max_replication_lag: u64,
    #[serde(default = "default_read_only_period")]
    pub read_only_period: u64,
    #[serde(default = "default_read_only_timeout")]
    pub read_only_timeout: u64,
    #[serde(default = "default_read_only_failure_threshold")]
    pub read_only_failure_threshold: u64,
    #[serde(default = "default_read_only_enabled")]
    pub read_only_enabled: bool,
    #[serde(default = "default_replication_lag_enabled")]
    pub replication_lag_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegexRule {
    pub name: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub regex: Vec<String>,
    pub target: TargetRole,
    pub algorithm_name: AlgorithmName,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenericRule {
    pub name: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub algorithm_name: AlgorithmName,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TargetRole {
    Read,
    ReadWrite,
}

impl Default for TargetRole {
    fn default() -> Self {
        Self::ReadWrite
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sharding {
    pub table_name: String,
    pub actual_datanodes: Vec<String>,
    pub binding_tables: Option<Vec<String>>,
    pub broadcast_tables: Option<Vec<String>>,
    pub database_strategy: Option<StrategyType>,
    pub table_strategy: Option<StrategyType>,
    pub database_table_strategy: Option<StrategyType>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum StrategyType {
    DatabaseStrategyConfig(DatabaseStrategyConfig),
    DatabaseStrategyInline(StrategyInline),
    TableStrategyConfig(TableStrategyConfig),
    TableStrategyInline(StrategyInline),
    DatabaseTableStrategyConfig(DatabaseTableStrategyConfig),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ShardingAlgorithmName {
    Mod,
    CRC32Mod,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseStrategyConfig {
    pub database_sharding_algorithm_name: ShardingAlgorithmName,
    pub database_sharding_column: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrategyInline {
    pub algorithm_expression: String,
    pub allow_range_query_with_inline_sharding: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableStrategyConfig {
    pub datanode_name: String,
    pub table_sharding_algorithm_name: ShardingAlgorithmName,
    pub table_sharding_column: String,
    pub sharding_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseTableStrategyConfig {
    pub database_sharding_algorithm_name: ShardingAlgorithmName,
    pub table_sharding_algorithm_name: ShardingAlgorithmName,
    pub database_sharding_column: String,
    pub table_sharding_column: String,
    pub shading_count: u32,
}

fn default_monitor_period() -> u64 {
    1000
}

fn default_connect_period() -> u64 {
    1000
}

fn default_connect_timeout() -> u64 {
    6000
}

fn default_connect_failure_threshold() -> u64 {
    1
}

fn default_ping_period() -> u64 {
    1000
}

fn default_ping_timeout() -> u64 {
    6000
}

fn default_ping_failure_threshold() -> u64 {
    1
}

fn default_replication_lag_period() -> u64 {
    1000
}

fn default_replication_lag_timeout() -> u64 {
    6000
}

fn default_replication_lag_failure_threshold() -> u64 {
    1
}

fn default_max_replication_lag() -> u64 {
    10000
}

fn default_read_only_period() -> u64 {
    1000
}

fn default_read_only_timeout() -> u64 {
    6000
}

fn default_read_only_failure_threshold() -> u64 {
    1
}

fn default_read_only_enabled() -> bool {
    true
}

fn default_replication_lag_enabled() -> bool {
    true
}
