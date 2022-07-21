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

use std::{collections::HashMap, sync::Arc};

use conn_pool::Pool;
// use tokio_stream::StreamExt;
use futures::StreamExt;
use mysql_protocol::{client::conn::ClientConn, util::*};
use tokio::time::{self, Duration};

use crate::{config::MasterHighAvailability, readwritesplitting::ReadWriteEndpoint};

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
    pool: Pool<ClientConn>,
    pub monitors: Vec<MonitorKind>,
}

#[async_trait::async_trait]
impl Discovery for DiscoveryMasterHighAvailability {
    type Output = Self;

    fn build(config: MasterHighAvailability, rw_endpoint: ReadWriteEndpoint) -> Self::Output {
        // build conn pool for monitor
        let pool = Pool::<ClientConn>::new(config.pool_size.unwrap_or(64).into());
        Self { config, rw_endpoint, pool, monitors: vec![] }
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
            self.pool.clone(),
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
            self.pool.clone(),
        )));
        monitors.push(MonitorKind::ReadOnly(MonitorReadOnly::new(
            self.config.user.clone(),
            self.config.password.clone(),
            self.config.read_only_interval,
            self.config.read_only_timeout,
            self.config.read_only_max_failures,
            monitor_channel.read_only_tx,
            self.rw_endpoint.clone(),
            self.pool.clone(),
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
    // fn report(&self);
    async fn run_check(&self);
}

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

// define Connect Monitor
impl MonitorConnect {
    fn new(
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

        let mut response =
            ConnectMonitorResponse { read: HashMap::new(), readwrite: HashMap::new() };

        tokio::spawn(async move {
            let mut retries = 1;
            loop {
                // maybe connection will timeout
                if let Err(_) = time::timeout(Duration::from_millis(connect_timeout), async {
                    // probe read endpoint
                    for read in rw_endpoint.clone().read {
                        let conn_res = MonitorConnect::connnect_check(read.addr.clone()).await;
                        match conn_res {
                            ConnectStatus::Connected => {}
                            ConnectStatus::Disconnected => {
                                // connect failures retry
                                loop {
                                    if retries > connect_max_failures {
                                        response.read.insert(read.addr.clone(), conn_res.clone());
                                        retries = 0;
                                        break;
                                    } else {
                                        match MonitorConnect::connnect_check(read.addr.clone())
                                            .await
                                        {
                                            ConnectStatus::Disconnected => retries += 1,
                                            ConnectStatus::Connected => break,
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // probe readwrite endpoint
                    for readwrite in rw_endpoint.clone().readwrite {
                        let conn_res = MonitorConnect::connnect_check(readwrite.addr.clone()).await;
                        match conn_res {
                            ConnectStatus::Connected => {}
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
                                        ConnectStatus::Connected => break,
                                    }
                                }
                            },
                        }
                    }
                })
                .await
                {
                    // start connect max failures retry
                    if retries > connect_max_failures {
                        // after connect_max_failures retrying time send message to Monitor Reconcile
                        connect_tx.send(response.clone()).unwrap();
                        retries = 1;
                    }
                    retries += 1;
                }

                // println!("check.....");
                // connect_tx.send(response.clone()).unwrap();

                // connect monitor probe interval
                std::thread::sleep(std::time::Duration::from_millis(connect_interval));
            }
        });
    }
}

#[derive(Debug)]
pub struct MonitorPing {
    pub user: String,
    pub password: String,
    pub ping_interval: u64,
    pub ping_timeout: u64,
    pub ping_max_failures: u64,
    pub ping_tx: crossbeam_channel::Sender<PingMonitorResponse>,
    pub rw_endpoint: ReadWriteEndpoint,
    pool: Pool<ClientConn>,
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

impl MonitorPing {
    fn new(
        user: String,
        password: String,
        ping_interval: u64,
        ping_timeout: u64,
        ping_max_failures: u64,
        ping_tx: crossbeam_channel::Sender<PingMonitorResponse>,
        rw_endpoint: ReadWriteEndpoint,
        pool: Pool<ClientConn>,
    ) -> Self {
        MonitorPing {
            user,
            password,
            ping_interval,
            ping_timeout,
            ping_max_failures,
            ping_tx,
            rw_endpoint,
            pool,
        }
    }

    async fn ping_check(
        user: String,
        password: String,
        addr: String,
        mut pool: Pool<ClientConn>,
    ) -> PingStatus {
        let factory = ClientConn::with_opts(user, password, addr.clone());
        pool.set_factory(factory);
        let mut client_conn = pool.get_conn_with_endpoint(&addr).await.unwrap();
        match client_conn.send_ping().await {
            Ok(data) => {
                if is_ok(&data.0) {
                    PingStatus::PingOk
                } else {
                    PingStatus::PingNotOk
                }
            }
            Err(_) => PingStatus::PingNotOk,
        }
    }
}

#[async_trait::async_trait]
impl Monitor for MonitorPing {
    async fn run_check(&self) {
        let user = self.user.clone();
        let password = self.password.clone();
        let ping_interval = self.ping_interval;
        let ping_timeout = self.ping_timeout;
        let ping_max_failures = self.ping_max_failures;
        let rw_endpoint = self.rw_endpoint.clone();
        let ping_tx = self.ping_tx.clone();
        let pool = self.pool.clone();

        let mut response = PingMonitorResponse { read: HashMap::new(), readwrite: HashMap::new() };
        tokio::spawn(async move {
            let mut retries = 1;
            loop {
                if let Err(_) = time::timeout(Duration::from_millis(ping_timeout), async {
                    for read in rw_endpoint.clone().read {
                        match MonitorPing::ping_check(
                            user.clone(),
                            password.clone(),
                            read.addr.clone(),
                            pool.clone(),
                        )
                        .await
                        {
                            PingStatus::PingOk => {
                                response.read.insert(read.addr, PingStatus::PingOk);
                            }
                            PingStatus::PingNotOk => loop {
                                if retries > ping_max_failures {
                                    response.read.insert(read.addr.clone(), PingStatus::PingNotOk);
                                } else {
                                    match MonitorPing::ping_check(
                                        user.clone(),
                                        password.clone(),
                                        read.addr.clone(),
                                        pool.clone(),
                                    )
                                    .await
                                    {
                                        PingStatus::PingOk => {
                                            response
                                                .read
                                                .insert(read.addr.clone(), PingStatus::PingOk);
                                            break;
                                        }
                                        PingStatus::PingNotOk => {
                                            retries += 1;
                                        }
                                    }
                                }
                            },
                        }
                    }

                    for readwrite in rw_endpoint.clone().readwrite {
                        match MonitorPing::ping_check(
                            user.clone(),
                            password.clone(),
                            readwrite.addr.clone(),
                            pool.clone(),
                        )
                        .await
                        {
                            PingStatus::PingOk => {
                                response.read.insert(readwrite.addr, PingStatus::PingOk);
                            }
                            PingStatus::PingNotOk => loop {
                                if retries > ping_max_failures {
                                    response
                                        .read
                                        .insert(readwrite.addr.clone(), PingStatus::PingNotOk);
                                } else {
                                    match MonitorPing::ping_check(
                                        user.clone(),
                                        password.clone(),
                                        readwrite.addr.clone(),
                                        pool.clone(),
                                    )
                                    .await
                                    {
                                        PingStatus::PingOk => {
                                            response
                                                .read
                                                .insert(readwrite.addr.clone(), PingStatus::PingOk);
                                            break;
                                        }
                                        PingStatus::PingNotOk => {
                                            retries += 1;
                                        }
                                    }
                                }
                            },
                        }
                    }
                })
                .await
                {
                    if retries > ping_max_failures {
                        // after ping_max_failures retrying time send message to Monitor Reconcile
                        ping_tx.send(response.clone()).unwrap();
                        retries = 1;
                    }
                    retries += 1;
                }
            }
        });
    }
}

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
    pool: Pool<ClientConn>,
}

impl MonitorReplicationLag {
    fn new(
        user: String,
        password: String,
        replication_lag_interval: u64,
        replication_lag_timeout: u64,
        replication_lag_max_failures: u64,
        max_replication_lag: u64,
        replication_lag_tx: crossbeam_channel::Sender<ReplicationLagMonitorResponse>,
        rw_endpoint: ReadWriteEndpoint,
        pool: Pool<ClientConn>,
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
            pool,
        }
    }

    async fn replication_lag_check(
        user: String,
        password: String,
        addr: String,
        mut pool: Pool<ClientConn>,
    ) -> Option<u64> {
        let mut reasponse_replication_lag: Option<u64> = None;

        let factory = ClientConn::with_opts(user, password, addr.clone());
        pool.set_factory(factory);
        let mut client_conn = pool.get_conn_with_endpoint(&addr).await.unwrap();
        let mut res =
            client_conn.query_result("show slave status".as_bytes()).await.unwrap().unwrap();

        while let Some(mut data) = res.1.next().await {
            if is_eof(&data.as_ref().unwrap()) {
                break;
            }
            let mut row =
                mysql_protocol::row::RowData::new(res.0.clone(), &data.as_mut().unwrap()[4..]);
            let seconds_behind_master =
                row.decode_with_name::<Option<u64>>("Seconds_Behind_Master").unwrap();
            match seconds_behind_master {
                Some(lag) => reasponse_replication_lag = Some(lag),
                None => {}
            }
        }
        println!("replication lag >>> {:#?}", reasponse_replication_lag);
        reasponse_replication_lag
    }
}

#[derive(Debug, Clone)]
pub struct ReplicationLagResponseInner {
    lag: u64,
    is_latency: bool,
}

#[derive(Debug, Clone)]
pub struct ReplicationLagMonitorResponse {
    // define slave late from master
    latency: HashMap<String, ReplicationLagResponseInner>,
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
        let pool = self.pool.clone();
        let mut response = ReplicationLagMonitorResponse { latency: HashMap::new() };

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
                                pool.clone(),
                            )
                            .await
                            {
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
                                                    pool.clone(),
                                                )
                                                .await
                                                {
                                                    Some(lag) => {
                                                        if lag > max_replication_lag {
                                                            retries += 1;
                                                        }
                                                    }
                                                    None => {
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
                                            pool.clone(),
                                        )
                                        .await
                                        {
                                            Some(lag) => {
                                                if lag > max_replication_lag {
                                                    retries += 1;
                                                }
                                            }
                                            None => {
                                                retries += 1;
                                            }
                                        }
                                    }
                                },
                            }
                        }
                    })
                    .await
                {
                    // start connect max failures retry
                    if retries > replication_lag_max_failures {
                        // after connect_max_failures retrying time send message to Monitor Reconcile
                        replication_lag_tx.send(response.clone()).unwrap();
                        retries = 1;
                    }
                    retries += 1;
                }

                std::thread::sleep(time::Duration::from_millis(reaplication_lag_interval));
            }
        });
    }
}

#[derive(Debug)]
pub struct MonitorReadOnly {
    pub user: String,
    pub password: String,
    pub read_only_interval: u64,
    pub read_only_timeout: u64,
    pub read_only_max_failures: u64,
    pub read_only_tx: crossbeam_channel::Sender<ReadOnlyMonitorResponse>,
    pub rw_endpoint: ReadWriteEndpoint,
    pool: Pool<ClientConn>,
}

#[derive(Debug, Clone)]
pub struct ReadOnlyMonitorResponse {
    read: HashMap<String, String>,
    readwrite: HashMap<String, String>,
}

impl MonitorReadOnly {
    fn new(
        user: String,
        password: String,
        read_only_interval: u64,
        read_only_timeout: u64,
        read_only_max_failures: u64,
        read_only_tx: crossbeam_channel::Sender<ReadOnlyMonitorResponse>,
        rw_endpoint: ReadWriteEndpoint,
        pool: Pool<ClientConn>,
    ) -> Self {
        MonitorReadOnly {
            user,
            password,
            read_only_interval,
            read_only_timeout,
            read_only_max_failures,
            read_only_tx,
            rw_endpoint,
            pool,
        }
    }

    // show variables like 'read_only';
    async fn read_only_check(
        user: String,
        password: String,
        addr: String,
        mut pool: Pool<ClientConn>,
    ) -> Option<String> {
        let mut res_read_only_status: Option<String> = None;

        let factory = ClientConn::with_opts(user, password, addr.clone());

        pool.set_factory(factory);

        let mut client_conn = pool.get_conn_with_endpoint(&addr).await.unwrap();
        let mut res = client_conn
            .query_result("SHOW VARIABLES LIKE 'read_only'".as_bytes())
            .await
            .unwrap()
            .unwrap();
        while let Some(mut data) = res.1.next().await {
            if is_eof(&data.as_ref().unwrap()) {
                break;
            }
            let mut row =
                mysql_protocol::row::RowData::new(res.0.clone(), &data.as_mut().unwrap()[4..]);
            let read_only_status = row.decode_with_name::<String>("Variable_name").unwrap();
            let read_only_values = row.decode_with_name::<String>("Value").unwrap();

            if read_only_status.eq("read_only") {
                res_read_only_status = Some(read_only_values);
            }
        }
        res_read_only_status
    }
}

#[async_trait::async_trait]
impl Monitor for MonitorReadOnly {
    async fn run_check(&self) {
        let user = self.user.clone();
        let password = self.password.clone();
        let pool = self.pool.clone();
        let rw_endpoint = self.rw_endpoint.clone();
        let read_only_interval = self.read_only_interval.clone();
        let read_only_timeout = self.read_only_timeout;
        let read_only_max_failures = self.read_only_max_failures;
        let read_only_tx = self.read_only_tx.clone();

        let mut response =
            ReadOnlyMonitorResponse { read: HashMap::new(), readwrite: HashMap::new() };

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
                            pool.clone(),
                        )
                        .await
                        {
                            Some(read_only_status) => {
                                response.read.insert(read.addr, read_only_status).unwrap();
                            }
                            None => {
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
                            pool.clone(),
                        )
                        .await
                        {
                            Some(read_only_status) => {
                                response
                                    .readwrite
                                    .insert(readwrite.addr, read_only_status)
                                    .unwrap();
                            }
                            None => {
                                continue;
                            }
                        }
                    }
                })
                .await
                {
                    if retries > read_only_max_failures {
                        read_only_tx.send(response.clone()).unwrap();
                        retries = 1;
                    }
                    retries += 1;
                }

                std::thread::sleep(time::Duration::from_millis(read_only_interval));
            }
        });
    }
}
