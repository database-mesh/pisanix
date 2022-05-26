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

use regex::Regex;

use crate::{
    config,
    err::{BoxError, PluginError},
    layer::{Layer, Service},
};

#[derive(Clone)]
pub struct CircuitBreakerLayer {
    config: Option<Vec<config::CircuitBreaker>>,
}

#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub regex: String,
}

#[derive(Clone)]
pub struct CircuitBreakerInstance {
    regex: Regex,
}

impl CircuitBreakerLayer {
    pub fn new(config: Vec<config::CircuitBreaker>) -> CircuitBreakerLayer {
        CircuitBreakerLayer { config: Some(config) }
    }

    pub fn with_opt(config: Option<Vec<config::CircuitBreaker>>) -> CircuitBreakerLayer {
        CircuitBreakerLayer { config }
    }

    fn create_instances(&self) -> Option<Vec<CircuitBreakerInstance>> {
        if let Some(config) = &self.config {
            let mut instances = Vec::with_capacity(config.len());
            for c in config {
                let regex = Regex::new(&c.regex).unwrap();
                instances.push(CircuitBreakerInstance { regex })
            }
            return Some(instances);
        }

        None
    }
}

impl<S> Layer<S> for CircuitBreakerLayer {
    type Service = CircuitBreaker<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let instances = self.create_instances();
        CircuitBreaker { inner, instances }
    }
}

#[derive(Clone)]
pub struct CircuitBreaker<S> {
    inner: S,
    instances: Option<Vec<CircuitBreakerInstance>>,
}

impl<S> CircuitBreaker<S> {
    // if allow return true, otherwise return false
    fn is_allow(&self, input: &str) -> bool {
        if let Some(instances) = &self.instances {
            for c in instances {
                if c.regex.is_match(input) {
                    return false;
                }
            }
        }

        true
    }
}

impl<S, Input> Service<Input> for CircuitBreaker<S>
where
    S: Service<Input>,
    Input: AsRef<str>,
    S::Error: Into<BoxError>,
{
    type Output = S::Output;
    type Error = BoxError;

    fn handle(&mut self, input: Input) -> Result<Self::Output, Self::Error> {
        let is_allow = self.is_allow(input.as_ref());
        if is_allow {
            return self.inner.handle(input).map_err(Into::into);
        }

        Err(Box::new(PluginError::CircuitBreakerPluginReject))
    }
}

#[cfg(test)]
mod test {
    use std::io::Error;

    use super::CircuitBreakerLayer;
    use crate::{
        config,
        layer::{service_fn, Service, ServiceBuilder},
    };

    fn test_service(input: &str) -> Result<String, Error> {
        Ok(input.to_string())
    }

    #[test]
    fn test_circuit_breaker() {
        let config = vec![config::CircuitBreaker { regex: String::from(r"[A-Za-z]+") }];

        let mut wrap_svc = ServiceBuilder::new()
            .with_layer(CircuitBreakerLayer::new(config))
            .build(service_fn(test_service));

        let res = wrap_svc.handle("abc");
        assert_eq!(res.is_err(), true)
    }
}
