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

use std::{collections::HashMap, path::Path};

use bpf::load::{self, TrafficTyp};
use config::{Endpoint, PisaDaemonConfig, QosGroup, Service};
use tc_command::tc::*;

const DEFAULT_CLASSIFIER: &str = "htb";

pub struct TrafficQos {
    config: PisaDaemonConfig,
    endpoint_classid: Vec<(Endpoint, u32)>,
}

impl TrafficQos {
    pub fn new(mut config: PisaDaemonConfig) -> Self {
        sort_app_serivce(&mut config);
        TrafficQos {
            config,
            endpoint_classid: vec![],
        }
    }

    pub fn add_root_qdisc(&self) -> bool {
        // First step: clear root qdisc
        self.delete_root_qdisc();

        let attr = QdiscRootAttr {
            netns: None,
            device: &self.config.global.bridge_device,
            typ: DEFAULT_CLASSIFIER,
        };

        if !add_root_qdisc(&attr) {
            return false;
        };

        let attr = QdiscRootAttr {
            netns: None,
            device: &self.config.global.egress_device,
            typ: DEFAULT_CLASSIFIER,
        };

        add_root_qdisc(&attr)
    }

    pub fn delete_root_qdisc(&self) {
        delete_root_qdisc(&self.config.global.bridge_device);
        delete_root_qdisc(&self.config.global.egress_device);
    }

    pub fn add_class(&self) -> (Vec<(Endpoint, u32)>, bool) {
        let mut endpoint_classid = vec![];

        for (app_idx, app) in self.config.app.iter().enumerate() {
            for (svc_idx, svc) in app.service.iter().enumerate() {
                let (is_succ, class_id) =
                    self.add_class_svc(svc, (app_idx + 1) as u8, (svc_idx + 1) as u8);
                if !is_succ {
                    return (vec![], false);
                }

                if is_succ && class_id == 0 {
                    return (vec![], true);
                }

                for ep in &svc.endpoints {
                    endpoint_classid.push((ep.clone(), class_id))
                }
            }
        }

        (endpoint_classid, true)
    }

    fn add_class_svc(&self, svc: &Service, app_idx: u8, svc_idx: u8) -> (bool, u32) {
        match &svc.qos_group {
            Some(qos) => {
                if qos.rate.is_none() {
                    return (true, 0);
                }

                let minor_id = self.make_minor_id(app_idx, svc_idx);
                let handle = format!("{}:{:X}", 1, minor_id);
                let class_id =  (1_u32  & 0xFFFF0000) | (minor_id as u32 & 0x0000FFFF);

                let attr = self.make_class_attr(&self.config.global.bridge_device, qos, &handle);
                if !add_class(&attr) {
                    return (false, class_id);
                }

                let attr = self.make_class_attr(&self.config.global.egress_device, qos, &handle);
                (add_class(&attr), class_id)
            }
            None => (true, 0),
        }
    }

    fn make_class_attr<'a>(
        &'a self,
        device: &'a str,
        qos: &'a QosGroup,
        class_id: &'a str,
    ) -> ClassAttr<'a> {
        ClassAttr {
            netns: None,
            device,
            parent: None,
            class_id,
            rate: &qos.rate.as_ref().unwrap(),
            ceil: qos.ceil.as_ref().map(|x| &**x),
        }
    }

    fn make_minor_id(&self, app_idx: u8, svc_idx: u8) -> u16 {
        (app_idx as u16) << 8 | svc_idx as u16
    }

    pub fn load_bpf<P: AsRef<Path>>(
        &self,
        obj_path: P,
        endpoint_classid: Vec<(Endpoint, u32)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let app = TrafficTyp::App;

        add_filter(&FilterAttr {
                netns: None,
                device: self.config.global.bridge_device.as_str(),
                obj: "app.o",
        });
        add_filter(&FilterAttr {
                netns: None,
                device: self.config.global.egress_device.as_str(),
                obj: "app.o",
        });

        // load
        println!("endpoint classid {:?}", endpoint_classid);
        let mut bpf = app.load(
            obj_path,
            &vec![
                self.config.global.bridge_device.as_str(),
                self.config.global.egress_device.as_str(),
            ],
        )?;

        app.load_app_config(&mut bpf, endpoint_classid)?;

        Ok(())
    }
}

// Sort by app and service name
fn sort_app_serivce(config: &mut PisaDaemonConfig) {
    for v in config.app.iter_mut() {
        // Sort service
        v.service.sort_by(|a, b| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        });
    }

    config.app.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    })
}

#[cfg(test)]
mod test {
    use config::PisaDaemonConfig;
    use tc_command::tc::delete_root_qdisc;

    use crate::qos::{sort_app_serivce, TrafficQos};

    const CONFIG_STR: &str = r#"
        [global]
egress_device = "lo"
bridge_device = "docker0"

[[app]]
name = "test1"

[[app.service]]
name = "svc"
qos_class = "guaranteed" # "burstable" | "besteffort"
[[app.service.endpoints]]
ip = "1.1.1.1"
port = 3306

[[app.service.endpoints]]
ip = "1.1.1.1"
port = 3307
[app.service.qos_group]
rate = "1mbps"
ceil = "1mbps"

[[app]]
name = "app1"

[[app.service]]
name = "test"
qos_class = "guaranteed" # "burstable" | "besteffort"
[[app.service.endpoints]]
ip = "2.2.2.2"
port = 3306

[[app.service.endpoints]]
ip = "2.2.2.2"
port = 3307

[app.service.qos_group]
rate = "2mbps"
ceil = "2mbps"
"#;
    #[test]
    fn test_sort_app_service() {
        let mut config: PisaDaemonConfig = toml::from_str(CONFIG_STR).unwrap();
        println!("config {:?}", config);
        sort_app_serivce(&mut config);
        assert_eq!(config.app[0].name, "app1");
    }

    #[test]
    fn test_add_qdsic_class() {
        let config: PisaDaemonConfig = toml::from_str(CONFIG_STR).unwrap();

        delete_root_qdisc(&config.global.bridge_device);
        delete_root_qdisc(&config.global.egress_device);

        let tq = TrafficQos::new(config);
        let res = tq.add_root_qdisc();
        assert_eq!(res, true);
        let res = tq.add_class();
        assert_eq!(res.1, true);
    }
}

