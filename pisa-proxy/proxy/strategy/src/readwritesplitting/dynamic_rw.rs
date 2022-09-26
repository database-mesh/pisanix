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
use indexmap::IndexMap;
use loadbalance::balance::LoadBalance;

use super::{
    rule_match::{RulesMatch, RulesMatchBuilder},
    ReadWriteEndpoint,
};
use crate::{
    config,
    config::{ReadWriteSplittingRule, TargetRole},
    discovery::{
        discovery::{Discovery, DiscoveryMasterHighAvailability, Monitor},
        monitor_reconcile::MonitorReconcile,
    },
    monitors::{
        connect_monitor::ConnectMonitorResponse, ping_monitor::PingMonitorResponse,
        read_only_monitor::ReadOnlyMonitorResponse,
        replication_lag_monitor::ReplicationLagMonitorResponse,
    },
    route::{BoxError, RouteBalance, RouteStrategy},
    Route, RouteInput,
};

pub struct ReadWriteSplittingDynamicBuilder;

#[derive(Debug, Clone)]
pub enum MonitorResponse {
    ConnectMonitorResponse(ConnectMonitorResponse),
    PingMonitorResponse(PingMonitorResponse),
    ReplicationLagResponse(ReplicationLagMonitorResponse),
    ReadOnlyMonitorResponse(ReadOnlyMonitorResponse),
}

#[derive(Debug, Clone)]
pub struct MonitorResponseChannel {
    pub monitor_response_tx: crossbeam_channel::Sender<MonitorResponse>,
    pub monitor_response_rx: crossbeam_channel::Receiver<MonitorResponse>,
}

impl MonitorResponseChannel {
    fn build() -> Self {
        let (monitor_response_tx, monitor_response_rx) = crossbeam_channel::unbounded();

        MonitorResponseChannel { monitor_response_tx, monitor_response_rx }
    }
}

impl ReadWriteSplittingDynamicBuilder {
    pub fn build(
        config: config::ReadWriteSplittingDynamic,
        node_group_config: Option<config::NodeGroup>,
        endpoint_group: IndexMap<String, ReadWriteEndpoint>,
        rw_endpoint: ReadWriteEndpoint,
    ) -> ReadWriteSplittingDynamic {
        let rules_match = RulesMatchBuilder::build(
            config.clone().rules,
            config.clone().default_target,
            node_group_config.clone(),
            endpoint_group,
            rw_endpoint.clone(),
        );

        let monitor_response_channel = MonitorResponseChannel::build();

        // Match discovery type
        let reciver = match config.clone().discovery {
            // Use Master High Availability Discovery
            crate::config::Discovery::Mha(cc) => {
                let monitors =
                    DiscoveryMasterHighAvailability::build(cc.clone(), rw_endpoint.clone())
                        .build_monitors(monitor_response_channel.clone());
                let monitors_len = monitors.len();
                for monitor in monitors {
                    tokio::spawn(async move {
                        monitor.run_check().await;
                    });
                }

                let mut monitor_reconcile =
                    MonitorReconcile::new(config.clone(), rw_endpoint.clone());

                monitor_reconcile.start_monitor_reconcile(
                    cc.clone(),
                    monitor_response_channel.clone(),
                    monitors_len,
                )
            }
        };

        ReadWriteSplittingDynamic { rx: reciver, rules: config.clone().rules, node_group_config, rules_match }
    }
}

pub struct ReadWriteSplittingDynamic {
    rx: crossbeam_channel::Receiver<ReadWriteEndpoint>,
    rules: Vec<ReadWriteSplittingRule>,
    node_group_config: Option<config::NodeGroup>,
    rules_match: RulesMatch,
}

impl Route for ReadWriteSplittingDynamic {
    type Error = BoxError;

    fn dispatch(
        &mut self,
        input: &RouteInput,
    ) -> Result<(Option<Endpoint>, TargetRole), Self::Error> {
        let v: Vec<_> = self.rx.try_iter().collect();
        match v.last() {
            Some(rw_endpoint) => {
                self.rules_match.default_balance = RulesMatchBuilder::build_default_balance(
                    &self.rules_match.default_target,
                    rw_endpoint.clone(),
                );
                self.rules_match.default_trans_balance = RulesMatchBuilder::build_default_balance(
                    &TargetRole::ReadWrite,
                    rw_endpoint.clone(),
                );

                // `unwrap` is safely`,  because has initilized success when to here.
                let endpoint_group = RouteStrategy::get_endpoint_group(&self.node_group_config, rw_endpoint).unwrap();
                self.rules_match.inner = RulesMatchBuilder::build_rules(
                    self.rules.clone(),
                    endpoint_group,
                    rw_endpoint.clone(),
                    self.rules_match.default_target.clone(),
                );

                let b = self.rules_match.get(input);
                Ok((b.0.next(), b.1))
            }

            None => {
                let b = self.rules_match.get(input);
                Ok((b.0.next(), b.1))
            }
        }
    }
}
