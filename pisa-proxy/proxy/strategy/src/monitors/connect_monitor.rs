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
use crate::discovery::discovery::Monitor;

#[derive(Debug)]
pub struct MonitorConnect {
    pub user: String,
    pub password: String,
    pub connect_interval: u64,
    pub connect_timeout: u64,
    pub connect_max_failures: u64,
    pub rw_endpoint: ReadWriteEndpoint,
    pub connect_tx: crossbeam_channel::Sender<ConnectMonitorResponse>,
}

// define Connect Monitor probe status
#[derive(Debug, Clone)]
pub enum ConnectStatus {
    Connected,
    Disconnected,
}

// define COnnect Monitor response
#[derive(Debug, Clone)]
pub struct ConnectMonitorResponse {
    pub read: HashMap<String, ConnectStatus>,
    pub readwrite: HashMap<String, ConnectStatus>,
}

impl ConnectMonitorResponse {
    pub fn new(rw_endpoint: ReadWriteEndpoint) -> Self {
        let mut read = HashMap::new();
        let mut readwrite = HashMap::new();
        for r in rw_endpoint.read {
            read.insert(r.addr, ConnectStatus::Disconnected);
        }
        for rw in rw_endpoint.readwrite {
            readwrite.insert(rw.addr, ConnectStatus::Connected);
        }
        ConnectMonitorResponse { read, readwrite }
    }
}

// define Connect Monitor
impl MonitorConnect {
    pub fn new(
        user: String,
        password: String,
        connect_interval: u64,
        connect_timeout: u64,
        connect_max_failures: u64,
        rw_endpoint: ReadWriteEndpoint,
        connect_tx: crossbeam_channel::Sender<ConnectMonitorResponse>,
    ) -> Self {
        MonitorConnect {
            connect_tx,
            user,
            password,
            connect_interval,
            connect_timeout,
            connect_max_failures,
            rw_endpoint,
        }
    }

    // probe datasource by connect
    pub async fn connnect_check(endpoint: String) -> ConnectStatus {
        match tokio::net::TcpStream::connect(endpoint.clone()).await {
            Ok(_) => ConnectStatus::Connected,
            Err(_) => ConnectStatus::Disconnected,
        }
    }
}

#[async_trait::async_trait]
impl Monitor for MonitorConnect {
    async fn run_check(&self) {
        let connect_interval = self.connect_interval;
        let connect_max_failures = self.connect_max_failures;
        let connect_timeout = self.connect_timeout;
        let rw_endpoint = self.rw_endpoint.clone();
        let connect_tx = self.connect_tx.clone();

        // build connect monitor message channel
        let mut response = ConnectMonitorResponse::new(rw_endpoint.clone());

        tokio::spawn(async move {
            let mut retries = 1;
            loop {
                println!("connect check....");
                // maybe connection will timeout
                if let Err(_) = time::timeout(Duration::from_millis(connect_timeout), async {
                    // probe read endpoint
                    for read in rw_endpoint.clone().read {
                        let conn_res = MonitorConnect::connnect_check(read.addr.clone()).await;
                        match conn_res {
                            ConnectStatus::Connected => {
                                response.read.insert(read.addr.clone(), ConnectStatus::Connected);
                            }
                            ConnectStatus::Disconnected => {
                                // connect failures retry
                                loop {
                                    if retries > connect_max_failures {
                                        response
                                            .read
                                            .insert(read.addr.clone(), ConnectStatus::Disconnected);
                                        retries = 1;
                                        break;
                                    } else {
                                        match MonitorConnect::connnect_check(read.addr.clone())
                                            .await
                                        {
                                            ConnectStatus::Disconnected => retries += 1,
                                            ConnectStatus::Connected => {
                                                response.read.insert(
                                                    read.addr.clone(),
                                                    ConnectStatus::Connected,
                                                );
                                                break;
                                            }
                                        }
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(
                                        connect_interval,
                                    ));
                                }
                            }
                        }
                    }

                    // probe readwrite endpoint
                    for readwrite in rw_endpoint.clone().readwrite {
                        let conn_res = MonitorConnect::connnect_check(readwrite.addr.clone()).await;
                        match conn_res {
                            ConnectStatus::Connected => {
                                response
                                    .readwrite
                                    .insert(readwrite.addr.clone(), ConnectStatus::Connected);
                            }
                            ConnectStatus::Disconnected => loop {
                                if retries > connect_max_failures {
                                    response
                                        .readwrite
                                        .insert(readwrite.addr.clone(), conn_res.clone());
                                    retries = 0;
                                    break;
                                } else {
                                    match MonitorConnect::connnect_check(readwrite.addr.clone())
                                        .await
                                    {
                                        ConnectStatus::Disconnected => retries += 1,
                                        ConnectStatus::Connected => {
                                            response.readwrite.insert(
                                                readwrite.addr.clone(),
                                                ConnectStatus::Connected,
                                            );
                                            break;
                                        }
                                    }
                                }
                                std::thread::sleep(std::time::Duration::from_millis(
                                    connect_interval,
                                ));
                            },
                        }
                    }
                })
                .await
                {
                    // start connect max failures retry
                    if retries > connect_max_failures {
                        // after connect_max_failures retrying time send message to Monitor Reconcile
                        retries = 1;
                    }
                    retries += 1;
                }

                if let Err(e) = connect_tx.send(response.clone()) {
                    println!("{}", e);
                }

                // connect monitor probe interval
                std::thread::sleep(std::time::Duration::from_millis(connect_interval));
            }
        });
    }
}