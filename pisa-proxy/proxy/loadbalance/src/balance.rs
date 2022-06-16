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

// use std::io::{Error, ErrorKind};
use endpoint::endpoint::Endpoint;
use serde::{Deserialize, Serialize};

use crate::{random_weighted::RandomWeighted, roundrobin_weighted::RoundRobinWeightd};
pub struct Balance;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AlgorithmName {
    Random,
    RoundRobin,
}

pub trait LoadBalance {
    fn next(&mut self) -> Option<&Endpoint>;
    fn add(&mut self, endpoint: Endpoint);
    fn item_exists(&self, endpoint: &Endpoint) -> bool;
    fn get_all(&mut self) -> &Vec<Endpoint>;
    fn remove_item(&mut self, endpoint: Endpoint);
    fn remove_all(&mut self);
}

impl Balance {
    pub fn build_balance(
        &mut self,
        algorithm_name: AlgorithmName,
    ) -> Box<dyn LoadBalance + Send + Sync> {
        match algorithm_name {
            AlgorithmName::Random => Box::new(RandomWeighted::default()),
            AlgorithmName::RoundRobin => Box::new(RoundRobinWeightd::default()),
        }
    }
}
