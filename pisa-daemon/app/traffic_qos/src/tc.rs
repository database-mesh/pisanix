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

use config::{Config, Service, QosGroup};
use tc_command::tc::{QdiscRootAttr, add_root_qdisc, ClassAttr, add_class};

const DEFAULT_CLASSIFIER: &str = "htb";

pub struct TrafficQos(Config);

impl TrafficQos {
    pub fn new(mut config: Config) -> Self {
        sort_app_serivce(&mut config);
        TrafficQos(config)
    }

    pub fn add_root_qdisc(&self) -> bool {
        let attr = QdiscRootAttr {
            netns: None,
            device: &self.0.global.bridge_device,
            typ: DEFAULT_CLASSIFIER,
        };

        if !add_root_qdisc(&attr) {
            return false;
        };

        let attr = QdiscRootAttr {
            netns: None,
            device: &self.0.global.egress_device,
            typ: DEFAULT_CLASSIFIER,
        };

        add_root_qdisc(&attr)
    }

    pub fn add_class(&self) -> bool {
       for (app_idx, app) in self.0.app.iter().enumerate() {
            for (svc_idx, svc) in app.service.iter().enumerate() {
               if !self.add_class_svc(svc, (app_idx + 1) as u8, (svc_idx + 1) as u8) {
                   return false
               }
            }
        }

        true
    }

    fn add_class_svc(&self, svc: &Service, app_idx: u8, svc_idx: u8) -> bool {
        match &svc.qos_group {
            Some(qos) => {
                if qos.rate.is_none() {
                    return true;
                }

                let class_id = format!("{}:{}", 1, self.make_class_id(app_idx, svc_idx));
                let attr = self.make_class_attr(&self.0.global.bridge_device, qos, &class_id);
                if !add_class(&attr) {
                    return false;
                }

                let attr = self.make_class_attr(&self.0.global.egress_device, qos, &class_id);
                add_class(&attr)

            }
            None => true,
        }
    }

    fn make_class_attr<'a>(&'a self, device:&'a str, qos: &'a QosGroup, class_id: &'a str) -> ClassAttr<'a> {
        ClassAttr {
            netns: None,
            device,
            parent: None,
            class_id,
            rate: &qos.rate.as_ref().unwrap(),
            ceil: qos.ceil.as_ref().map(|x| &**x),
        }

    }

    fn make_class_id(&self, app_idx: u8, svc_idx: u8) -> String {
        let id = (app_idx as u16) << 8 | svc_idx as u16;
        format!("{:#06x}", id)
    }
}

// Sort by app and service name
fn sort_app_serivce(config: &mut Config) {
    for v in config.app.iter_mut() {
        // Sort service
        v.service.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    }

    config.app.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
}

#[cfg(test)]
mod test {
    use config::Config;
    use tc_command::tc::delete_root_qdisc;

    use crate::tc::{TrafficQos, sort_app_serivce};

    const config_str: &str = r#"
        [global]
egress_device = "lo"
bridge_device = "docker0"

[[app]]
name = "test1"

[[app.service]]
name = "svc"
qos_class = "guaranteed" # "burstable" | "besteffort"

[app.service.qos_group]
rate = "1mbps"
ceil = "1mbps"

[[app]]
name = "app1"

[[app.service]]
name = "test"
qos_class = "guaranteed" # "burstable" | "besteffort"

[app.service.qos_group]
rate = "2mbps"
ceil = "2mbps"

"#;
    #[test]
    fn test_sort_app_service() {
        let mut config: Config = toml::from_str(config_str).unwrap();
        sort_app_serivce(&mut config);
        assert_eq!(config.app[0].name, "app1");
    }

    #[test]
    fn test_add_qdsic_class() {
        let config: Config = toml::from_str(config_str).unwrap();

        delete_root_qdisc(&config.global.bridge_device, "htb");
        delete_root_qdisc(&config.global.egress_device, "htb");

        let tq = TrafficQos::new(config);
        let res = tq.add_root_qdisc();
        assert_eq!(res, true);
        let res = tq.add_class();
        assert_eq!(res, true);
    }
}

