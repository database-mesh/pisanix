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
pub struct MonitorReadOnly {
    pub user: String,
    pub password: String,
    pub read_only_interval: u64,
    pub read_only_timeout: u64,
    pub read_only_max_failures: u64,
    pub read_only_tx: crossbeam_channel::Sender<ReadOnlyMonitorResponse>,
    pub rw_endpoint: ReadWriteEndpoint,
}

#[derive(Debug, Clone)]
pub struct ReadOnlyMonitorResponse {
    read: HashMap<String, String>,
    readwrite: HashMap<String, String>,
}

impl ReadOnlyMonitorResponse {
    pub fn new(rw_endpoint: ReadWriteEndpoint) -> Self {
        let mut read = HashMap::new();
        let mut readwrite = HashMap::new();
        for r in rw_endpoint.read {
            read.insert(r.addr, String::from("OFF"));
        }
        for rw in rw_endpoint.readwrite {
            readwrite.insert(rw.addr, String::from("OFF"));
        }
        ReadOnlyMonitorResponse { read, readwrite }
    }
}

impl MonitorReadOnly {
    pub fn new(
        user: String,
        password: String,
        read_only_interval: u64,
        read_only_timeout: u64,
        read_only_max_failures: u64,
        read_only_tx: crossbeam_channel::Sender<ReadOnlyMonitorResponse>,
        rw_endpoint: ReadWriteEndpoint,
    ) -> Self {
        MonitorReadOnly {
            user,
            password,
            read_only_interval,
            read_only_timeout,
            read_only_max_failures,
            read_only_tx,
            rw_endpoint,
        }
    }

    // show variables like 'read_only';
    async fn read_only_check(
        user: String,
        password: String,
        addr: String,
    ) -> Result<Option<String>, Error> {
        let mut res_read_only_status: Option<String> = None;

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
            // let mut row =
            //     mysql_protocol::row::RowData::new(res.0.clone(), &data.as_mut().unwrap()[4..]);
            let read_only_status = row.decode_with_name::<String>("Variable_name").unwrap();
            let read_only_values = row.decode_with_name::<String>("Value").unwrap();

            if read_only_status.eq("read_only") {
                res_read_only_status = Some(read_only_values);
            }
        }
        Ok(res_read_only_status)
    }
}

#[async_trait::async_trait]
impl Monitor for MonitorReadOnly {
    async fn run_check(&self) {
        let user = self.user.clone();
        let password = self.password.clone();
        let rw_endpoint = self.rw_endpoint.clone();
        let read_only_interval = self.read_only_interval.clone();
        let read_only_timeout = self.read_only_timeout;
        let read_only_max_failures = self.read_only_max_failures;
        let read_only_tx = self.read_only_tx.clone();

        let mut response = ReadOnlyMonitorResponse::new(rw_endpoint.clone());

        tokio::spawn(async move {
            let mut retries = 1;
            loop {
                if let Err(_) = time::timeout(Duration::from_millis(read_only_timeout), async {
                    // probe read endpoint
                    for read in rw_endpoint.clone().read {
                        // ping_res include slave addr and latency from master
                        match MonitorReadOnly::read_only_check(
                            user.clone(),
                            password.clone(),
                            read.addr.clone(),
                        )
                        .await
                        {
                            Ok(read_only_status) => {
                                match read_only_status {
                                    Some(read_only_status) => {
                                        response.read.insert(read.addr, read_only_status);
                                    }
                                    None => {
                                        continue;
                                    }
                                }
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    }

                    for readwrite in rw_endpoint.clone().readwrite {
                        // ping_res include slave addr and latency from master
                        match MonitorReadOnly::read_only_check(
                            user.clone(),
                            password.clone(),
                            readwrite.addr.clone(),
                        )
                        .await
                        {
                            Ok(read_only_status) => {
                                match read_only_status {
                                    Some(read_only_status) => {
                                        response.readwrite.insert(readwrite.addr, read_only_status);
                                    }
                                    None => {
                                        continue;
                                    }
                                }
                            },
                            Err(_) => {
                                continue;
                            }
                        }
                    }
                })
                .await
                {
                    if retries > read_only_max_failures {
                        retries = 1;
                    }
                    retries += 1;
                    std::thread::sleep(time::Duration::from_millis(read_only_interval));
                }

                if let Err(e) = read_only_tx.send(response.clone()) {
                    println!("{}", e);
                }
                std::thread::sleep(time::Duration::from_millis(read_only_interval));
            }
        });
    }
}
