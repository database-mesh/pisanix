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

// use conn_pool::{Pool, PoolConn};
// use mysql_protocol::client::conn::ClientConn;

use crate::config::MasterHighAvailability;

//define discovery kind (support MHA,RDS,MGR etc.)
pub enum DiscoveryKind {
    MasterHighAvailability(DiscoveryMasterHighAvailability),
}

pub trait Discovery {
    type Output;

    fn new(config: &MasterHighAvailability) -> Self::Output;
    fn register_monitor(&mut self);
    fn dispatch_event(&self);
    fn run(&self);
}

#[derive(Debug)]
pub struct DiscoveryMasterHighAvailability {
    pub user: String,
    pub password: String,
    pub monitor_interval: u64,
    pub connect_interval: u64,
    pub connect_timeout: u64,
    pub connect_max_failures: u64,
    pub ping_interval: u64,
    pub ping_timeout: u64,
    pub ping_max_failures: u64,
    pub replication_lag_interval: u64,
    pub replication_lag_timeout: u64,
    pub replication_lag_max_failures: u64,
    pub max_replication_lag: u64,
    pub read_only_interval: u64,
    pub read_only_timeout: u64,
    pub read_only_max_failures: u64,
}

impl Discovery for DiscoveryMasterHighAvailability {
    type Output = Self;

    fn new(config: &MasterHighAvailability) -> Self::Output {
        Self {
            user: config.user.clone(),
            password: config.password.clone(),
            monitor_interval: config.monitor_interval,
            connect_interval: config.connect_interval,
            connect_timeout: config.connect_timeout,
            connect_max_failures: config.connect_max_failures,
            ping_interval: config.ping_interval,
            ping_timeout: config.ping_timeout,
            ping_max_failures: config.ping_max_failures,
            replication_lag_interval: config.replication_lag_interval,
            replication_lag_timeout: config.replication_lag_timeout,
            replication_lag_max_failures: config.replication_lag_max_failures,
            max_replication_lag: config.max_replication_lag,
            read_only_interval: config.read_only_interval,
            read_only_timeout: config.read_only_timeout,
            read_only_max_failures: config.read_only_max_failures,
        }
    }

    fn register_monitor(&mut self) {}

    fn dispatch_event(&self) {}

    fn run(&self) {
        println!("run mha monitor");
    }
}

pub enum BaseMonitorKind {
    Connect(MonitorConnect),
    Ping(MonitorPing),
}

pub enum MHAMonitorKind {
    Lag(MonitorLag),
    ReadOnly(MonitorReadOnly),
}

// pub struct MonitorEvent {
//     pub event_kind: MonitorKind,
// }

// pub trait MonitorEventHandle {
//     fn on_event(&self, event: &MonitorEvent);
// }

pub struct MonitorReconcile {}

pub trait Monitor {
    fn run_check(&self);
}

pub trait Connect: Monitor {}
pub trait Ping: Monitor {}
pub trait Lag: Monitor {}
pub trait ReadOnly: Monitor {}

pub struct MonitorConnect {
    pub connect_interval: u64,
    pub connect_timeout: u64,
}

pub struct MonitorPing {
    pub ping_interval: u64,
    pub ping_timeout: u64,
    pub ping_max_failures: u64,
}

pub struct MonitorLag {
    pub replication_lag_interval: u64,
    pub replication_lag_timeout: u64,
    pub max_replication_lag: u64,
}

pub struct MonitorReadOnly {
    pub read_only_interval: u64,
    pub read_only_timeout: u64,
}
