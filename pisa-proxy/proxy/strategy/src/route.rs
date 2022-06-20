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

use std::error::Error;

use endpoint::endpoint::Endpoint;
use loadbalance::balance::BalanceType;

use crate::config::TargetRole;



pub enum RouteInput {
    Statement(String),
}

pub trait Route {
    type Error;
    
    fn do_route<'a>(&'a mut self, input: RouteInput) -> Result<Option<&'a Endpoint>, Self::Error>;
}

pub trait RouteRuleMatch {
    fn is_match(&self, input: &RouteInput) -> bool;
}

pub trait RouteBalance {
    fn get(&mut self, input: &RouteInput) -> (&mut BalanceType, TargetRole);
}


#[cfg(test)]
mod test {
    #[test]
    fn test() {
        assert!(true)
    }
}