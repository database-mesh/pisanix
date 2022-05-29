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

#![feature(decl_macro)]

use config::config::PisaConfig;
use pisa_error::error::*;
use pisa_metrics::metrics::PisaMetrics;

use crate::controllers::{healthz::healthz, version::version};

#[async_trait::async_trait]
pub trait HttpServer {
    async fn start(&mut self) -> Result<(), Error>;
}

pub struct RocketServer {
    pisa_config: PisaConfig,
}

pub fn new_rocket_server(pisa_config: PisaConfig) -> RocketServer {
    RocketServer { pisa_config }
}

impl RocketServer {
    fn new(pisa_config: PisaConfig) -> RocketServer {
        RocketServer { pisa_config }
    }
}

#[async_trait::async_trait]
impl HttpServer for RocketServer {
    async fn start(&mut self) -> Result<(), Error> {
        let m = PisaMetrics::new();
        m.register();

        let p = &self.pisa_config.get_admin().port.to_string();
        let httpport: i32 = p.parse().unwrap();

        let hostaddr = "0.0.0.0";
        let figment = rocket::Config::figment()
            .merge(("port", httpport))
            .merge(("address", hostaddr))
            .merge(("cli_colors", false))
            .merge(("shutdown.ctrlc", false));

        return rocket::Rocket::custom(figment)
            .attach(m.get_routes())
            .mount("/", routes![healthz, version])
            .mount("/metrics", m.get_routes())
            .launch()
            .await
            .map_err(|e| Error::new(ErrorKind::Rocket(Box::new(e))));
    }
}

pub async fn bg_task<T: HttpServer>(mut s: T) {
    s.start().await.unwrap();
}
