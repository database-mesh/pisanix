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

use std::io::{Error, ErrorKind};

use endpoint::endpoint::Endpoint;

use crate::{random_weighted::RandomWeighted, roundrobin_weighted::RoundRobinWeightd};
pub struct Balancer;

pub enum BalancerStrategy {
    Random,
    RoundRobin,
}

pub trait LoadBalancer {
    fn next(&mut self) -> Option<&Endpoint>;
    fn add(&mut self, endpoint: Endpoint);
    fn item_exists(&self, endpoint: &Endpoint) -> bool;
    fn get_all(&mut self) -> &Vec<Endpoint>;
    fn remove_item(&mut self, endpoint: Endpoint);
    fn remove_all(&mut self);
}

impl Balancer {
    pub fn build_balancer(
        &mut self,
        bs: String,
    ) -> Result<Box<dyn LoadBalancer + Send + Sync>, Error> {
        match bs.as_str() {
            "random" => Ok(Box::new(RandomWeighted::default())),
            "roundrobin" => Ok(Box::new(RoundRobinWeightd::default())),
            &_ => Err(Error::new(ErrorKind::Other, "balancer type not support")),
        }
    }
}
