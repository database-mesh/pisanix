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
use mysql_protocol::{client::conn::ClientConn, row::RowData, util::*};
use pisa_error::error::{Error, ErrorKind};
use tokio::time::{self, Duration};

use crate::{
    config::MasterHighAvailability, discovery::discovery::Monitor,
    readwritesplitting::ReadWriteEndpoint,
};

#[derive(Debug)]
pub struct MonitorReplicationLag {
    pub user: String,
    pub password: String,
    pub replication_lag_interval: u64,
    pub replication_lag_timeout: u64,
    pub replication_lag_max_failures: u64,
    pub max_replication_lag: u64,
    pub replication_lag_tx: crossbeam_channel::Sender<ReplicationLagMonitorResponse>,
    pub rw_endpoint: ReadWriteEndpoint,
}

impl MonitorReplicationLag {
    pub fn new(
        user: String,
        password: String,
        replication_lag_interval: u64,
        replication_lag_timeout: u64,
        replication_lag_max_failures: u64,
        max_replication_lag: u64,
        replication_lag_tx: crossbeam_channel::Sender<ReplicationLagMonitorResponse>,
        rw_endpoint: ReadWriteEndpoint,
    ) -> Self {
        MonitorReplicationLag {
            user,
            password,
            replication_lag_interval,
            replication_lag_timeout,
            replication_lag_max_failures,
            max_replication_lag,
            replication_lag_tx,
            rw_endpoint,
        }
    }

    async fn replication_lag_check(
        user: String,
        password: String,
        addr: String,
    ) -> Result<Option<u64>, Error> {
        let mut reasponse_replication_lag: Option<u64> = None;

        let factory = ClientConn::with_opts(user, password, addr.clone());
        let mut client_conn = match factory.connect().await {
            Ok(client_conn) => client_conn,
            Err(e) => return Err(Error::new(ErrorKind::Protocol(e))),
        };

        let mut res =
            client_conn.query_result("show slave status".as_bytes()).await.unwrap().unwrap();

        while let Some(data) = res.next().await {
            let mut row = data.unwrap();
            let seconds_behind_master =
                row.decode_with_name::<Option<u64>>("Seconds_Behind_Master").unwrap();
            match seconds_behind_master {
                Some(lag) => reasponse_replication_lag = Some(lag),
                None => {}
            }
        }
        Ok(reasponse_replication_lag)
    }
}

#[derive(Debug, Clone)]
pub struct ReplicationLagResponseInner {
    lag: u64,
    pub is_latency: bool,
}

#[derive(Debug, Clone)]
pub struct ReplicationLagMonitorResponse {
    // define slave late from master
    pub latency: HashMap<String, ReplicationLagResponseInner>,
}

impl ReplicationLagMonitorResponse {
    fn new(rw_endpoint: ReadWriteEndpoint) -> Self {
        let mut latency = HashMap::new();

        for r in rw_endpoint.read {
            let inner = ReplicationLagResponseInner { lag: 0, is_latency: true };
            latency.insert(r.addr.clone(), inner);
        }

        ReplicationLagMonitorResponse { latency }
    }
}

#[async_trait::async_trait]
impl Monitor for MonitorReplicationLag {
    async fn run_check(&self) {
        let user = self.user.clone();
        let password = self.password.clone();
        let replication_lag_timeout = self.replication_lag_timeout;
        let replication_lag_max_failures = self.replication_lag_max_failures;
        let reaplication_lag_interval = self.replication_lag_interval;
        let rw_endpoint = self.rw_endpoint.clone();
        let replication_lag_tx = self.replication_lag_tx.clone();
        // customer define threshold
        let max_replication_lag = self.max_replication_lag;
        let mut response = ReplicationLagMonitorResponse::new(rw_endpoint.clone());

        tokio::spawn(async move {
            let mut retries = 1;
            loop {
                if let Err(_) =
                    time::timeout(Duration::from_millis(replication_lag_timeout), async {
                        // probe read endpoint
                        for read in rw_endpoint.clone().read {
                            // ping_res include slave addr and latency from master
                            match MonitorReplicationLag::replication_lag_check(
                                user.clone(),
                                password.clone(),
                                read.addr.clone(),
                            )
                            .await
                            {
                                Ok(lag) => {
                                    match lag {
                                        Some(lag) => {
                                            if lag > max_replication_lag {
                                                loop {
                                                    if retries > replication_lag_max_failures {
                                                        response.latency.insert(
                                                            read.addr.clone(),
                                                            ReplicationLagResponseInner {
                                                                lag,
                                                                is_latency: true,
                                                            },
                                                        );
                                                        retries = 1;
                                                    } else {
                                                        match MonitorReplicationLag::replication_lag_check(
                                                            user.clone(),
                                                            password.clone(),
                                                            read.addr.clone(),
                                                        )
                                                        .await
                                                        {
                                                            Ok(lag) => {
                                                                match lag {
                                                                    Some(lag) => {
                                                                        if lag > max_replication_lag {
                                                                            retries += 1;
                                                                        }
                                                                    }
                                                                    None => {
                                                                        retries += 1;
                                                                    }
                                                                }
                                                            },
                                                            Err(_) => {
                                                                retries += 1;
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                response.latency.insert(
                                                    read.addr,
                                                    ReplicationLagResponseInner { lag, is_latency: false },
                                                );
                                            }
                                        }
                                        None => loop {
                                            if retries > replication_lag_max_failures {
                                                response.latency.insert(
                                                    read.addr.clone(),
                                                    ReplicationLagResponseInner {
                                                        lag: 0,
                                                        is_latency: true,
                                                    },
                                                );
                                                retries = 1;
                                            } else {
                                                match MonitorReplicationLag::replication_lag_check(
                                                    user.clone(),
                                                    password.clone(),
                                                    read.addr.clone(),
                                                )
                                                .await
                                                {
                                                   Ok(lag) => {
                                                       match lag {
                                                            Some(lag) => {
                                                                if lag > max_replication_lag {
                                                                    retries += 1;
                                                                }
                                                            }
                                                            None => {
                                                                retries += 1;
                                                            }
                                                       }
                                                   },
                                                   Err(_) => {
                                                       retries += 1;
                                                   }
                                                }
                                            }
                                        },
                                    }
                                }
                                Err(e) => {

                                }
                            }
                        }
                    })
                    .await
                {
                    // TODO: add timeout handler
                    // start connect max failures retry
                    // if retries > replication_lag_max_failures {
                    //     // after connect_max_failures retrying time send message to Monitor Reconcile
                    //     retries = 1;
                    // }
                    // retries += 1;
                }
                replication_lag_tx.send(response.clone()).unwrap();
                std::thread::sleep(time::Duration::from_millis(reaplication_lag_interval));
            }
        });
    }
}
