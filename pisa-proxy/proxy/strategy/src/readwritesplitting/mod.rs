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

pub mod dynamic_rw;
pub mod rule_match;
pub mod static_rw;
pub use dynamic_rw::*;
use endpoint::endpoint::Endpoint;
pub use static_rw::*;

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ReadWriteEndpoint {
    pub read: Vec<Endpoint>,
    pub readwrite: Vec<Endpoint>,
}

lazy_static! {
    pub static ref GENERIC_RULE_TOKEN: HashMap<&'static str, u8> =
        HashMap::from([("SELECT", 1), ("UPDATE", 2), ("INSERT", 3), ("DELETE", 4), ("SET", 5), ("START", 6)]);
}

