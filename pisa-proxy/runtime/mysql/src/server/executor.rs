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

use futures::executor;
use mysql_protocol::client::conn::SessionAttr;
use pisa_error::error::{Error, ErrorKind};
use strategy::sharding_rewrite::{DataSource, ShardingRewriteOutput};

#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    #[error("execute sql: {0:?} error")]
    DataSourceNotFound(String),
}

pub fn sharding_executor(
    rewrite_outputs: Vec<ShardingRewriteOutput>,
    attrs: Vec<SessionAttr>,
) -> Result<(), Error> {
    let endpoints = rewrite_outputs
        .iter()
        .map(|x| match &x.data_source {
            DataSource::Endpoint(ep) => Ok(ep),
            _ => Err(ExecuteError::DataSourceNotFound(x.target_sql.clone())),
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ErrorKind::Runtime(e.into()));

    Ok(())
}
