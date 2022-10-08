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

use std::process::Command;

#[derive(Clone)]
pub struct QdiscRootAttr<'a> {
    pub netns: Option<&'a str>,
    pub device: &'a str,
    pub typ: &'a str,
}

pub fn add_root_qdisc<'a>(attr: &QdiscRootAttr<'a>) -> bool {
    let mut args = vec![
        "qdisc",
        "add",
        "dev",
        &attr.device,
        "root",
        "handle",
        "1:",
        &attr.typ,
    ];

    execute_tc_command!( attr, args, "add root qdisc")

}

pub fn delete_root_qdisc<'a>(device: &str) -> bool {
    let args = vec![
        "qdisc",
        "delete",
        "dev",
        device,
        "root",
        "handle",
        "1:",
    ];

    let out = Command::new("tc")
        .args(&args)
        .output()
        .expect("faild to delete root qdisc");


    if !out.status.success() {
        println!("Failed to delete root qdisc to device {}: {:?}", device, out);
        return false;
    }

    true
}

pub fn show_qdisc(device: String) -> bool {
    let out = Command::new("tc")
        .args(["qdisc", "show", "dev", &device])
        .output()
        .expect("faild to show qdisc");

    if !out.status.success() {
        println!("Failed to show qdisc to device {}: {:?}", device, out);

        return false;
    }

    true
}

pub struct ClassAttr<'a> {
    pub netns: Option<&'a str>,
    pub device: &'a str,
    pub parent: Option<&'a str>,
    pub class_id: &'a str,
    pub rate: &'a str,
    pub ceil: Option<&'a str>,
}

pub fn add_class<'a>(attr: &ClassAttr<'a>) -> bool {
    let mut args = vec![
        "class",
        "add",
        "dev",
        &attr.device,
        "parent",
        &attr.parent.unwrap_or("1:"),
        "classid",
        &attr.class_id,
        "htb",
        "rate",
        &attr.rate,
    ];

    if let Some(ceil) = &attr.ceil {
        args.push("ceil");
        args.push(ceil);
    }


    execute_tc_command!( attr, args, "add class")

}

pub fn delete_class<'a>(attr: &ClassAttr<'a>) -> bool {
    let mut args = vec![
        "class",
        "delete",
        "dev",
        &attr.device,
        "parent",
        "1:",
        "classid",
        &attr.class_id,
        "htb",
        "rate",
        &attr.rate,
    ];


    if let Some(ceil) = &attr.ceil {
        args.push("ceil");
        args.push(ceil);
    }


    execute_tc_command!( attr, args, "delete class")

}

pub fn show_class(netns: Option<&str>, device: &str) -> (Vec<ClassResult>, bool) {
    let mut args = vec!["class", "show", "dev", &device];

    if let Some(ns) = netns {
        args.insert(0, "-n");
        args.insert(1, ns);
    }
    
    let out = Command::new("tc")
        .args(&args)
        .output()
        .expect("faild to show class");

    if !out.status.success() {
        println!("Failed to show class to device {}: {:?}", device, out);

        return (vec![], false);
    }

   let lines  = std::str::from_utf8(&out.stdout).unwrap().lines();

   let mut res = vec![];
   for line in lines {
        let arr = line.split(" ").map(|x| x.to_string()).collect::<Vec<_>>();
       res.push(ClassResult::from(arr));

   }

    (res, true)
}

#[derive(Debug)]
pub enum ClassResult {
    Htb(Htb)
}

#[derive(Debug)]
pub struct Htb {
    pub parent: String,
    pub class_id: String,
    pub rate: String,
    pub ceil: String,
}

impl  From<&[String]> for Htb {
    fn from(input: &[String]) -> Self {
        let parent = if &input[1] == "root" {
            "root"
        } else {
           &input[2]
        };

        let rate_idx = input.iter().position(|x| x == "rate").unwrap();
        let ceil_idx = input.iter().position(|x| x == "ceil").unwrap();

        Htb { 
            parent: parent.to_string(), 
            class_id: input[0].clone(), 
            rate: input[rate_idx+1].clone(), 
            ceil: input[ceil_idx+1].clone(),
        }
    }
}

impl From<Vec<String>> for ClassResult {
    fn from(input: Vec<String>) -> ClassResult {
        match input[1].as_str() {
           "htb" => {
                ClassResult::Htb(Htb::from(&input[2..]))
           },
           _ => unreachable!()
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    
    fn create_test_ns(ns: &str) {
        let out = Command::new("ip")
            .args(["netns", "add", ns])
            .output()
            .unwrap();
        println!("creat test ns {:?}", out);
    }

    fn delete_test_ns(ns: &str) {
        let out = Command::new("ip")
            .args(["netns", "delete", ns])
            .output()
            .unwrap();
        println!("delete test ns {:?}", out);
    }


    fn create_root_qdisc(ns: &str) -> bool {
        let attr = QdiscRootAttr {
            netns: Some(ns),
            device: "lo",
            typ: "htb",
        };

        add_root_qdisc(&attr)
    }

    #[test]
    fn test_create_root_qdisc() {
        create_test_ns("footest1");
        assert_eq!(create_root_qdisc("footest1"), true);
        delete_test_ns("footest1");
    }

    #[test]
    fn test_create_delete_class() {
        create_test_ns("footest2");

        create_root_qdisc("footest2");

        let attr = ClassAttr {
            netns: Some("footest2"),
            device: "lo",
            class_id: "1:10",
            rate: "1mbps",
            ceil: None,
            parent: None,
        };

        assert_eq!(add_class(&attr), true);
        assert_eq!(delete_class(&attr), true);

        delete_test_ns("footest2");
    }

    #[test]
    fn test_show_class() {
        create_test_ns("footest");
        create_root_qdisc("footest");

        let attr = ClassAttr {
            netns: Some("footest"),
            device: "lo",
            class_id: "1:10",
            rate: "1mbps",
            ceil: None,
            parent: Some("1:1"),
        };

        add_class(&attr);

        let (res, _ ) = show_class(Some("footest"), "lo");

        let ClassResult::Htb(htb) = &res[0];
        {
            assert_eq!(htb.class_id, "1:10");
        }
        
        delete_test_ns("footest");
    }
}
