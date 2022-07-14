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

use std::sync::Arc;
use endpoint::endpoint::Endpoint;
use loadbalance::balance::LoadBalance;
use tokio::sync::{Mutex, RwLock};
use crossbeam_channel::unbounded;

use super::{
    rule_match::{RulesMatch, RulesMatchBuilder},
    ReadWriteEndpoint,
};
use crate::{
    config,
    config::TargetRole,
    discovery::{
        discovery::{Discovery, DiscoveryKind, DiscoveryMasterHighAvailability},
        monitor_reconcile::MonitorReconcile,
    },
    route::{BoxError, RouteBalance},
    Route, RouteInput,
};

pub struct ReadWriteSplittingDynamicBuilder;

impl ReadWriteSplittingDynamicBuilder {
    pub fn build(
        config: config::ReadWriteSplittingDynamic,
        rw_endpoint: ReadWriteEndpoint,
    ) -> ReadWriteSplittingDynamic {

        let mut mc = MonitorReconcile::new(config.clone());

        let rules_match = RulesMatchBuilder::build(
            config.clone().rules,
            config.clone().default_target,
            rw_endpoint,
        );

        mc.start_monitor_reconcile();

        ReadWriteSplittingDynamic {rules_match}
    }
}

pub struct ReadWriteSplittingDynamic {
    rules_match: RulesMatch,
}

impl Route for ReadWriteSplittingDynamic {
    type Error = BoxError;

    fn dispatch<'a>(
        &'a mut self,
        input: &RouteInput,
    ) -> Result<(Option<&'a Endpoint>, TargetRole), Self::Error> {
        let b = self.rules_match.get(input);
        Ok((b.0.next(), b.1))
    }
}