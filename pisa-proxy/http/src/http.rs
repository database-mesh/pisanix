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

use config::config::PisaConfig;
use pisa_error::error::*;
use pisa_metrics::metrics::MetricsManager;

use crate::controllers::{healthz::healthz, version::version};

#[async_trait::async_trait]
pub trait HttpServer {
    async fn start(&mut self) -> Result<(), Error>;
}

pub struct RocketServer {
    pisa_config: PisaConfig,
    metrics_manager: MetricsManager,
}

pub fn new_rocket_server(pisa_config: PisaConfig, metrics_manager: MetricsManager) -> RocketServer {
    RocketServer { pisa_config, metrics_manager }
}

#[async_trait::async_trait]
impl HttpServer for RocketServer {
    async fn start(&mut self) -> Result<(), Error> {
        self.metrics_manager.register();
        let figment = rocket::Config::figment()
            .merge(("address", &self.pisa_config.get_admin().host))
            .merge(("port", &self.pisa_config.get_admin().port))
            .merge(("cli_colors", false))
            .merge(("shutdown.ctrlc", false));

        return rocket::Rocket::custom(figment)
            .attach(self.metrics_manager.get_server())
            .mount("/", routes![healthz, version])
            .mount("/metrics", self.metrics_manager.get_server())
            .launch()
            .await
            .map_err(|e| Error::new(ErrorKind::Rocket(Box::new(e))));
    }
}

pub async fn bg_task<T: HttpServer>(mut s: T) {
    s.start().await.unwrap();
}
