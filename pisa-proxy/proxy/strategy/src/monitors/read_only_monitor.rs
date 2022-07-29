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
use mysql_protocol::{client::conn::ClientConn, row::RowData};
use pisa_error::error::{Error, ErrorKind};
use tokio::time::{self, Duration};
use tracing::{debug, error};

use crate::{
    discovery::discovery::Monitor,
    readwritesplitting::{dynamic_rw::MonitorResponse, ReadWriteEndpoint},
};

#[derive(Debug)]
pub struct MonitorReadOnly {
    pub user: String,
    pub password: String,
    pub read_only_period: u64,
    pub read_only_timeout: u64,
    pub read_only_failure_threshold: u64,
    pub monitor_response_tx: crossbeam_channel::Sender<MonitorResponse>,
    pub rw_endpoint: ReadWriteEndpoint,
}

#[derive(Debug, Clone)]
pub enum NodeRole {
    Master,
    Slave,
}

#[derive(Debug, Clone)]
pub struct ReadOnlyMonitorResponse {
    pub roles: HashMap<String, NodeRole>,
}

impl ReadOnlyMonitorResponse {
    pub fn new(rw_endpoint: ReadWriteEndpoint) -> Self {
        let mut roles = HashMap::new();

        for r in rw_endpoint.read {
            roles.insert(r.addr, NodeRole::Slave);
        }
        for rw in rw_endpoint.readwrite {
            roles.insert(rw.addr, NodeRole::Master);
        }
        ReadOnlyMonitorResponse { roles }
    }
}

impl MonitorReadOnly {
    pub fn new(
        user: String,
        password: String,
        read_only_period: u64,
        read_only_timeout: u64,
        read_only_failure_threshold: u64,
        monitor_response_tx: crossbeam_channel::Sender<MonitorResponse>,
        rw_endpoint: ReadWriteEndpoint,
    ) -> Self {
        MonitorReadOnly {
            user,
            password,
            read_only_period,
            read_only_timeout,
            read_only_failure_threshold,
            monitor_response_tx,
            rw_endpoint,
        }
    }

    // show variables like 'read_only';
    pub async fn read_only_check(
        user: String,
        password: String,
        addr: String,
    ) -> Result<NodeRole, Error> {
        let factory = ClientConn::with_opts(user, password, addr.clone());
        let mut client_conn = match factory.connect().await {
            Ok(client_conn) => client_conn,
            Err(e) => return Err(Error::new(ErrorKind::Protocol(e))),
        };

        let mut res = client_conn
            .query_result("SHOW VARIABLES LIKE 'read_only'".as_bytes())
            .await
            .unwrap()
            .unwrap();
        while let Some(data) = res.next().await {
            let mut row = data.unwrap();
            let read_only_status = row.decode_with_name::<String>("Variable_name").unwrap();
            let read_only_values = row.decode_with_name::<String>("Value").unwrap();

            if read_only_status.eq("read_only") && read_only_values.eq("ON") {
                return Ok(NodeRole::Slave);
            }
        }
        return Ok(NodeRole::Master);
    }
}

#[async_trait::async_trait]
impl Monitor for MonitorReadOnly {
    async fn run_check(&self) {
        let user = self.user.clone();
        let password = self.password.clone();
        let rw_endpoint = self.rw_endpoint.clone();
        let read_only_period = self.read_only_period.clone();
        let read_only_timeout = self.read_only_timeout;
        let read_only_failure_threshold = self.read_only_failure_threshold;
        let monitor_response_tx = self.monitor_response_tx.clone();

        let mut response = ReadOnlyMonitorResponse::new(rw_endpoint.clone());

        tokio::spawn(async move {
            let mut retries = 1;
            loop {
                // probe read endpoint
                for read in rw_endpoint.clone().read {
                    if let Err(_) = time::timeout(Duration::from_millis(read_only_timeout), async {
                        match MonitorReadOnly::read_only_check(
                            user.clone(),
                            password.clone(),
                            read.addr.clone(),
                        )
                        .await
                        {
                            Ok(read_only_status) => {
                                response.roles.insert(read.addr, read_only_status);
                            }
                            Err(_) => loop {
                                if retries > read_only_failure_threshold {
                                    retries = 1;
                                    break;
                                } else {
                                    match MonitorReadOnly::read_only_check(
                                        user.clone(),
                                        password.clone(),
                                        read.addr.clone(),
                                    )
                                    .await
                                    {
                                        Ok(read_only_status) => {
                                            response.roles.insert(read.addr, read_only_status);
                                            break;
                                        }
                                        Err(_) => retries += 1,
                                    }
                                }
                            },
                        }
                    })
                    .await
                    {
                        debug!("read only monitor check timeout");
                    }
                }

                for readwrite in rw_endpoint.clone().readwrite {
                    if let Err(_) = time::timeout(Duration::from_millis(read_only_timeout), async {
                        // ping_res include slave addr and latency from master
                        match MonitorReadOnly::read_only_check(
                            user.clone(),
                            password.clone(),
                            readwrite.addr.clone(),
                        )
                        .await
                        {
                            Ok(read_only_status) => {
                                response.roles.insert(readwrite.addr, read_only_status);
                            }
                            Err(_) => loop {
                                if retries > read_only_failure_threshold {
                                    retries = 1;
                                    break;
                                } else {
                                    match MonitorReadOnly::read_only_check(
                                        user.clone(),
                                        password.clone(),
                                        readwrite.addr.clone(),
                                    )
                                    .await
                                    {
                                        Ok(read_only_status) => {
                                            response.roles.insert(readwrite.addr, read_only_status);
                                            break;
                                        }
                                        Err(_) => retries += 1,
                                    }
                                }
                            },
                        }
                    })
                    .await
                    {
                        debug!("read only monitor check timeout");
                    };
                }
                println!("resonse {:#?}", response);
                if let Err(err) = monitor_response_tx
                    .send(MonitorResponse::ReadOnlyMonitorResponse(response.clone()))
                {
                    error!("send read only response err: {:#?}", err.into_inner());
                }
                std::thread::sleep(time::Duration::from_millis(read_only_period));
            }
        });
    }
}
