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

use once_cell::sync::Lazy;
use rocket_prometheus::prometheus::{opts, GaugeVec, HistogramOpts, HistogramVec, IntCounterVec};

pub static SQL_PROCESSED_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        opts!("sql_processed_total", "The total of processed SQL"),
        &["domain", "type", "server"],
    )
    .expect("Could not create SQL_PROCESSED_TOTAL")
});

pub static SQL_PROCESSED_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    let opt = HistogramOpts {
        common_opts: opts!("sql_processed_duration", "The duration of processed SQL"),
        buckets: Vec::<f64>::new(),
    };
    HistogramVec::new(opt, &["domain", "type", "server"])
        .expect("Cound not create SQL_PROCESSED_DURATION")
});

pub static SQL_UNDER_PROCESSING: Lazy<GaugeVec> = Lazy::new(|| {
    GaugeVec::new(
        opts!("sql_under_processing", "The active SQL under processing"),
        &["domain", "type", "server"],
    )
    .expect("Cound not create SQL_UNDER_PROCESSING")
});

#[derive(Clone, Copy)]
pub struct MySqlServerMetricsCollector;

impl MySqlServerMetricsCollector {
    pub fn new() -> Self {
        MySqlServerMetricsCollector {}
    }
    pub fn set_sql_processed_total(&self, labels: &[&str]) {
        SQL_PROCESSED_TOTAL.with_label_values(labels).inc();
    }

    pub fn set_sql_processed_duration(&self, labels: &[&str], duration: f64) {
        SQL_PROCESSED_DURATION.with_label_values(labels).observe(duration);
    }

    pub fn set_sql_under_processing_inc(&self, labels: &[&str]) {
        SQL_UNDER_PROCESSING.with_label_values(labels).inc();
    }

    pub fn set_sql_under_processing_dec(&self, labels: &[&str]) {
        SQL_UNDER_PROCESSING.with_label_values(labels).dec();
    }
}