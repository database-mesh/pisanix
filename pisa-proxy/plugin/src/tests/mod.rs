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

use std::{io::Error, time::Duration};

use crate::{
    circuit_breaker::CircuitBreakerLayer,
    config,
    err::PluginError,
    layer::{service_fn, Service, ServiceBuilder},
    limit::LimitLayer,
};

fn test_service(input: &str) -> Result<String, Error> {
    Ok(input.to_string())
}

#[test]
fn test_chain_limit_audit() {
    let limit_config = vec![config::Limit {
        regex: String::from(r"[A-Za-z]+$"),
        limit: 0,
        duration: Duration::new(5, 0),
    }];

    let audit_config = vec![config::Audit { regex: String::from(r"[A-Za-z]+") }];

    let mut wrap_svc = ServiceBuilder::new()
        .with_layer(LimitLayer::new(limit_config))
        .with_layer(CircuitBreakerLayer::new(audit_config))
        .build(service_fn(test_service));

    let res = wrap_svc.handle("abc");
    println!("{:?}", res);
    if let Err(e) = res {
        let e = e.downcast::<PluginError>().unwrap();
        assert_eq!(*e, PluginError::LimitPluginReject)
    }
}
