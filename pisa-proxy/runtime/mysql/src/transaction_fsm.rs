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

use async_trait::async_trait;
use conn_pool::{ConnAttrMut, Pool, PoolConn};
use endpoint::endpoint::Endpoint;
use mysql_protocol::client::conn::{ClientConn, SessionAttr};
use pisa_error::error::{Error, ErrorKind};
use strategy::route::{Route, RouteInput, RouteStrategy};
use tokio::sync::Mutex;
use tracing::debug;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TransState {
    TransDummyState,
    TransUseState,
    TransSetSessionState,
    TransStartState,
    TransPrepareState,
}

impl Default for TransState {
    fn default() -> Self {
        TransState::TransDummyState
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TransEventName {
    DummyEvent,
    UseEvent,
    SetSessionEvent,
    QueryEvent,
    StartEvent,
    PrepareEvent,
    SendLongDataEvent,
    ExecuteEvent,
    CloseEvent,
    ResetEvent,
    CommitRollBackEvent,
    DropEvent,
}

impl Default for TransEventName {
    fn default() -> Self {
        TransEventName::DummyEvent
    }
}

#[async_trait]
pub trait ConnDriver {
    async fn get_driver_conn(
        &self,
        loadbalance: Arc<Mutex<RouteStrategy>>,
        pool: &mut Pool<ClientConn>,
        input: RouteInput<'_>,
    ) -> Result<(PoolConn<ClientConn>, Option<Endpoint>), Error>;
}

#[derive(Clone)]
pub struct Driver;

#[async_trait]
impl ConnDriver for Driver {
    async fn get_driver_conn(
        &self,
        route_strategy: Arc<Mutex<RouteStrategy>>,
        pool: &mut Pool<ClientConn>,
        input: RouteInput<'_>,
    ) -> Result<(PoolConn<ClientConn>, Option<Endpoint>), Error> {
        let mut strategy = route_strategy.clone().lock_owned().await;
        let dispatch_res = strategy.dispatch(&input).unwrap();
        debug!("route_strategy to {:?} for input: {:?}", dispatch_res, input);

        let endpoint = dispatch_res.0.unwrap();
        let factory = ClientConn::with_opts(
            endpoint.user.clone(),
            endpoint.password.clone(),
            endpoint.addr.clone(),
        );
        pool.set_factory(factory);

        match pool.get_conn_with_endpoint(endpoint.addr.as_ref()).await {
            Ok(client_conn) => {
                if !client_conn.is_ready().await {
                    return Ok((
                        pool.rebuild_conn().await.map_err(ErrorKind::Protocol)?,
                        Some(endpoint.clone()),
                    ));
                }
                Ok((client_conn, Some(endpoint.clone())))
            }
            Err(err) => {
                println!("errr {:?}", err);
                Err(Error::new(ErrorKind::Protocol(err)))
            }
        }
    }
}

pub struct TransEvent {
    name: TransEventName,
    src_state: TransState,
    dst_state: TransState,
    driver: Option<Box<dyn ConnDriver + Send + Sync>>,
}

fn init_trans_events() -> Vec<TransEvent> {
    return vec![
        TransEvent {
            name: TransEventName::UseEvent,
            src_state: TransState::TransDummyState,
            dst_state: TransState::TransUseState,
            driver: Some(Box::new(Driver)),
        },
        TransEvent {
            name: TransEventName::UseEvent,
            src_state: TransState::TransUseState,
            dst_state: TransState::TransUseState,
            driver: Some(Box::new(Driver)),
        },
        TransEvent {
            name: TransEventName::SetSessionEvent,
            src_state: TransState::TransDummyState,
            dst_state: TransState::TransSetSessionState,
            driver: Some(Box::new(Driver)),
        },
        TransEvent {
            name: TransEventName::SetSessionEvent,
            src_state: TransState::TransUseState,
            dst_state: TransState::TransSetSessionState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::SetSessionEvent,
            src_state: TransState::TransSetSessionState,
            dst_state: TransState::TransSetSessionState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::QueryEvent,
            src_state: TransState::TransSetSessionState,
            dst_state: TransState::TransSetSessionState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::QueryEvent,
            src_state: TransState::TransUseState,
            dst_state: TransState::TransUseState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::QueryEvent,
            src_state: TransState::TransDummyState,
            dst_state: TransState::TransDummyState,
            driver: Some(Box::new(Driver)),
        },
        TransEvent {
            name: TransEventName::StartEvent,
            src_state: TransState::TransDummyState,
            dst_state: TransState::TransStartState,
            driver: Some(Box::new(Driver)),
        },
        TransEvent {
            name: TransEventName::StartEvent,
            src_state: TransState::TransUseState,
            dst_state: TransState::TransStartState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::StartEvent,
            src_state: TransState::TransSetSessionState,
            dst_state: TransState::TransStartState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::PrepareEvent,
            src_state: TransState::TransDummyState,
            dst_state: TransState::TransPrepareState,
            driver: Some(Box::new(Driver)),
        },
        TransEvent {
            name: TransEventName::PrepareEvent,
            src_state: TransState::TransUseState,
            dst_state: TransState::TransPrepareState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::PrepareEvent,
            src_state: TransState::TransStartState,
            dst_state: TransState::TransPrepareState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::SendLongDataEvent,
            src_state: TransState::TransPrepareState,
            dst_state: TransState::TransPrepareState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::ExecuteEvent,
            src_state: TransState::TransPrepareState,
            dst_state: TransState::TransPrepareState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::CloseEvent,
            src_state: TransState::TransPrepareState,
            dst_state: TransState::TransDummyState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::ResetEvent,
            src_state: TransState::TransPrepareState,
            dst_state: TransState::TransDummyState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::DropEvent,
            src_state: TransState::TransPrepareState,
            dst_state: TransState::TransDummyState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::CommitRollBackEvent,
            src_state: TransState::TransPrepareState,
            dst_state: TransState::TransDummyState,
            driver: None,
        },
        TransEvent {
            name: TransEventName::CommitRollBackEvent,
            src_state: TransState::TransDummyState,
            dst_state: TransState::TransDummyState,
            driver: Some(Box::new(Driver)),
        },
        TransEvent {
            name: TransEventName::CommitRollBackEvent,
            src_state: TransState::TransStartState,
            dst_state: TransState::TransDummyState,
            driver: Some(Box::new(Driver)),
        },
        TransEvent {
            name: TransEventName::QueryEvent,
            src_state: TransState::TransSetSessionState,
            dst_state: TransState::TransDummyState,
            driver: Some(Box::new(Driver)),
        },
    ];
}

pub struct TransFsm {
    pub events: Vec<TransEvent>,
    pub current_state: TransState,
    pub current_event: TransEventName,
    pub route_strategy: Arc<Mutex<RouteStrategy>>,
    pub pool: Pool<ClientConn>,
    pub client_conn: Option<PoolConn<ClientConn>>,
    pub endpoint: Option<Endpoint>,
    pub db: Option<String>,
    pub charset: String,
    pub autocommit: Option<String>,
}

impl TransFsm {
    pub fn new_trans_fsm(
        route_strategy: Arc<Mutex<RouteStrategy>>,
        pool: Pool<ClientConn>,
    ) -> TransFsm {
        TransFsm {
            events: init_trans_events(),
            current_state: TransState::TransDummyState,
            current_event: TransEventName::DummyEvent,
            route_strategy,
            pool,
            client_conn: None,
            endpoint: None,
            db: None,
            charset: String::from("utf8mb4"),
            autocommit: None,
        }
    }

    pub async fn trigger(
        &mut self,
        state_name: TransEventName,
        input: RouteInput<'_>,
    ) -> Result<(), Error> {
        for event in &self.events {
            if event.name == state_name && event.src_state == self.current_state {
                match event.src_state {
                    TransState::TransDummyState => {
                        let (mut client_conn, endpoint) = event
                            .driver
                            .as_ref()
                            .unwrap()
                            .get_driver_conn(self.route_strategy.clone(), &mut self.pool, input)
                            .await?;

                        client_conn.init(self.build_conn_attrs()).await;
                        self.client_conn = Some(client_conn);

                        self.endpoint = endpoint;
                    }
                    _ => {}
                }
                self.current_state = event.dst_state;
                self.current_event = event.name;
                return Ok(());
            }
        }
        Ok(())
    }

    // when autocommit=0, should be reset fsm state
    pub async fn reset_fsm_state(&mut self, input: RouteInput<'_>) -> Result<(), Error> {
        self.current_state = TransState::TransDummyState;
        self.current_event = TransEventName::DummyEvent;

        self.trigger(TransEventName::QueryEvent, input).await?;
        Ok(())
    }

    // Set current db.
    pub fn set_db(&mut self, db: Option<String>) {
        self.db = db
    }

    // Set current charset
    pub fn set_charset(&mut self, name: String) {
        self.charset = name;
    }

    // Set current autocommit
    pub fn set_autocommit(&mut self, status: String) {
        self.autocommit = Some(status)
    }

    pub async fn get_conn(&mut self) -> Result<PoolConn<ClientConn>, Error> {
        let conn = self.client_conn.take();
        let addr = self.endpoint.as_ref().unwrap().addr.as_ref();
        match conn {
            Some(client_conn) => Ok(client_conn),
            None => match self.pool.get_conn_with_endpoint(addr).await {
                Ok(mut client_conn) => {
                    client_conn.init(self.build_conn_attrs()).await;
                    Ok(client_conn)
                }
                Err(err) => Err(Error::new(ErrorKind::Protocol(err))),
            },
        }
    }

    pub fn put_conn(&mut self, conn: PoolConn<ClientConn>) {
        self.client_conn = Some(conn)
    }

    #[inline]
    fn build_conn_attrs(&self) -> Vec<SessionAttr> {
        vec![
            SessionAttr::DB(self.db.clone()),
            SessionAttr::Charset(self.charset.clone()),
            SessionAttr::Autocommit(self.autocommit.clone()),
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_trigger() {
        let lb = Arc::new(tokio::sync::Mutex::new(RouteStrategy::None));
        let mut tsm = TransFsm::new_trans_fsm(lb, Pool::new(1));
        tsm.current_state = TransState::TransUseState;
        let _ = tsm.trigger(TransEventName::QueryEvent, RouteInput::None).await;
        assert_eq!(tsm.current_state, TransState::TransUseState);
        assert_eq!(tsm.current_event, TransEventName::QueryEvent);
        let _ = tsm.trigger(TransEventName::SetSessionEvent, RouteInput::None).await;
        assert_eq!(tsm.current_state, TransState::TransSetSessionState);
        assert_eq!(tsm.current_event, TransEventName::SetSessionEvent);
        let _ = tsm.trigger(TransEventName::StartEvent, RouteInput::None).await;
        assert_eq!(tsm.current_state, TransState::TransStartState);
        assert_eq!(tsm.current_event, TransEventName::StartEvent);
        let _ = tsm.trigger(TransEventName::PrepareEvent, RouteInput::None).await;
        assert_eq!(tsm.current_state, TransState::TransPrepareState);
        assert_eq!(tsm.current_event, TransEventName::PrepareEvent);
        let _ = tsm.trigger(TransEventName::SendLongDataEvent, RouteInput::None).await;
        assert_eq!(tsm.current_state, TransState::TransPrepareState);
        assert_eq!(tsm.current_event, TransEventName::SendLongDataEvent);
        let _ = tsm.trigger(TransEventName::ExecuteEvent, RouteInput::None).await;
        assert_eq!(tsm.current_state, TransState::TransPrepareState);
        assert_eq!(tsm.current_event, TransEventName::ExecuteEvent);
        let _ = tsm.trigger(TransEventName::CommitRollBackEvent, RouteInput::None).await;
        assert_eq!(tsm.current_state, TransState::TransDummyState);
        assert_eq!(tsm.current_event, TransEventName::CommitRollBackEvent);
    }
}
