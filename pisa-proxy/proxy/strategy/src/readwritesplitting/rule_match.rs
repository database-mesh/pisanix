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

use std::{error::Error, sync::Arc};

use endpoint::endpoint::Endpoint;
use loadbalance::balance::{AlgorithmName, Balance, BalanceType, LoadBalance};
use regex::Regex;
use tracing::debug;

use super::ReadWriteEndpoint;
use crate::{
    config::{GenericRule, ReadWriteSplittingRule, RegexRule, TargetRole},
    route::{RouteBalance, RouteRuleMatch},
    RouteInput,
};

pub struct RulesMatchBuilder;
use crate::readwritesplitting::*;

impl RulesMatchBuilder {
    pub fn build(
        rules: Vec<ReadWriteSplittingRule>,
        default_target: TargetRole,
        rw_endpoint: ReadWriteEndpoint,
    ) -> RulesMatch {
        let inner = RulesMatchBuilder::build_rules(
            rules.clone(),
            rw_endpoint.clone(),
            default_target.clone(),
        );
        let default_balance =
            RulesMatchBuilder::build_default_balance(&default_target, rw_endpoint.clone());

        let default_trans_balance =
            RulesMatchBuilder::build_default_balance(&TargetRole::ReadWrite, rw_endpoint);

        let rules_match = RulesMatch {
            default_target: default_target.clone(),
            default_trans_balance,
            inner,
            default_balance,
        };

        return rules_match;
    }

    pub fn build_rules(
        rules: Vec<ReadWriteSplittingRule>,
        rw_endpoint: ReadWriteEndpoint,
        default_target: TargetRole,
    ) -> Vec<RulesMatchInner> {
        let mut instances: Vec<RulesMatchInner> = Vec::with_capacity(rules.clone().len());
        let mut generic_instances: Vec<RulesMatchInner> = Vec::with_capacity(rules.clone().len());
        for r in &rules {
            match r {
                ReadWriteSplittingRule::Regex(r) => {
                    let inner = RegexRuleMatchInner::new(r.clone(), rw_endpoint.clone()).unwrap();
                    instances.push(RulesMatchInner::Regex(inner));
                }
                ReadWriteSplittingRule::Generic(r) => {
                    let inner = GenericRuleMatchInner::new(
                        r.clone(),
                        default_target.clone(),
                        rw_endpoint.clone(),
                    );
                    generic_instances.push(RulesMatchInner::Generic(inner));
                }
            }
        }

        instances.extend_from_slice(&generic_instances);

        instances
    }

    pub fn build_default_balance(
        default_target: &TargetRole,
        rw_endpoint: ReadWriteEndpoint,
    ) -> BalanceType {
        let mut default_balance = Balance.build_balance(AlgorithmName::Random);
        match default_target {
            TargetRole::Read => balance_add_endpoint(&mut default_balance, rw_endpoint.read),
            TargetRole::ReadWrite => {
                balance_add_endpoint(&mut default_balance, rw_endpoint.readwrite)
            }
        }
        default_balance
    }
}

pub struct RulesMatch {
    pub default_target: TargetRole,
    pub default_balance: BalanceType,
    // Default transaction balance
    pub default_trans_balance: BalanceType,
    pub inner: Vec<RulesMatchInner>,
}

#[derive(Debug, Clone)]
pub enum RulesMatchInner {
    Regex(RegexRuleMatchInner),
    Generic(GenericRuleMatchInner),
}

// Retrun balance when match success, otherwise return default_balance
impl RouteBalance for RulesMatch {
    fn get(&mut self, input: &RouteInput) -> (&mut BalanceType, TargetRole) {
        // Currently, if RouteInput variant type is Transaction, return readwrite balnace directly.
        if let RouteInput::Transaction(_) = input {
            return (&mut self.default_trans_balance, TargetRole::ReadWrite);
        }

        for rule in self.inner.iter_mut() {
            match rule {
                RulesMatchInner::Regex(inner) => {
                    if inner.is_match(input) {
                        return inner.get(input);
                    }
                }
                RulesMatchInner::Generic(inner) => {
                    if inner.is_match(input) {
                        return inner.get(input);
                    }
                }
            }
        }

        (&mut self.default_balance, self.default_target.clone())
    }
}

#[derive(Debug, Clone)]
pub struct RegexRuleMatchInner {
    rule: RegexRule,
    regexs: Vec<Regex>,
    balance: BalanceType,
}

impl RegexRuleMatchInner {
    fn new(
        rule: RegexRule,
        rw_endpoint: ReadWriteEndpoint,
    ) -> Result<RegexRuleMatchInner, Box<dyn Error>> {
        let balance = RegexRuleMatchInner::build_balance(
            &rule.target,
            rule.algorithm_name.clone(),
            rw_endpoint,
        );
        let regexs: Vec<Regex> = rule
            .regex
            .iter()
            .map(|r| Regex::new(r))
            .collect::<Result<Vec<Regex>, regex::Error>>()?;

        Ok(RegexRuleMatchInner { rule, regexs, balance })
    }

    fn build_balance(
        target: &TargetRole,
        algorithm_name: AlgorithmName,
        rw_endpoint: ReadWriteEndpoint,
    ) -> BalanceType {
        let mut balance = Balance.build_balance(algorithm_name);
        match target {
            TargetRole::Read => {
                if rw_endpoint.read.len() == 0 {
                    balance_add_endpoint(&mut balance, rw_endpoint.readwrite);
                }
                balance_add_endpoint(&mut balance, rw_endpoint.read);
            }

            TargetRole::ReadWrite => {
                balance_add_endpoint(&mut balance, rw_endpoint.readwrite);
            }
        };

        balance
    }
}

impl RouteRuleMatch for RegexRuleMatchInner {
    fn is_match(&self, input: &RouteInput) -> bool {
        match input {
            RouteInput::Statement(val) | RouteInput::Transaction(val) => {
                self.regexs.iter().any(|r| r.is_match(val))
            }

            RouteInput::None => false,
        }
    }
}

impl RouteBalance for RegexRuleMatchInner {
    fn get(&mut self, _input: &RouteInput) -> (&mut BalanceType, TargetRole) {
        (&mut self.balance, self.rule.target.clone())
    }
}

fn balance_add_endpoint(balance: &mut BalanceType, endpoints: Vec<Endpoint>) {
    for ep in endpoints {
        balance.add(ep);
    }
}

#[derive(Debug, Clone)]
pub struct GenericRuleMatchInner {
    r_balance: BalanceType,
    rw_balance: BalanceType,
    default_balance: BalanceType,
    default_target_role: TargetRole,
}

impl GenericRuleMatchInner {
    fn new(
        rule: GenericRule,
        default_target_role: TargetRole,
        rw_endpoint: ReadWriteEndpoint,
    ) -> GenericRuleMatchInner {
        let r_balance = GenericRuleMatchInner::build_balance(
            TargetRole::Read,
            rule.algorithm_name.clone(),
            rw_endpoint.clone(),
        );
        let rw_balance = GenericRuleMatchInner::build_balance(
            TargetRole::ReadWrite,
            rule.algorithm_name.clone(),
            rw_endpoint.clone(),
        );
        let default_balance = GenericRuleMatchInner::build_balance(
            default_target_role.clone(),
            rule.algorithm_name,
            rw_endpoint,
        );
        GenericRuleMatchInner { r_balance, rw_balance, default_balance, default_target_role }
    }

    fn build_balance(
        role: TargetRole,
        algorithm_name: AlgorithmName,
        rw_endpoint: ReadWriteEndpoint,
    ) -> BalanceType {
        let mut balance = Balance.build_balance(algorithm_name);

        match role {
            TargetRole::Read => {
                if rw_endpoint.read.len() == 0 {
                    balance_add_endpoint(&mut balance, rw_endpoint.readwrite);
                }
                balance_add_endpoint(&mut balance, rw_endpoint.read);
            }

            TargetRole::ReadWrite => {
                balance_add_endpoint(&mut balance, rw_endpoint.readwrite);
            }
        };

        balance
    }
}

impl RouteRuleMatch for GenericRuleMatchInner {
    fn is_match(&self, input: &RouteInput) -> bool {
        match input {
            RouteInput::Statement(sql) | RouteInput::Transaction(sql) => {
                let str_vec: Vec<&str> = sql.split(" ").collect();
                let token = str_vec[0].to_uppercase();
                if GENERIC_RULE_TOKEN.contains_key(&*token) {
                    return true;
                } else {
                    return false;
                }
            }

            RouteInput::None => false,
        }
    }
}

impl RouteBalance for GenericRuleMatchInner {
    fn get(&mut self, input: &RouteInput) -> (&mut BalanceType, TargetRole) {
        match input {
            RouteInput::Statement(sql) => match sql.split_once(" ") {
                Some(key_word) => {
                    let token = key_word.0.to_uppercase();
                    match token.as_str() {
                        "SELECT" => return (&mut self.r_balance, TargetRole::Read),
                        "INSERT" => return (&mut self.rw_balance, TargetRole::ReadWrite),
                        "UPDATE" => return (&mut self.rw_balance, TargetRole::ReadWrite),
                        "DELETE" => return (&mut self.rw_balance, TargetRole::ReadWrite),
                        "SET" => return (&mut self.rw_balance, TargetRole::ReadWrite),
                        "START" => return (&mut self.rw_balance, TargetRole::ReadWrite),
                        _ => return (&mut self.default_balance, self.default_target_role.clone()),
                    }
                }
                None => return (&mut self.default_balance, self.default_target_role.clone()),
            },
            RouteInput::Transaction(_) => {
                return (&mut self.rw_balance, TargetRole::ReadWrite);
            }
            RouteInput::None => {
                return (&mut self.default_balance, self.default_target_role.clone());
            }
        }
    }
}

#[cfg(test)]
mod test {
    use endpoint::endpoint::Endpoint;
    use loadbalance::balance::*;

    use super::RulesMatchBuilder;
    use crate::{config::*, readwritesplitting::ReadWriteEndpoint, RouteBalance, RouteInput};

    #[test]
    fn test_regex_match() {
        let rules = vec![
            ReadWriteSplittingRule::Regex(RegexRule {
                name: String::from("t1"),
                rule_type: String::from("regex"),
                regex: vec![String::from("^select")],
                target: TargetRole::Read,
                algorithm_name: AlgorithmName::Random,
            }),
            ReadWriteSplittingRule::Regex(RegexRule {
                name: String::from("t2"),
                rule_type: String::from("regex"),
                regex: vec![String::from("^insert")],
                target: TargetRole::Read,
                algorithm_name: AlgorithmName::Random,
            }),
        ];

        let default_target = TargetRole::ReadWrite;

        let rw_endpoint = ReadWriteEndpoint {
            read: vec![Endpoint {
                weight: 1,
                name: String::from("test1"),
                db: String::from("db"),
                user: String::from("user"),
                password: String::from("password"),
                addr: String::from("127.0.0.1"),
            }],
            readwrite: vec![Endpoint {
                weight: 1,
                name: String::from("test2"),
                db: String::from("db"),
                user: String::from("user"),
                password: String::from("password"),
                addr: String::from("127.0.0.2"),
            }],
        };

        let mut m = RulesMatchBuilder::build(rules, default_target, rw_endpoint);
        let (b, target) = m.get(&RouteInput::Statement("insert"));
        let endpoint = b.next();
        assert_eq!(target, TargetRole::Read);
        assert_eq!(endpoint.unwrap().name, "test1");
        let (b, target) = m.get(&RouteInput::Statement("create"));
        let endpoint = b.next();
        assert_eq!(target, TargetRole::ReadWrite);
        assert_eq!(endpoint.unwrap().name, "test2");
    }
}
