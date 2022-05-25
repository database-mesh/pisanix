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

use pisa_error::error::{Error, ErrorKind};
use proxy::proxy::{MySQLNode, ProxyConfig};

pub struct ShardingProxy {
    pub proxy_config: ProxyConfig,
    pub shardingproxy_nodes: Vec<MySQLNode>,
}

#[async_trait::async_trait]
impl proxy::factory::Proxy for ShardingProxy {
    async fn start(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
