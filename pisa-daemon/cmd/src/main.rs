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

use std::io::{Error, ErrorKind};

use config::PisaDaemonConfigBuilder;
use traffic_qos::TrafficQos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = PisaDaemonConfigBuilder::new().collect_from_cmd().build()?;
    let qos = TrafficQos::new(config);

    // Init root qdisc
    qos.add_root_qdisc();

    // Add class
    let (endpoint_classid, is_succ) = qos.add_class();
    if !is_succ {
        return Err(Box::new(Error::new(ErrorKind::Other, "add class failed")));
    }

    // Load bpf
    qos.load_bpf("./app.o", endpoint_classid)?;

    Ok(())
}