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

use crate::{
    config::{self, TargetRole},
    readwritesplitting::{
        ReadWriteEndpoint, ReadWriteSplittingStatic, ReadWriteSplittingStaticBuilder,
    },
};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// RouteInput may have more fields or variants added in the future,
/// As parameter of Route trait, Possible values are  `sql statement`, `sql ast`,etc.
#[derive(Debug)]
#[non_exhaustive]
pub enum RouteInput<'a> {
    Statement(&'a str),
    None,
}

/// Route trait, Used to decide on which endpoint to execute the sql statement.
pub trait Route {
    type Error;

    // The dispatch function, return a endpoint and role.
    fn dispatch<'a>(
        &'a mut self,
        input: &RouteInput,
    ) -> Result<(Option<&'a Endpoint>, TargetRole), Self::Error>;
}

/// Route rule, Currrently support `Regex` only.
pub trait RouteRuleMatch {
    fn is_match(&self, input: &RouteInput) -> bool;
}

/// RouteBalance trait, Used with RouteRuleMatch trait to get a balance type.
pub trait RouteBalance {
    fn get(&mut self, input: &RouteInput) -> (&mut BalanceType, TargetRole);
}

/// Supported routing strategies
pub enum RouteStrategy {
    Static(ReadWriteSplittingStatic),
    Simple(BalanceType),
    None,
}

impl RouteStrategy {
    pub fn new(config: config::ReadWriteSplitting, rw_endpoint: ReadWriteEndpoint) -> Self {
        if let Some(config) = config.model {
            return Self::Static(ReadWriteSplittingStaticBuilder::build(config, rw_endpoint));
        }

        // Just to return
        Self::None
    }

    pub fn new_with_simple_route(balance: BalanceType) -> Self {
        Self::Simple(balance)
    }
}

impl Route for RouteStrategy {
    type Error = BoxError;

    fn dispatch<'a>(
        &'a mut self,
        input: &RouteInput,
    ) -> Result<(Option<&'a Endpoint>, TargetRole), Self::Error> {
        match self {
            Self::Static(ins) => ins.dispatch(input),

            Self::Simple(ins) => Ok((ins.next(), TargetRole::ReadWrite)),

            _ => unreachable!(),
        }
    }
}
