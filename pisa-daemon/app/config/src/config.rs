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

use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub global: Global,
    pub app: Vec<App>
}

#[derive(Debug, Deserialize, Default)]
pub struct Global {
   pub egress_device: String, 
   pub bridge_device: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct App {
    pub name: String,
    pub service: Vec<Service>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Service {
    pub name: String,
    pub endpoints: Vec<Endpoint>,
    pub qos_class: Option<QosClass>,
    pub qos_group: Option<QosGroup>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Endpoint {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum QosClass {
    Guaranteed,
    Burstable,
    Besteffort,
}

impl Default for QosClass {
    fn default() -> Self {
        Self::Besteffort
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct QosGroup {
    pub rate: Option<String>,
    pub ceil: Option<String>,
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_str_to_config() {
        let toml_str = r#"
        [global]
egress_device = "eth0"
bridge_device = "cni0"

[[app]]
name = "testapp"

[[app.service]]
name = "test"
qos_class = "guaranteed" # "burstable" | "besteffort"
[[app.service.endpoints]]
ip = "1.1.1.1"
port = 3306

[[app.service.endpoints]]
ip = "1.1.1.1"
port = 3307

[app.service.qos_group]
rate = "1MB"
ceil = "1MB"

"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        println!("{:?}", config) ;
        assert_eq!(config.app[0].name, "testapp");
        assert_eq!(config.app[0].service[0].qos_class, Some(QosClass::Guaranteed));
    }
}
