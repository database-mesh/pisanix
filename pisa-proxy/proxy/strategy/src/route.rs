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
use loadbalance::balance::{BalanceType, LoadBalance};

use crate::{config::{TargetRole, self}, readwritesplitting::{ReadWriteEndpoint, ReadWriteSplittingStaticBuilder, ReadWriteSplittingStatic}};


pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub enum RouteInput {
    Statement(String),
}

pub trait Route {
    type Error;
    
    fn dispatch<'a>(&'a mut self, input: RouteInput) -> Result<Option<&'a Endpoint>, Self::Error>;
}

pub trait RouteRuleMatch {
    fn is_match(&self, input: &RouteInput) -> bool;
}

pub trait RouteBalance {
    fn get(&mut self, input: &RouteInput) -> (&mut BalanceType, TargetRole);
}

pub enum RouteStrategy {
    Static(ReadWriteSplittingStatic),
    Simple(BalanceType),
    None,
}

impl RouteStrategy {
   pub fn new(config: config::ReadWriteSplitting, rw_endpoint: ReadWriteEndpoint) -> Self {
       if let Some(config) = config.model {
           return Self::Static(ReadWriteSplittingStaticBuilder::build(config, rw_endpoint))
       }


       Self::None
   }

   pub fn new_with_simple_route(balance: BalanceType) -> Self {
       Self::Simple(balance)
   }
}

impl Route for RouteStrategy {
    type Error = BoxError;

    fn dispatch<'a>(&'a mut self, input: RouteInput) -> Result<Option<&'a Endpoint>, Self::Error> {
        match self {
            Self::Static(ins) => ins.dispatch(input),

            Self::Simple(ins) => Ok(ins.next()),

            _ => Ok(None)
        }
    }
}