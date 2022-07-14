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

use std::{error::Error, ops::Deref, sync::Arc};
use std::{thread, time};

use crossbeam_channel::{unbounded, Select};
use endpoint::endpoint::Endpoint;
use loadbalance::balance::{AlgorithmName, Balance, BalanceType, LoadBalance};
use regex::Regex;
use tokio::sync::{Mutex, RwLock};

use super::ReadWriteEndpoint;
use crate::{
    config::{ReadWriteSplittingRule, RegexRule, TargetRole},
    route::{RouteBalance, RouteRuleMatch},
    RouteInput,
};

pub struct RulesMatchBuilder;

impl RulesMatchBuilder {
    pub fn build(
        rules: Vec<ReadWriteSplittingRule>,
        default_target: TargetRole,
        rw_endpoint: ReadWriteEndpoint,
    ) -> RulesMatch {
        let inner = RulesMatchBuilder::build_rules(rules.clone(), rw_endpoint.clone());
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

    fn build_rules(
        rules: Vec<ReadWriteSplittingRule>,
        rw_endpoint: ReadWriteEndpoint,
    ) -> Vec<RulesMatchInner> {
        let mut instances: Vec<RulesMatchInner> = Vec::with_capacity(rules.len());
        for r in rules {
            match r {
                ReadWriteSplittingRule::Regex(r) => {
                    let inner = RegexRuleMatchInner::new(r, rw_endpoint.clone()).unwrap();
                    instances.push(RulesMatchInner::Regex(inner));
                }
            }
        }
        instances
    }

    fn build_default_balance(
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

impl RulesMatch {
    pub async fn change(r: crossbeam_channel::Receiver<ReadWriteEndpoint>, inner: Arc<Mutex<RulesMatch>>) {
        tokio::spawn(async move {
            loop {
                println!("test: >>> {:#?}", r.clone().recv().unwrap());
                let ten_millis = time::Duration::from_millis(1000);
                thread::sleep(ten_millis);
            }
        });
    }

    pub async fn start_rules_match_reconcile(
        &mut self,
        rx: crossbeam_channel::Receiver<ReadWriteEndpoint>,
        rules: Vec<ReadWriteSplittingRule>,
        default_target: TargetRole,
    ) {
        tokio_scoped::scope(|scope| {
            scope.spawn(async {
                loop {
                    let rw_endpoint = rx.recv().unwrap();
                    self.default_target = default_target.clone();
                    self.default_balance = RulesMatchBuilder::build_default_balance(
                        &default_target,
                        rw_endpoint.clone(),
                    );
                    self.default_trans_balance = RulesMatchBuilder::build_default_balance(
                        &TargetRole::ReadWrite,
                        rw_endpoint.clone(),
                    );
                    self.inner = RulesMatchBuilder::build_rules(rules.clone(), rw_endpoint.clone());
                    // let ten_millis = time::Duration::from_millis(30000);
                    // thread::sleep(ten_millis);
                }
            });
        });
    }
}

pub enum RulesMatchInner {
    Regex(RegexRuleMatchInner),
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
            }
        }

        (&mut self.default_balance, self.default_target.clone())
    }
}

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
