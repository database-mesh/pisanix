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

use std::collections::HashMap;
use futures::StreamExt;
use tokio::time::{self, Duration};

use pisa_error::error::{Error, ErrorKind};
use mysql_protocol::{client::conn::ClientConn, util::*};
use crate::{config::MasterHighAvailability, readwritesplitting::ReadWriteEndpoint};
use mysql_protocol::row::RowData;
use crate::monitors::connect_monitor::MonitorConnect;
use crate::monitors::ping_monitor::MonitorPing;
use crate::monitors::read_only_monitor::MonitorReadOnly;
use crate::monitors::replication_lag_monitor::MonitorReplicationLag;

//define discovery kind (support MHA,RDS,MGR etc.)
pub enum DiscoveryKind {
    MasterHighAvailability(DiscoveryMasterHighAvailability),
}

#[async_trait::async_trait]
pub trait Discovery {
    type Output;

    fn build(config: MasterHighAvailability, rw_endpoint: ReadWriteEndpoint) -> Self::Output;
    async fn run(&self, monitor_channel: crate::readwritesplitting::MonitorChannel);
}

pub struct DiscoveryMasterHighAvailability {
    config: MasterHighAvailability,
    rw_endpoint: ReadWriteEndpoint,
    pub monitors: Vec<MonitorKind>,
}

#[async_trait::async_trait]
impl Discovery for DiscoveryMasterHighAvailability {
    type Output = Self;

    fn build(config: MasterHighAvailability, rw_endpoint: ReadWriteEndpoint) -> Self::Output {
        Self { config, rw_endpoint, monitors: vec![] }
    }

    async fn run(&self, monitor_channel: crate::readwritesplitting::MonitorChannel) {
        let mut monitors = vec![];
        monitors.push(MonitorKind::Connect(MonitorConnect::new(
            self.config.user.clone(),
            self.config.password.clone(),
            self.config.connect_interval,
            self.config.connect_timeout,
            self.config.connect_max_failures,
            self.rw_endpoint.clone(),
            monitor_channel.connect_tx,
        )));
        monitors.push(MonitorKind::Ping(MonitorPing::new(
            self.config.user.clone(),
            self.config.password.clone(),
            self.config.ping_interval,
            self.config.ping_timeout,
            self.config.ping_max_failures,
            monitor_channel.ping_tx,
            self.rw_endpoint.clone(),
        )));
        monitors.push(MonitorKind::Lag(MonitorReplicationLag::new(
            self.config.user.clone(),
            self.config.password.clone(),
            self.config.replication_lag_interval,
            self.config.replication_lag_timeout,
            self.config.replication_lag_max_failures,
            self.config.max_replication_lag,
            monitor_channel.replication_lag_tx,
            self.rw_endpoint.clone(),
        )));
        monitors.push(MonitorKind::ReadOnly(MonitorReadOnly::new(
            self.config.user.clone(),
            self.config.password.clone(),
            self.config.read_only_interval,
            self.config.read_only_timeout,
            self.config.read_only_max_failures,
            monitor_channel.read_only_tx,
            self.rw_endpoint.clone(),
        )));

        for monitor in monitors {
            tokio::spawn(async move {
                monitor.run_check().await;
            });
        }
    }
}

#[derive(Debug)]
pub enum MonitorKind {
    Connect(MonitorConnect),
    Ping(MonitorPing),
    Lag(MonitorReplicationLag),
    ReadOnly(MonitorReadOnly),
}

#[async_trait::async_trait]
impl Monitor for MonitorKind {
    async fn run_check(&self) {
        match self {
            MonitorKind::Connect(inner_connect_monitor) => inner_connect_monitor.run_check().await,
            MonitorKind::Ping(inner_ping_monitor) => inner_ping_monitor.run_check().await,
            MonitorKind::Lag(inner_lag_monitor) => inner_lag_monitor.run_check().await,
            MonitorKind::ReadOnly(inner_read_only_monitor) => {
                inner_read_only_monitor.run_check().await
            }
        }
    }
}

#[derive(Debug)]
pub struct MonitorReconcile {}

#[async_trait::async_trait]
pub trait Monitor {
    async fn run_check(&self);
}