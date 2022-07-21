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

pub fn check_tc_exist() -> bool {
    match Command::new("tc").spawn() {
        Ok(_) =>  true,
        Err(e) => {
            println!("check tc command err: {:?}", e);
            false
        }
    }
}


#[macro_use]
mod macros;
pub mod tc;
