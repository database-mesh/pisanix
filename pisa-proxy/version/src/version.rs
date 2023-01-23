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

pub fn get_version() -> String {
    let errmgs = "Can not get build informations";
    format!(
        "Pisa-Proxy Release: {}, Pisa Edition: {}, Rustc Version: {}, Rustc CommitDate: {}, Rustc CommitHash: {}, Git Commit: {}, Git Branch: {}",
        env!("CARGO_PKG_VERSION"),
        option_env!("EDITION").unwrap_or("Community"),
        option_env!("VERGEN_RUSTC_SEMVER").unwrap_or(errmgs),
        option_env!("VERGEN_RUSTC_COMMIT_DATE").unwrap_or(errmgs),
        option_env!("VERGEN_RUSTC_COMMIT_HASH").unwrap_or(errmgs),
        option_env!("VERGEN_GIT_SHA").unwrap_or(errmgs),
        option_env!("VERGEN_GIT_BRANCH").unwrap_or(errmgs),
    )
}
