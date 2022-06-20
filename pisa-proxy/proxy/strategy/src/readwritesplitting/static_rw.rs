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


use std::marker::PhantomData;

use endpoint::endpoint::Endpoint;
use loadbalance::balance::BalanceType;
use loadbalance::balance::LoadBalance;

use crate::RouteInput;
use crate::Route;
use crate::config;
use crate::config::TargetRole;
use crate::route::RouteBalance;

use super::ReadWriteEndpoint;
use super::rule_match::RulesMatch;
use super::rule_match::RulesMatchBuilder;


pub struct ReadWriteSplittingStaticBuilder;

impl ReadWriteSplittingStaticBuilder {
    pub fn build(config: config::ReadWriteSplittingStatic, rw_endpoint: ReadWriteEndpoint) -> ReadWriteSplittingStatic {
        let rules_match = RulesMatchBuilder::build(config.rules, config.default_target, rw_endpoint);

        ReadWriteSplittingStatic { rules_match }
    }
}

pub struct ReadWriteSplittingStatic {
   rules_match: RulesMatch
}

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

impl Route for ReadWriteSplittingStatic {
    type Error = BoxError;
    fn do_route<'a>(&'a mut self, input: RouteInput) -> Result<Option<&'a Endpoint>, Self::Error> {
        let b = self.rules_match.get(&input);
        Ok(b.0.next())
    }
}


#[cfg(test)]
mod test {
    use endpoint::endpoint::Endpoint;
    use loadbalance::balance::AlgorithmName;

    use crate::{route::{Route, RouteInput}, config::{ReadWriteSplittingRule, RegexRule, TargetRole}, readwritesplitting::{ReadWriteEndpoint, rule_match::RulesMatchBuilder, static_rw::ReadWriteSplittingStaticBuilder}};


    #[test]
    fn test_route() {
        let rules = vec![
            ReadWriteSplittingRule::Regex(
                RegexRule { 
                    name: String::from("t1"), 
                    rule_type: String::from("regex"), 
                    regex: vec![String::from("^select")], 
                    target: TargetRole::Read, 
                    algorithm_name: AlgorithmName::Random,
                }
            ),

            ReadWriteSplittingRule::Regex(
                RegexRule { 
                    name: String::from("t2"), 
                    rule_type: String::from("regex"), 
                    regex: vec![String::from("^insert")], 
                    target: TargetRole::Read, 
                    algorithm_name: AlgorithmName::Random,
                }
            )
        ];

        let default_target = TargetRole::ReadWrite;
        
        let rw_endpoint = ReadWriteEndpoint {
            read: vec![
                Endpoint { 
                    weight: 1, 
                    name: String::from("test1"), 
                    db: String::from("db"), 
                    user: String::from("user"), 
                    password: String::from("password"), 
                    addr: String::from("127.0.0.1") 
                },
            ],
            readwrite: vec![
                Endpoint { 
                    weight: 1, 
                    name: String::from("test2"), 
                    db: String::from("db"), 
                    user: String::from("user"), 
                    password: String::from("password"), 
                    addr: String::from("127.0.0.2") 
                },
            ],
        };

        let config = super::config::ReadWriteSplitting {
            model: super::config::ReadWriteSplittingStatic {
                default_target,
                rules,
            },
        };

        let mut rws = ReadWriteSplittingStaticBuilder::build(config.model, rw_endpoint);
        let input = RouteInput::Statement("insert".to_string());
        let res = rws.do_route(input).unwrap();
        println!("{:#?}", res);
    }
}