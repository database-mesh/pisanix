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

#[macro_export]
macro_rules! execute_tc_command {
    ($attr:expr, $args:expr, $command_type:tt) => {
        {
            if let Some(netns) = &$attr.netns {
                $args.insert(0, "-n");
                $args.insert(1, netns);
            }

            let out = Command::new("tc")
                .args(&$args)
                .output()
                .expect("failed to add class");

            if !out.status.success() {
                println!("Failed to {} to device {}: {:?}", $command_type, $attr.device, out);
                return false;
            }

            true
        }
    }
}

