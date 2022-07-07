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

// LABEL_NAME_DOMAIN refers to the name of current working proxy runtime
const LABEL_NAME_DOMAIN: &'static str = "domain";
// LABEL_NAME_TYPE refers to the type of current working command type
const LABEL_NAME_TYPE: &'static str = "type";
// LABEL_NAME_SERVER refers to the host of current backend database
const LABEL_NAME_SERVER: &'static str = "server";

pub static SQL_PROCESSED_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    IntCounterVec::new(
        opts!("sql_processed_total", "The total of processed SQL"),
        &[LABEL_NAME_DOMAIN, LABEL_NAME_TYPE, LABEL_NAME_SERVER],
    )
    .expect("Could not create SQL_PROCESSED_TOTAL")
});

pub static SQL_PROCESSED_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    let opt = HistogramOpts {
        common_opts: opts!("sql_processed_duration", "The duration of processed SQL"),
        buckets: Vec::<f64>::new(),
    };
    HistogramVec::new(opt, &[LABEL_NAME_DOMAIN, LABEL_NAME_TYPE, LABEL_NAME_SERVER])
        .expect("Cound not create SQL_PROCESSED_DURATION")
});

pub static SQL_UNDER_PROCESSING: Lazy<GaugeVec> = Lazy::new(|| {
    GaugeVec::new(
        opts!("sql_under_processing", "The active SQL under processing"),
        &[LABEL_NAME_DOMAIN, LABEL_NAME_TYPE, LABEL_NAME_SERVER],
    )
    .expect("Cound not create SQL_UNDER_PROCESSING")
});

#[derive(Clone, Copy)]
pub struct MySQLServerMetricsCollector;

impl MySQLServerMetricsCollector {
    pub fn new() -> Self {
        MySQLServerMetricsCollector {}
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

macro_rules! collect_sql_processed_total {
    ($s:expr, $x:expr, $c:expr) => {
        $s.metrics_collector.set_sql_processed_total(&[$s.name.as_str(), $x, $c]);
    };
}

macro_rules! collect_sql_processed_duration {
    ($s:expr, $x:expr, $c:expr, $e:expr) => {
        let now = SystemTime::now();
        let duration = now.duration_since($e).unwrap();
        $s.metrics_collector
            .set_sql_processed_duration(&[$s.name.as_str(), $x, $c], duration.as_secs_f64());
    };
}

macro_rules! collect_sql_under_processing_inc {
    ($s:expr, $x:expr, $c:expr) => {
        $s.metrics_collector.set_sql_under_processing_inc(&[$s.name.as_str(), $x, $c]);
    };
}

macro_rules! collect_sql_under_processing_dec {
    ($s:expr, $x:expr, $c:expr) => {
        $s.metrics_collector.set_sql_under_processing_dec(&[$s.name.as_str(), $x, $c]);
    };
}
