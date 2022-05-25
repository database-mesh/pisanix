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

use pprof::protos::Message;

#[get("/pprof")]
pub fn pprof() -> &'static str {
    "Hello, Metrics!"
}

async fn dump_cpu_prof(seconds: u64, frequency: i32) -> pprof::Result<pprof::Report> {
    let guard = pprof::ProfilerGuard::new(frequency)?;
    async_std::task::sleep(Duration::from_secs(seconds)).await;
    guard.report().build()
}

async fn profile_cpu_handler(dp: DebugParams) -> Result<impl warp::Reply, warp::Rejection> {
    let seconds = dp.seconds.unwrap_or(10);

    let frequency = dp.frequency.unwrap_or(99);

    let report = match dump_cpu_prof(seconds, frequency).await {
        Ok(report) => report,
        Err(_) => return Err(warp::reject()),
    };

    let mut body: Vec<u8> = Vec::new();
    match report.pprof() {
        Ok(profile) => match profile.encode(&mut body) {
            Ok(()) => {
                Ok(Response::builder().header("Content-Type", "application/protobuf").body(body))
            }
            Err(_) => Err(warp::reject()),
        },
        Err(_) => Err(warp::reject()),
    }
}

async fn dummy_handler() -> Result<impl warp::Reply, warp::Rejection> {
    let mut v = 0;
    for _ in 1..5000000 {
        v += 1;
    }

    Ok(format!("{} loop finished!", v))
}