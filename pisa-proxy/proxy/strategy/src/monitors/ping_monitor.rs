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
use pisa_error::error::Error;
use tokio::time::{self, Duration};
use tracing::{debug, error};

use crate::{
    discovery::discovery::Monitor,
    readwritesplitting::{dynamic_rw::MonitorResponse, ReadWriteEndpoint},
};

#[derive(Debug)]
pub struct MonitorPing {
    pub user: String,
    pub password: String,
    pub ping_period: u64,
    pub ping_timeout: u64,
    pub ping_failure_threshold: u64,
    pub monitor_response_tx: crossbeam_channel::Sender<MonitorResponse>,
    pub rw_endpoint: ReadWriteEndpoint,
}

#[derive(Debug, Clone)]
pub enum PingStatus {
    PingOk,
    PingNotOk,
}

#[derive(Debug, Clone)]
pub struct PingMonitorResponse {
    pub read: HashMap<String, PingStatus>,
    pub readwrite: HashMap<String, PingStatus>,
}

impl PingMonitorResponse {
    pub fn new(rw_endpoint: ReadWriteEndpoint) -> Self {
        let mut read = HashMap::new();
        let mut readwrite = HashMap::new();
        for r in rw_endpoint.read {
            read.insert(r.addr, PingStatus::PingNotOk);
        }
        for rw in rw_endpoint.readwrite {
            readwrite.insert(rw.addr, PingStatus::PingNotOk);
        }
        PingMonitorResponse { read, readwrite }
    }
}

impl MonitorPing {
    pub fn new(
        user: String,
        password: String,
        ping_period: u64,
        ping_timeout: u64,
        ping_failure_threshold: u64,
        monitor_response_tx: crossbeam_channel::Sender<MonitorResponse>,
        rw_endpoint: ReadWriteEndpoint,
    ) -> Self {
        MonitorPing {
            user,
            password,
            ping_period,
            ping_timeout,
            ping_failure_threshold,
            monitor_response_tx,
            rw_endpoint,
        }
    }

    async fn ping_check(user: String, password: String, addr: String) -> Result<PingStatus, Error> {
        let factory = ClientConn::with_opts(user, password, addr.clone());
        let mut client_conn = match factory.connect().await {
            Ok(client_conn) => client_conn,
            Err(_) => return Ok(PingStatus::PingNotOk),
        };

        match client_conn.send_ping().await {
            Ok(ping_ok) => {
                if ping_ok {
                    return Ok(PingStatus::PingOk);
                } else {
                    return Ok(PingStatus::PingNotOk);
                }
            }
            Err(_) => return Ok(PingStatus::PingNotOk),
        }
    }
}

#[async_trait::async_trait]
impl Monitor for MonitorPing {
    async fn run_check(&self) {
        let user = self.user.clone();
        let password = self.password.clone();
        let ping_period = self.ping_period;
        let ping_timeout = self.ping_timeout;
        let ping_failure_threshold = self.ping_failure_threshold;
        let rw_endpoint = self.rw_endpoint.clone();
        let monitor_response_tx = self.monitor_response_tx.clone();

        let mut response = PingMonitorResponse::new(rw_endpoint.clone());

        tokio::spawn(async move {
            let mut retries = 1;
            loop {
                for read in &rw_endpoint.read {
                    if let Err(_) = time::timeout(Duration::from_millis(ping_timeout), async {
                        match MonitorPing::ping_check(
                            user.clone(),
                            password.clone(),
                            read.addr.clone(),
                        )
                        .await
                        {
                            Ok(ping_status) => match ping_status {
                                PingStatus::PingOk => {
                                    response.read.insert(read.addr.clone(), PingStatus::PingOk);
                                }
                                PingStatus::PingNotOk => loop {
                                    if retries > ping_failure_threshold {
                                        response
                                            .read
                                            .insert(read.addr.clone(), PingStatus::PingNotOk);
                                        retries = 1;
                                        break;
                                    } else {
                                        match MonitorPing::ping_check(
                                            user.clone(),
                                            password.clone(),
                                            read.addr.clone(),
                                        )
                                        .await
                                        {
                                            Ok(ping_status) => match ping_status {
                                                PingStatus::PingOk => {
                                                    response.read.insert(
                                                        read.addr.clone(),
                                                        PingStatus::PingOk,
                                                    );
                                                    retries = 1;
                                                    break;
                                                }
                                                PingStatus::PingNotOk => {
                                                    retries += 1;
                                                }
                                            },
                                            Err(_) => retries += 1,
                                        }
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(
                                        ping_period,
                                    ));
                                },
                            },
                            Err(_) => loop {
                                if retries > ping_failure_threshold {
                                    response.read.insert(read.addr.clone(), PingStatus::PingNotOk);
                                    retries = 1;
                                    break;
                                } else {
                                    match MonitorPing::ping_check(
                                        user.clone(),
                                        password.clone(),
                                        read.addr.clone(),
                                    )
                                    .await
                                    {
                                        Ok(ping_status) => match ping_status {
                                            PingStatus::PingOk => {
                                                response
                                                    .read
                                                    .insert(read.addr.clone(), PingStatus::PingOk);
                                                break;
                                            }
                                            PingStatus::PingNotOk => {
                                                retries += 1;
                                            }
                                        },
                                        Err(_) => retries += 1,
                                    }
                                }
                                std::thread::sleep(std::time::Duration::from_millis(ping_period));
                            },
                        }
                    })
                    .await
                    {
                        debug!("ping monitor check timeout");
                    }
                }

                for readwrite in &rw_endpoint.readwrite {
                    if let Err(_) = time::timeout(Duration::from_millis(ping_timeout), async {
                        match MonitorPing::ping_check(
                            user.clone(),
                            password.clone(),
                            readwrite.addr.clone(),
                        )
                        .await
                        {
                            Ok(ping_status) => match ping_status {
                                PingStatus::PingOk => {
                                    response.readwrite.insert(readwrite.addr.clone(), PingStatus::PingOk);
                                }
                                PingStatus::PingNotOk => loop {
                                    if retries > ping_failure_threshold {
                                        response
                                            .readwrite
                                            .insert(readwrite.addr.clone(), PingStatus::PingNotOk);
                                        retries = 1;
                                        break;
                                    } else {
                                        match MonitorPing::ping_check(
                                            user.clone(),
                                            password.clone(),
                                            readwrite.addr.clone(),
                                        )
                                        .await
                                        {
                                            Ok(ping_status) => match ping_status {
                                                PingStatus::PingOk => {
                                                    response.readwrite.insert(
                                                        readwrite.addr.clone(),
                                                        PingStatus::PingOk,
                                                    );
                                                    retries = 1;
                                                    break;
                                                }
                                                PingStatus::PingNotOk => {
                                                    retries += 1;
                                                }
                                            },
                                            Err(_) => {
                                                response.readwrite.insert(
                                                    readwrite.addr.clone(),
                                                    PingStatus::PingNotOk,
                                                );
                                            }
                                        }
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(
                                        ping_period,
                                    ));
                                },
                            },
                            Err(_) => loop {
                                if retries > ping_failure_threshold {
                                    response
                                        .read
                                        .insert(readwrite.addr.clone(), PingStatus::PingNotOk);
                                    retries = 1;
                                    break;
                                } else {
                                    match MonitorPing::ping_check(
                                        user.clone(),
                                        password.clone(),
                                        readwrite.addr.clone(),
                                    )
                                    .await
                                    {
                                        Ok(ping_status) => match ping_status {
                                            PingStatus::PingOk => {
                                                response.read.insert(
                                                    readwrite.addr.clone(),
                                                    PingStatus::PingOk,
                                                );
                                                retries = 1;
                                                break;
                                            }
                                            PingStatus::PingNotOk => {
                                                retries += 1;
                                            }
                                        },
                                        Err(_) => retries += 1,
                                    }
                                }
                                std::thread::sleep(std::time::Duration::from_millis(ping_period));
                            },
                        }
                    })
                    .await
                    {
                        debug!("ping monitor check timeout");
                    }
                }

                if let Err(err) =
                    monitor_response_tx.send(MonitorResponse::PingMonitorResponse(response.clone()))
                {
                    error!("send ping response err: {:#?}", err.into_inner());
                }
                std::thread::sleep(std::time::Duration::from_millis(ping_period));
            }
        });
    }
}
