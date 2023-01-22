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

use prometheus::{Registry, Encoder};
use runtime_mysql::server::metrics::*;

const METRICS_NAMESPACE: &str = "pisa_proxy";

#[derive(Clone, Default, Debug)]
pub struct MetricsManager {
    registry: Registry,
}

impl MetricsManager {
    pub fn new() -> Self {
        let registry = Registry::new_custom(Some(METRICS_NAMESPACE.to_string()), None).unwrap();
        Self::register_metrics(&registry);

        MetricsManager { 
            registry 
        }
    }

    pub fn get_server(&self) -> Registry {
        self.registry.clone()
    }

    pub fn register_metrics(registry: &Registry) {
        registry.register(Box::new(SQL_PROCESSED_TOTAL.clone())).unwrap();
        registry.register(Box::new(SQL_PROCESSED_DURATION.clone())).unwrap();
        registry.register(Box::new(SQL_UNDER_PROCESSING.clone())).unwrap();
    }

    pub fn gather(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![];
        let encoder = prometheus::TextEncoder::new();
        encoder.encode(&self.registry.gather(), &mut buf).unwrap();

        buf
    }
}
