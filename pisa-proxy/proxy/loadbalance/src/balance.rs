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

use endpoint::endpoint::Endpoint;
use serde::{Deserialize, Serialize};

use crate::{random_weighted::RandomWeighted, roundrobin_weighted::RoundRobinWeighted};
pub struct Balance;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AlgorithmName {
    Random,
    RoundRobin,
}

impl Default for AlgorithmName {
    fn default() -> Self {
        AlgorithmName::Random
    }
}

pub trait LoadBalance {
    fn next(&mut self) -> Option<Endpoint>;
    fn add(&mut self, endpoint: Endpoint);
    fn item_exists(&self, endpoint: &Endpoint) -> bool;
    fn get_all(&mut self) -> &Vec<Endpoint>;
    fn remove_item(&mut self, endpoint: Endpoint);
    fn remove_all(&mut self);
}

pub enum BalanceType {
    Random(RandomWeighted),
    RoundRobin(RoundRobinWeighted),
}

impl LoadBalance for BalanceType {
    fn next(&mut self) -> Option<Endpoint> {
        match self {
            BalanceType::Random(inner_random) => inner_random.next(),
            BalanceType::RoundRobin(inner_roundrobin) => inner_roundrobin.next(),
        }
    }

    fn add(&mut self, endpoint: Endpoint) {
        match self {
            BalanceType::Random(inner_random) => inner_random.add(endpoint),
            BalanceType::RoundRobin(inner_roundrobin) => inner_roundrobin.add(endpoint),
        }
    }

    fn item_exists(&self, endpoint: &Endpoint) -> bool {
        match self {
            BalanceType::Random(inner_random) => inner_random.item_exists(endpoint),
            BalanceType::RoundRobin(inner_roundrobin) => inner_roundrobin.item_exists(endpoint),
        }
    }

    fn get_all(&mut self) -> &Vec<Endpoint> {
        match self {
            BalanceType::Random(inner_random) => inner_random.get_all(),
            BalanceType::RoundRobin(inner_roundrobin) => inner_roundrobin.get_all(),
        }
    }
    fn remove_item(&mut self, endpoint: Endpoint) {
        match self {
            BalanceType::Random(inner_random) => inner_random.remove_item(endpoint),
            BalanceType::RoundRobin(inner_roundrobin) => inner_roundrobin.remove_item(endpoint),
        }
    }

    fn remove_all(&mut self) {
        match self {
            BalanceType::Random(inner_random) => inner_random.remove_all(),
            BalanceType::RoundRobin(inner_roundrobin) => inner_roundrobin.remove_all(),
        }
    }
}

impl Balance {
    pub fn build_balance(&mut self, algorithm_name: AlgorithmName) -> BalanceType {
        match algorithm_name {
            AlgorithmName::Random => BalanceType::Random(RandomWeighted::default()),
            AlgorithmName::RoundRobin => BalanceType::RoundRobin(RoundRobinWeighted::default()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn load_balancer() {
        // let mut balance = Balance.build_balance(AlgorithmName::Random);
        let mut balance = Balance.build_balance(AlgorithmName::RoundRobin);
        let ep1 = Endpoint {
            weight: 2,
            name: String::from("dasheng001"),
            db: String::from("db001"),
            user: String::from("root"),
            password: String::from("root"),
            addr: String::from("127.0.0.1:3306"),
        };
        let ep2 = Endpoint {
            weight: 1,
            name: String::from("dasheng002"),
            db: String::from("db002"),
            user: String::from("root"),
            password: String::from("root"),
            addr: String::from("127.0.0.1:3307"),
        };
        balance.add(ep1.clone());
        balance.add(ep2.clone());
        for _i in 0..10 {
            println!("{:#?}", balance.next());
        }
        
    }
}