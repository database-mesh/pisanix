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

use chrono::prelude::*;
use endpoint::endpoint::Endpoint;
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::balance::LoadBalance;

pub struct RandomWeighted {
    pub items: Vec<Endpoint>,
    pub n: i64,
    pub sum_of_weights: i64,
    pub r: StdRng,
}

impl Default for RandomWeighted {
    fn default() -> RandomWeighted {
        RandomWeighted {
            items: vec![],
            n: 0,
            sum_of_weights: 0,
            r: StdRng::seed_from_u64(Utc::now().timestamp_subsec_nanos().into()),
        }
    }
}

impl LoadBalance for RandomWeighted {
    // next: get next endpoint
    fn next(&mut self) -> Option<&Endpoint> {
        if self.n == 0 {
            return None;
        }

        if self.sum_of_weights <= 0 {
            return None;
        }
        let mut random_weight = self.r.gen_range(0..self.sum_of_weights) + 1;
        for i in &self.items {
            random_weight -= i.weight;
            if random_weight <= 0 {
                return Some(i);
            }
        }
        let item = &self.items[self.items.len() - 1];
        Some(&item)
    }

    // add: add endpoint
    fn add(&mut self, endpoint: Endpoint) {
        if !self.item_exists(&endpoint) {
            self.sum_of_weights += endpoint.weight;
            self.n += 1;
            self.items.push(endpoint);
        }
    }

    // item_exists: endpoint exists
    fn item_exists(&self, endpoint: &Endpoint) -> bool {
        match self.items.iter().find(|&x| x.name == endpoint.name) {
            None => false,
            _ => true,
        }
    }

    // get_all: get all endpoint
    fn get_all(&mut self) -> &Vec<Endpoint> {
        &self.items
    }

    // remove_item: remove item
    fn remove_item(&mut self, endpoint: Endpoint) {
        let index = self.items.iter().position(|x| *x.name == *endpoint.name).unwrap();
        self.items.remove(index);
    }

    // remove_all: remove all item
    fn remove_all(&mut self) {
        self.items = vec![];
        self.r = StdRng::seed_from_u64(Utc::now().timestamp_subsec_nanos().into());
    }
}
