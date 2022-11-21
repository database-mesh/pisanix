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

use crate::config::{Sharding, ShardingAlgorithmName, StrategyType};

#[derive(Debug)]
pub(crate) struct ShardingMetaBaseInfo<'a> {
    pub column: (Option<&'a str>, Option<&'a str>),
    pub count: (Option<u32>, Option<u32>),
    pub algo: (Option<&'a ShardingAlgorithmName>, Option<&'a ShardingAlgorithmName>),
}

pub trait ShardingMeta {
    fn get_sharding_column(&self) -> (Option<&str>, Option<&str>);
    fn get_algo(&self) -> (Option<&ShardingAlgorithmName>, Option<&ShardingAlgorithmName>);
    fn get_sharding_count(&self) -> (Option<u32>, Option<u32>);
    fn get_actual_schema<'a>(
        &'a self,
        endpoints: &'a [Endpoint],
        idx: Option<usize>,
    ) -> Option<&'a str>;
    fn get_endpoint<'a>(
        &'a self,
        endpoints: &'a [Endpoint],
        idx: Option<usize>,
    ) -> Option<Endpoint>;
    fn get_strategy_typ(&self) -> super::StrategyTyp;
}

/// Todo: use macro generate
impl ShardingMeta for Sharding {
    fn get_sharding_column(&self) -> (Option<&str>, Option<&str>) {
        if let Some(strategy) = &self.database_strategy {
            return strategy.get_sharding_column();
        }

        if let Some(strategy) = &self.table_strategy {
            return strategy.get_sharding_column();
        }

        if let Some(strategy) = &self.database_table_strategy {
            return strategy.get_sharding_column();
        }

        (None, None)
    }

    fn get_algo(&self) -> (Option<&ShardingAlgorithmName>, Option<&ShardingAlgorithmName>) {
        if let Some(strategy) = &self.database_strategy {
            return strategy.get_algo();
        }

        if let Some(strategy) = &self.table_strategy {
            return strategy.get_algo();
        }

        if let Some(strategy) = &self.database_table_strategy {
            return strategy.get_algo();
        }

        (None, None)
    }

    fn get_sharding_count(&self) -> (Option<u32>, Option<u32>) {
        if let Some(_) = &self.database_strategy {
            return (Some(self.actual_datanodes.len() as u32), None);
        }

        if let Some(strategy) = &self.table_strategy {
            return (None, strategy.get_sharding_count().1);
        }

        if let Some(strategy) = &self.database_table_strategy {
            return (Some(self.actual_datanodes.len() as u32), strategy.get_sharding_count().1);
        }

        (None, None)
    }

    fn get_actual_schema<'a>(
        &self,
        endpoints: &'a [Endpoint],
        idx: Option<usize>,
    ) -> Option<&'a str> {
        if self.database_strategy.is_some() || self.database_table_strategy.is_some() {
            let ep = endpoints.iter().find(|ep| ep.name == self.actual_datanodes[idx.unwrap()]);
            return ep.map(|x| x.db.as_str());
        }

        None
    }

    fn get_endpoint(&self, endpoints: &[Endpoint], idx: Option<usize>) -> Option<Endpoint> {
        let idx = if self.table_strategy.is_some() { 0 } else { idx.unwrap() };
        endpoints.iter().find(|ep| ep.name == self.actual_datanodes[idx]).map(|x| x.clone())
    }

    fn get_strategy_typ(&self) -> super::StrategyTyp {
        if self.database_strategy.is_some() {
            super::StrategyTyp::Database
        } else if self.table_strategy.is_some() {
            super::StrategyTyp::Table
        } else {
            super::StrategyTyp::DatabaseTable
        }
    }
}

impl ShardingMeta for StrategyType {
    fn get_sharding_column(&self) -> (Option<&str>, Option<&str>) {
        match self {
            Self::DatabaseStrategyConfig(config) => (Some(&config.database_sharding_column), None),

            Self::DatabaseTableStrategyConfig(config) => {
                (Some(&config.database_sharding_column), Some(&config.table_sharding_column))
            }

            Self::TableStrategyConfig(config) => (None, Some(&config.table_sharding_column)),

            _ => (None, None),
        }
    }

    fn get_algo(&self) -> (Option<&ShardingAlgorithmName>, Option<&ShardingAlgorithmName>) {
        match self {
            Self::DatabaseStrategyConfig(config) => {
                (Some(&config.database_sharding_algorithm_name), None)
            }

            Self::DatabaseTableStrategyConfig(config) => (
                Some(&config.database_sharding_algorithm_name),
                Some(&config.table_sharding_algorithm_name),
            ),

            Self::TableStrategyConfig(config) => {
                (None, Some(&config.table_sharding_algorithm_name))
            }

            _ => (None, None),
        }
    }

    fn get_sharding_count(&self) -> (Option<u32>, Option<u32>) {
        match self {
            Self::DatabaseStrategyConfig(_) => {
                unimplemented!()
            }

            Self::DatabaseTableStrategyConfig(config) => (None, Some(config.shading_count.into())),

            Self::TableStrategyConfig(config) => (None, Some(config.sharding_count.into())),

            _ => (None, None),
        }
    }

    fn get_actual_schema<'a>(
        &'a self,
        _endpoints: &'a [Endpoint],
        _idx: Option<usize>,
    ) -> Option<&'a str> {
        None
    }

    fn get_endpoint(&self, _endpoints: &[Endpoint], _idx: Option<usize>) -> Option<Endpoint> {
        None
    }

    fn get_strategy_typ(&self) -> super::StrategyTyp {
        unimplemented!()
    }
}
