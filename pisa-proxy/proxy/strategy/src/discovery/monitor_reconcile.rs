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

use std::sync::Arc;
use tokio::sync::Mutex;
use std::{thread, time};

use crossbeam_channel::unbounded;
use endpoint::endpoint::Endpoint;

use crate::{config::ReadWriteSplittingDynamic, readwritesplitting::ReadWriteEndpoint};

pub struct MonitorReconcile {
    config: ReadWriteSplittingDynamic,

    monitors: Vec<String>,
}

async fn report(s: crossbeam_channel::Sender<ReadWriteEndpoint>) {
    tokio::task::spawn_blocking(move || loop {
        let ten_millis = time::Duration::from_millis(1000);

        thread::sleep(ten_millis);
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
    });
}

use crate::readwritesplitting::rule_match::{RulesMatch,RulesMatchBuilder};

impl MonitorReconcile {
    pub fn new(config: ReadWriteSplittingDynamic) -> Self {
        match config.discovery {
            crate::config::Discovery::Mha(config) => {
                DiscoveryKind::MasterHighAvailability(DiscoveryMasterHighAvailability::new(&config))
            }
        };

        MonitorReconcile {config, monitors: vec![] }
    }

    pub fn start_monitor_reconcile(&mut self) {// rules_match: Arc<Mutex<RulesMatch>>) {

        let (send, recv) = unbounded();
        let s = send.clone();
        let r = recv.clone();


        tokio::spawn(async move {
            RulesMatch::change(r).await;
            // self.rules_match.start_rules_match_reconcile(r, self.config.clone().rules, self.config.clone().default_target).await;
        });

        tokio::spawn(async move {
            report(s).await;
        });
    }

    fn register_monitor(&mut self) {}
}
