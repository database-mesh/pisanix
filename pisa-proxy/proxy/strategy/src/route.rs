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
use indexmap::{IndexMap, IndexSet};
use loadbalance::balance::{BalanceType, LoadBalance};
use thiserror::Error;

use crate::{
    config::{self, TargetRole},
    readwritesplitting::{
        ReadWriteEndpoint, ReadWriteSplittingDynamic, ReadWriteSplittingDynamicBuilder,
        ReadWriteSplittingStatic, ReadWriteSplittingStaticBuilder,
    },
};

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StragegyError {
    #[error("build node name not found {0:?}")]
    EndpointNotFound(String),

    #[error("build node group name not found {0:?}")]
    NodeGroupNotFound(String),
}

/// RouteInput may have more fields or variants added in the future,
/// As parameter of Route trait, Possible values are  `sql statement`, `sql ast`,etc.
#[derive(Debug)]
#[non_exhaustive]
pub enum RouteInput<'a> {
    Statement(&'a str),
    Transaction(&'a str),
    Sharding(Endpoint),
    ShardingStatement(&'a str, String),
    ShardingTransaction(&'a str, String),
    None,
}

#[derive(Debug)]
pub enum ShardingRouteInput<'a> {
    ShardingReadWriteSplitting(ReadWriteSplittingRouteInput<'a>, String),
    Sharding(Endpoint)
}

#[derive(Debug)]
pub enum ReadWriteSplittingRouteInput<'a> {
    Statement(&'a str),
    Transaction(&'a str),
}

/// Route trait, Used to decide on which endpoint to execute the sql statement.
pub trait Route {
    type Error;

    // The dispatch function, return a endpoint and role.
    fn dispatch(
        &mut self,
        input: &RouteInput,
    ) -> Result<(Option<Endpoint>, TargetRole), Self::Error>;
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
    ReadWriteSplitting(ReadWriteSplittingRouteStrategy),
    ShardingReadWriteSplitting(ReadWriteSplittingRouteStrategy),
    Sharding,
    Simple(BalanceType),
    None,
}

pub enum ReadWriteSplittingRouteStrategy {
    Static(ReadWriteSplittingStatic),
    Dynamic(ReadWriteSplittingDynamic),
    None,
}

pub enum ShardingRouteStrategy {
    ShardingReadWriteSplitting(ReadWriteSplittingRouteStrategy),
    Sharding,
}

impl ReadWriteSplittingRouteStrategy {
    pub fn new(config: config::ReadWriteSplitting, node_group_config: Option<config::NodeGroup>, endpoint_group: IndexMap<String, ReadWriteEndpoint>, rw_endpoint: ReadWriteEndpoint) -> Self {
        if let Some(config) = config.statics {
            return Self::Static(ReadWriteSplittingStaticBuilder::build(config, endpoint_group, rw_endpoint));
        }

        if let Some(config) = config.dynamic {
            return Self::Dynamic(ReadWriteSplittingDynamicBuilder::build(config, node_group_config, endpoint_group, rw_endpoint));
        }

        // Just to return
        Self::None
    }
}


impl RouteStrategy {
    pub fn new(
        config: config::ReadWriteSplitting,
        node_group_config: &Option<config::NodeGroup>,
        rw_endpoint: ReadWriteEndpoint,
        has_sharding: bool,
    ) -> Result<Self, StragegyError> {
        let endpoint_group = Self::get_endpoint_group(node_group_config, &rw_endpoint)?;

        let rw_strategy = ReadWriteSplittingRouteStrategy::new(config, node_group_config.clone(), endpoint_group, rw_endpoint);
        if has_sharding {
            Ok(Self::ReadWriteSplitting(rw_strategy))
        } else {
            Ok(Self::ShardingReadWriteSplitting(rw_strategy))
        }
    }

    pub fn new_with_simple_route(balance: BalanceType) -> Self {
        Self::Simple(balance)
    }

    pub fn new_with_sharding_only() -> Self {
        Self::Sharding
    }

    pub fn get_endpoint_group(nodegroup: &Option<config::NodeGroup>, rw_endpoint: &ReadWriteEndpoint) -> Result<IndexMap<String, ReadWriteEndpoint>, StragegyError> {
        let mut endpoint_group =
                IndexMap::<String, ReadWriteEndpoint>::new();

        match nodegroup {
            Some(group) => {
                let set: IndexSet<&String> = rw_endpoint.read.iter().map(|x| &x.name).collect();

                for member in group.members.iter() {
                    let currset: IndexSet<&String> = member.reads.iter().collect();
                    let intersec = set.intersection(&currset).collect::<Vec<_>>();
                    let read = intersec
                        .into_iter()
                        .filter_map(|x| rw_endpoint.read.iter().find(|r| &r.name == *x))
                        .cloned()
                        .collect::<Vec<_>>();
                    let readwrite = rw_endpoint
                        .readwrite
                        .iter()
                        .find(|x| x.name == member.readwrite)
                        .cloned()
                        .ok_or(StragegyError::EndpointNotFound(member.readwrite.clone()))?;

                    let rw = ReadWriteEndpoint { read, readwrite: vec![readwrite] };

                    endpoint_group.insert(member.name.clone(), rw);
                }

                Ok(endpoint_group)
            }
            None => Ok(endpoint_group)
        }
    }

    fn readwritesplitting_dispatch(strategy: &mut ReadWriteSplittingRouteStrategy, input: &RouteInput) -> Result<(Option<Endpoint>, TargetRole), BoxError> {
        match strategy {
            ReadWriteSplittingRouteStrategy::Static(ins) => ins.dispatch(input),
            ReadWriteSplittingRouteStrategy::Dynamic(ins) => ins.dispatch(input),
            _ => unreachable!()
        }
    }

    //fn gen_route_input<'a>(&'a self, input: RouteInput<'a>) -> RouteInput<'a> {
    //    match self {
    //        Self::ReadWriteSplitting(_) => {
    //            input
    //        }

    //        Self

    //        _ => unreachable!()
    //    }
    //}

}

impl Route for RouteStrategy {
    type Error = BoxError;

    fn dispatch(
        &mut self,
        input: &RouteInput,
    ) -> Result<(Option<Endpoint>, TargetRole), Self::Error> {
        match self {
            Self::ReadWriteSplitting(strategy) => {
                Self::readwritesplitting_dispatch(strategy, input)
            }

            Self::ShardingReadWriteSplitting(strateyy) => {
                Self::readwritesplitting_dispatch(strateyy, input)
            }

            Self::Sharding => {
                if let RouteInput::Sharding(input) = input {
                    Ok((Some(input.clone()), TargetRole::ReadWrite))
                } else {
                    unreachable!()
                }
            }
            
            Self::Simple(ins) => Ok((ins.next(), TargetRole::ReadWrite)),

            _ => unreachable!(),
        }
    }
}
