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

use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plugin {
    pub concurrency_control: Option<Vec<ConcurrencyControl>>,
    pub circuit_break: Option<Vec<CircuitBreak>>,
}

#[serde_with::serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConcurrencyControl {
    pub regex: String,
    pub max_concurrency: u32,
    #[serde_as(as = "serde_with::DurationSeconds<u64>")]
    pub duration: Duration,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CircuitBreak {
    pub regex: String,
}
