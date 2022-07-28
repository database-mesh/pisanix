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

use std::sync::Arc;

use endpoint::endpoint::Endpoint;
use loadbalance::balance::LoadBalance;
use parking_lot::Mutex;

use super::{
    rule_match::{RulesMatch, RulesMatchBuilder},
    ReadWriteEndpoint,
};
use crate::{
    config,
    config::TargetRole,
    discovery::{
        discovery::{Discovery, DiscoveryMasterHighAvailability, Monitor},
        monitor_reconcile::MonitorReconcile,
    },
    monitors::{
        connect_monitor::ConnectMonitorResponse, ping_monitor::PingMonitorResponse,
        read_only_monitor::ReadOnlyMonitorResponse,
        replication_lag_monitor::ReplicationLagMonitorResponse,
    },
    route::{BoxError, RouteBalance},
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
        rw_endpoint: ReadWriteEndpoint,
    ) -> ReadWriteSplittingDynamic {
        let rules_match = RulesMatchBuilder::build(
            config.clone().rules,
            config.clone().default_target,
            rw_endpoint.clone(),
        );

        // let monitor_channel = MonitorChannel::build();
        let monitor_response_channel = MonitorResponseChannel::build();

        let mut reciver: Option<crossbeam_channel::Receiver<ReadWriteEndpoint>> = None;

        // Match discovery type
        match config.clone().discovery {
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

                reciver =
                    Some(monitor_reconcile.start_monitor_reconcile(
                        cc.monitor_period,
                        monitor_response_channel.clone(),
                        monitors_len,
                    ));
            }
        };

        let rules_match_wrapper = Arc::new(Mutex::new(rules_match));
        let rm = rules_match_wrapper.clone();
        tokio::spawn(async move {
            RulesMatch::start_rules_match_reconcile(
                reciver.unwrap().clone(),
                rm,
                config.clone().rules,
                config.clone().default_target,
            )
            .await;
        });
        ReadWriteSplittingDynamic { rules_match: rules_match_wrapper.clone() }
    }
}

pub struct ReadWriteSplittingDynamic {
    rules_match: Arc<Mutex<RulesMatch>>,
}

impl Route for ReadWriteSplittingDynamic {
    type Error = BoxError;

    fn dispatch(
        &mut self,
        input: &RouteInput,
    ) -> Result<(Option<Endpoint>, TargetRole), Self::Error> {
        let rules_match_wrapper = self.rules_match.clone();
        let mut rules_match = rules_match_wrapper.lock();
        let b = rules_match.get(input);
        Ok((b.0.next(), b.1))
    }
}
