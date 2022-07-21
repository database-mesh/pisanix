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

use crossbeam_channel::unbounded;
use endpoint::endpoint::Endpoint;
use futures::executor::block_on;
use loadbalance::balance::LoadBalance;
use tokio::sync::{Mutex, RwLock};

use super::{
    rule_match::{RulesMatch, RulesMatchBuilder},
    ReadWriteEndpoint,
};
use crate::{
    config,
    config::TargetRole,
    discovery::{
        discovery::{Discovery, DiscoveryKind, DiscoveryMasterHighAvailability},
        monitor_reconcile::MonitorReconcile,
    },
    route::{BoxError, RouteBalance},
    Route, RouteInput,
};

pub struct ReadWriteSplittingDynamicBuilder;

// define monitor channel
#[derive(Debug, Clone)]
pub struct MonitorChannel {
    pub connect_tx: crossbeam_channel::Sender<crate::discovery::discovery::ConnectMonitorResponse>,
    pub connect_rx:
        crossbeam_channel::Receiver<crate::discovery::discovery::ConnectMonitorResponse>,

    pub ping_tx: crossbeam_channel::Sender<crate::discovery::discovery::PingMonitorResponse>,
    pub ping_rx: crossbeam_channel::Receiver<crate::discovery::discovery::PingMonitorResponse>,

    pub replication_lag_tx:
        crossbeam_channel::Sender<crate::discovery::discovery::ReplicationLagMonitorResponse>,
    pub replication_lag_rx:
        crossbeam_channel::Receiver<crate::discovery::discovery::ReplicationLagMonitorResponse>,

    pub read_only_tx:
        crossbeam_channel::Sender<crate::discovery::discovery::ReadOnlyMonitorResponse>,
    pub read_only_rx:
        crossbeam_channel::Receiver<crate::discovery::discovery::ReadOnlyMonitorResponse>,
}

impl MonitorChannel {
    fn build() -> Self {
        let (connect_tx, connect_rx) = crossbeam_channel::unbounded();
        let (ping_tx, ping_rx) = crossbeam_channel::unbounded();
        let (replication_lag_tx, replication_lag_rx) = crossbeam_channel::unbounded();
        let (read_only_tx, read_only_rx) = crossbeam_channel::unbounded();

        MonitorChannel {
            connect_tx,
            connect_rx,
            ping_tx,
            ping_rx,
            replication_lag_tx,
            replication_lag_rx,
            read_only_tx,
            read_only_rx,
        }
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

        let monitor_channel = MonitorChannel::build();

        let mut reciver: Option<crossbeam_channel::Receiver<ReadWriteEndpoint>> = None;
        // Match discovery type
        match config.clone().discovery {
            // Use Master High Availability Discovery
            crate::config::Discovery::Mha(cc) => {
                let discovery_mha =
                    DiscoveryMasterHighAvailability::build(cc.clone(), rw_endpoint.clone());
                // discovery_mha.register_monitor();

                tokio_scoped::scope(|scope| {
                    scope.spawn(async {
                        discovery_mha.run(monitor_channel.clone()).await;
                    });
                });

                // discovery_mha.run();
                let mut monitor_reconcile =
                    MonitorReconcile::new(config.clone(), rw_endpoint.clone());
                reciver = Some(
                    monitor_reconcile.start_monitor_reconcile(cc.monitor_interval, monitor_channel),
                );

                // DiscoveryKind::MasterHighAvailability(discovery_mha);
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
        let mut rules_match = block_on(async move { rules_match_wrapper.lock_owned().await });
        let b = rules_match.get(input);
        Ok((b.0.next(), b.1))
    }
}
