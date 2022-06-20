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

use pisa_error::error::Error;

pub enum ProxyKind {
    MySQL,
    ShardingSphereProxy,
}

#[async_trait::async_trait]
pub trait Proxy {
    async fn start(&mut self) -> Result<(), Error>;
}

pub trait ProxyFactory {
    fn build_proxy(&self, kind: ProxyKind) -> Box<dyn Proxy + Send>;
}
