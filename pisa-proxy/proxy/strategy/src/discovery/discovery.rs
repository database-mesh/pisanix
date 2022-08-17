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

use crate::{
    config::MasterHighAvailability,
    monitors::{
        connect_monitor::MonitorConnect, ping_monitor::MonitorPing,
        read_only_monitor::MonitorReadOnly, replication_lag_monitor::MonitorReplicationLag,
    },
    readwritesplitting::ReadWriteEndpoint,
};

//define discovery kind (support MHA,RDS,MGR etc.)
pub enum DiscoveryKind {
    MasterHighAvailability(DiscoveryMasterHighAvailability),
}

pub trait Discovery {
    type Output;

    fn build(config: MasterHighAvailability, rw_endpoint: ReadWriteEndpoint) -> Self::Output;
    fn build_monitors(
        &self,
        monitor_response_channel: crate::readwritesplitting::MonitorResponseChannel,
    ) -> Vec<MonitorKind>;
}

pub struct DiscoveryMasterHighAvailability {
    config: MasterHighAvailability,
    rw_endpoint: ReadWriteEndpoint,
    pub monitors: Vec<MonitorKind>,
}

impl Discovery for DiscoveryMasterHighAvailability {
    type Output = Self;

    fn build(config: MasterHighAvailability, rw_endpoint: ReadWriteEndpoint) -> Self::Output {
        Self { config, rw_endpoint, monitors: vec![] }
    }

    fn build_monitors(
        &self,
        monitor_response_channel: crate::readwritesplitting::MonitorResponseChannel,
    ) -> Vec<MonitorKind> {
        let mut monitors = vec![];
        monitors.push(MonitorKind::Connect(MonitorConnect::new(
            self.config.user.clone(),
            self.config.password.clone(),
            self.config.connect_period,
            self.config.connect_timeout,
            self.config.connect_failure_threshold,
            self.rw_endpoint.clone(),
            monitor_response_channel.monitor_response_tx.clone(),
        )));
        monitors.push(MonitorKind::Ping(MonitorPing::new(
            self.config.user.clone(),
            self.config.password.clone(),
            self.config.ping_period,
            self.config.ping_timeout,
            self.config.ping_failure_threshold,
            monitor_response_channel.monitor_response_tx.clone(),
            self.rw_endpoint.clone(),
        )));
        if self.config.read_only_enabled {
            monitors.push(MonitorKind::ReadOnly(MonitorReadOnly::new(
                self.config.user.clone(),
                self.config.password.clone(),
                self.config.read_only_period,
                self.config.read_only_timeout,
                self.config.read_only_failure_threshold,
                monitor_response_channel.monitor_response_tx.clone(),
                self.rw_endpoint.clone(),
            )));
        }
        if self.config.replication_lag_enabled {
            monitors.push(MonitorKind::ReplicationLag(MonitorReplicationLag::new(
                self.config.user.clone(),
                self.config.password.clone(),
                self.config.replication_lag_period,
                self.config.replication_lag_timeout,
                self.config.replication_lag_failure_threshold,
                self.config.max_replication_lag,
                monitor_response_channel.monitor_response_tx,
                self.rw_endpoint.clone(),
            )));
        }

        monitors
    }
}

#[derive(Debug)]
pub enum MonitorKind {
    Connect(MonitorConnect),
    Ping(MonitorPing),
    ReplicationLag(MonitorReplicationLag),
    ReadOnly(MonitorReadOnly),
}

#[async_trait::async_trait]
impl Monitor for MonitorKind {
    async fn run_check(&self) {
        match self {
            MonitorKind::Connect(inner_connect_monitor) => inner_connect_monitor.run_check().await,
            MonitorKind::Ping(inner_ping_monitor) => inner_ping_monitor.run_check().await,
            MonitorKind::ReplicationLag(inner_lag_monitor) => inner_lag_monitor.run_check().await,
            MonitorKind::ReadOnly(inner_read_only_monitor) => {
                inner_read_only_monitor.run_check().await
            }
        }
    }
}

#[async_trait::async_trait]
pub trait Monitor {
    async fn run_check(&self);
}
