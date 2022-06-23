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

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ReadWriteSplitting {
    #[serde(rename = "static")]
    pub undynamic: ReadWriteSplittingStatic,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegexRule {
    pub name: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub regex: Vec<String>,
    pub target: TargetRole,
    pub algorithm_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TargetRole {
    Read,
    ReadWrite,
}

impl Default for TargetRole {
    fn default() -> Self {
        TargetRole::ReadWrite
    }
}
