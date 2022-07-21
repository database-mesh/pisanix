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

// use std::sync::Arc;
// use tokio::sync::Mutex;
use std::{thread, time};

use crossbeam_channel::unbounded;
use endpoint::endpoint::Endpoint;

use crate::{
    config::ReadWriteSplittingDynamic,
    discovery::discovery::{Discovery, DiscoveryKind, DiscoveryMasterHighAvailability},
    readwritesplitting::ReadWriteEndpoint,
};

pub struct MonitorReconcile {
    config: crate::config::Discovery,
}

impl MonitorReconcile {
    pub fn new(config: ReadWriteSplittingDynamic, rw_endpoint: ReadWriteEndpoint) -> Self {
        MonitorReconcile { config: config.discovery }
    }

    pub fn start_monitor_reconcile(
        &mut self,
        monitor_interval: u64,
        monitor_channel: crate::readwritesplitting::MonitorChannel,
    ) -> crossbeam_channel::Receiver<ReadWriteEndpoint> {
        let (send, recv) = unbounded();
        let tx = send.clone();
        let rx = recv.clone();

        tokio::spawn(async move {
            MonitorReconcile::report(tx, monitor_interval, monitor_channel).await;
        });

        rx
    }

    async fn report(
        s: crossbeam_channel::Sender<ReadWriteEndpoint>,
        monitor_interval: u64,
        monitor_channel: crate::readwritesplitting::MonitorChannel,
    ) {
        tokio::task::spawn_blocking(move || loop {
            //TODO compute from 4 monitors
            loop {
                let connect_monitor_response = monitor_channel.connect_rx.recv().unwrap();
                // println!("connect monitor channel : {:#?}", aa);

                // let send_msg = match connect_monitor_response.read.get("127.0.0.1:3306").unwrap() {
                //     crate::discovery::discovery::ConnectStatus::Disconnected => ReadWriteEndpoint {
                //         read: vec![Endpoint {
                //             weight: 2,
                //             name: String::from("dasheng001"),
                //             db: String::from("test"),
                //             user: String::from("root"),
                //             password: String::from("12345678"),
                //             addr: String::from("127.0.0.1:3308"),
                //         }],
                //         readwrite: vec![Endpoint {
                //             weight: 2,
                //             name: String::from("dasheng002"),
                //             db: String::from("test"),
                //             user: String::from("root"),
                //             password: String::from("12345678"),
                //             addr: String::from("127.0.0.1:3308"),
                //         }],
                //     },
                //     crate::discovery::discovery::ConnectStatus::Connected => ReadWriteEndpoint {
                //         read: vec![Endpoint {
                //             weight: 2,
                //             name: String::from("dasheng001"),
                //             db: String::from("test"),
                //             user: String::from("root"),
                //             password: String::from("12345678"),
                //             addr: String::from("127.0.0.1:3306"),
                //         }],
                //         readwrite: vec![Endpoint {
                //             weight: 2,
                //             name: String::from("dasheng002"),
                //             db: String::from("test"),
                //             user: String::from("root"),
                //             password: String::from("12345678"),
                //             addr: String::from("127.0.0.1:3306"),
                //         }],
                //     },
                // };
                // final data
                let send_msg = ReadWriteEndpoint {
                    read: vec![Endpoint {
                        weight: 2,
                        name: String::from("dasheng001"),
                        db: String::from("test"),
                        user: String::from("root"),
                        password: String::from("12345678"),
                        addr: String::from("127.0.0.1:3306"),
                    }],
                    readwrite: vec![Endpoint {
                        weight: 2,
                        name: String::from("dasheng002"),
                        db: String::from("test"),
                        user: String::from("root"),
                        password: String::from("12345678"),
                        addr: String::from("127.0.0.1:3306"),
                    }],
                };
                if let Err(err) = s.try_send(send_msg) {
                    println!("err >>> {:#?}", err);
                }
                let ten_millis = time::Duration::from_millis(1000);
                thread::sleep(ten_millis);
            }
        });
    }
}
