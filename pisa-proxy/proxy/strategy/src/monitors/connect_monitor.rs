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

use mysql_protocol::client::conn::ClientConn;
use tokio::time::{self, Duration};
use tracing::{debug, error};

use crate::{discovery::discovery::Monitor, readwritesplitting::ReadWriteEndpoint};

#[derive(Debug)]
pub struct MonitorConnect {
    pub user: String,
    pub password: String,
    pub connect_period: u64,
    pub connect_timeout: u64,
    pub connect_failure_threshold: u64,
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
            read.insert(r.addr, ConnectStatus::Connected);
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
        connect_period: u64,
        connect_timeout: u64,
        connect_failure_threshold: u64,
        rw_endpoint: ReadWriteEndpoint,
        connect_tx: crossbeam_channel::Sender<ConnectMonitorResponse>,
    ) -> Self {
        MonitorConnect {
            connect_tx,
            user,
            password,
            connect_period,
            connect_timeout,
            connect_failure_threshold,
            rw_endpoint,
        }
    }

    // probe datasource by connect
    pub async fn connnect_check(endpoint: String) -> ConnectStatus {
        let factory =
            ClientConn::with_opts(String::from("root"), String::from("12345678"), endpoint);
        match factory.connect().await {
            Ok(_) => return ConnectStatus::Connected,
            Err(_) => return ConnectStatus::Disconnected,
        }

        // match tokio::net::TcpStream::connect(endpoint.clone()).await {
        //     Ok(_) => ConnectStatus::Connected,
        //     Err(_) => ConnectStatus::Disconnected,
        // }
    }
}

#[async_trait::async_trait]
impl Monitor for MonitorConnect {
    async fn run_check(&self) {
        let connect_period = self.connect_period;
        let connect_failure_threshold = self.connect_failure_threshold;
        let connect_timeout = self.connect_timeout;
        let rw_endpoint = self.rw_endpoint.clone();
        let connect_tx = self.connect_tx.clone();

        // build connect monitor message channel
        let mut response = ConnectMonitorResponse::new(rw_endpoint.clone());

        tokio::spawn(async move {
            let mut retries = 1;
            loop {
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
                                    if retries > connect_failure_threshold {
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
                                        connect_period,
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
                                if retries > connect_failure_threshold {
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
                                    connect_period,
                                ));
                            },
                        }
                    }
                })
                .await
                {
                    debug!("connect monitor check timeout");
                }

                if let Err(e) = connect_tx.send(response.clone()) {
                    error!("send connect response err: {:#?}", e.into_inner());
                }

                // connect monitor probe interval
                std::thread::sleep(std::time::Duration::from_millis(connect_period));
            }
        });
    }
}
