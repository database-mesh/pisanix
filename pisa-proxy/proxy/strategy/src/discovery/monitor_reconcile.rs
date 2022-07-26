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

use tracing::debug;
use crossbeam_channel::unbounded;
use crate::{config::ReadWriteSplittingDynamic, readwritesplitting::ReadWriteEndpoint};

pub struct MonitorReconcile {
    config: crate::config::Discovery,
    rw_endpoint: ReadWriteEndpoint,
}

impl MonitorReconcile {
    pub fn new(config: ReadWriteSplittingDynamic, rw_endpoint: ReadWriteEndpoint) -> Self {
        MonitorReconcile { config: config.discovery, rw_endpoint }
    }

    pub fn start_monitor_reconcile(
        &mut self,
        monitor_interval: u64,
        monitor_channel: crate::readwritesplitting::MonitorChannel,
    ) -> crossbeam_channel::Receiver<ReadWriteEndpoint> {
        let (send, recv) = unbounded();
        let tx = send.clone();
        let rx = recv.clone();

        let rw_endpoint = self.rw_endpoint.clone();

        tokio::spawn(async move {
            MonitorReconcile::report(tx, monitor_interval, rw_endpoint, monitor_channel).await;
        });

        rx
    }

    async fn report(
        s: crossbeam_channel::Sender<ReadWriteEndpoint>,
        monitor_interval: u64,
        rw_endpoint: ReadWriteEndpoint,
        monitor_channel: crate::readwritesplitting::MonitorChannel,
    ) {
        tokio::task::spawn_blocking(move || loop {
            loop {
                let mut curr_rw_endpoint = rw_endpoint.clone();
                let mut replication_lag_monitor_response: Option<
                    crate::monitors::replication_lag_monitor::ReplicationLagMonitorResponse,
                > = None;

                let connect_monitor_response = monitor_channel.connect_rx.recv().unwrap();
                let ping_monitor_response = monitor_channel.ping_rx.recv().unwrap();

                match monitor_channel.replication_lag_rx.recv() {
                    Ok(replication_lag_response) => {
                        replication_lag_monitor_response = Some(replication_lag_response);
                    }
                    Err(_) => {}
                };

                let read_only_response = monitor_channel.read_only_rx.recv().unwrap();

                for (read_write_connect_addr, read_write_connect_status) in
                    connect_monitor_response.clone().readwrite
                {
                    match read_write_connect_status {
                        // check master connected
                        crate::monitors::connect_monitor::ConnectStatus::Connected => {
                            for (read_write_ping_addr, read_write_ping_status) in
                                ping_monitor_response.clone().readwrite
                            {
                                match read_write_ping_status {
                                    // if master connected is ok, check ping
                                    crate::monitors::ping_monitor::PingStatus::PingOk => {}
                                    crate::monitors::ping_monitor::PingStatus::PingNotOk => {
                                        if curr_rw_endpoint.clone().read.len() > 0 {
                                            //check if slave is change to master
                                            for read_endpoint in curr_rw_endpoint.clone().read {
                                                match read_only_response.roles.get(&read_endpoint.addr).unwrap() {
                                                    // slave change to master
                                                    crate::monitors::read_only_monitor::NodeRole::Master => {
                                                        // clean readwrite list
                                                        curr_rw_endpoint.readwrite = vec![];
                                                        // add new read write into master list
                                                        curr_rw_endpoint.readwrite.push(read_endpoint);
                                                        //TODO send replication_lag_endpoint tx to update read only list
                                                        // replication_lag_endpoint_tx.send(curr_rw_endpoint.clone()).unwrap();
                                                    },
                                                    // slave doesn't change to master
                                                    crate::monitors::read_only_monitor::NodeRole::Slave => {
                                                        // nothing to do
                                                        continue;
                                                    },
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // master node connected failed
                        crate::monitors::connect_monitor::ConnectStatus::Disconnected => {
                            // check if slave is change to master
                            for read_endpoint in curr_rw_endpoint.clone().read {
                                match read_only_response.roles.get(&read_endpoint.addr).unwrap() {
                                    // slave change to master
                                    crate::monitors::read_only_monitor::NodeRole::Master => {
                                        curr_rw_endpoint.readwrite = vec![];
                                        // add new read write into master list
                                        curr_rw_endpoint.readwrite.push(read_endpoint);
                                    }
                                    // slave doesn't change to master
                                    crate::monitors::read_only_monitor::NodeRole::Slave => {
                                        // nothing to do
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                for (read_addr, read_connect_status) in connect_monitor_response.clone().read {
                    match read_connect_status {
                        crate::monitors::connect_monitor::ConnectStatus::Connected => {
                            for (read_ping_addr, read_ping_status) in ping_monitor_response.clone().read {
                                match read_ping_status {
                                    crate::monitors::ping_monitor::PingStatus::PingOk => {
                                        match replication_lag_monitor_response.clone() {
                                            Some(replication_lag_response) => {
                                                for (replication_lag_addr, lag_status) in &replication_lag_response.latency {
                                                    if !lag_status.is_latency {
                                                        curr_rw_endpoint.read.append(&mut curr_rw_endpoint.read.clone());
                                                        continue;
                                                    } else {
                                                        // add replication_lag_addr to read_write list
                                                        // curr_rw_endpoint.readwrite.push(rw_endpoint.read.iter().find(|r| {r.addr.eq(replication_lag_addr)}).unwrap().clone());
                                                        curr_rw_endpoint.read.remove(rw_endpoint.read.iter().position(|r| {r.addr.eq(replication_lag_addr)}).unwrap());
                                                    }
                                                }
                                            }
                                            None => {}
                                        }
                                    }
                                    crate::monitors::ping_monitor::PingStatus::PingNotOk => {
                                        curr_rw_endpoint.read.remove(rw_endpoint.read.iter().position(|r| {r.addr.eq(&read_ping_addr)}).unwrap());
                                        // curr_rw_endpoint.readwrite.push(
                                        //     rw_endpoint.read.iter().find(|r| r.addr.eq(&read_ping_addr)).unwrap().clone());
                                    }
                                }
                            }
                        }
                        crate::monitors::connect_monitor::ConnectStatus::Disconnected => {
                            curr_rw_endpoint.read.remove(
                                rw_endpoint.read.iter().position(|r| r.addr.eq(&read_addr)).unwrap(),
                            );
                            // curr_rw_endpoint.readwrite.append(&mut curr_rw_endpoint.read.clone());
                        }
                    }
                }

                if let Err(err) = s.try_send(curr_rw_endpoint) {
                    debug!("err >>> {:#?}", err);
                }

                std::thread::sleep(std::time::Duration::from_millis(monitor_interval));
            }
        });
    }
}
