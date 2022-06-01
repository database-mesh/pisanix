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
use conn_pool::{ConnAttr, Pool, PoolConn};
use endpoint::endpoint::Endpoint;
use loadbalance::balance::LoadBalance;
use mysql_protocol::client::conn::ClientConn;
use pisa_error::error::{Error, ErrorKind};
use tokio::sync::Mutex;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TransState {
    TransDummyState,
    TransUseState,
    TransSetSessionState,
    TransStartState,
    TransPrepareState,
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

#[async_trait]
pub trait ConnDriver {
    async fn get_driver_conn(
        &self,
        loadbalance: Arc<Mutex<Box<dyn LoadBalance + Send + Sync>>>,
        pool: &mut Pool<ClientConn>,
    ) -> Result<(PoolConn<ClientConn>, Option<Endpoint>), Error>;
}

#[derive(Clone)]
pub struct Driver;

#[async_trait]
impl ConnDriver for Driver {
    async fn get_driver_conn(
        &self,
        loadbalance: Arc<Mutex<Box<dyn LoadBalance + Send + Sync>>>,
        pool: &mut Pool<ClientConn>,
    ) -> Result<(PoolConn<ClientConn>, Option<Endpoint>), Error> {
        let mut lb = loadbalance.clone().lock_owned().await;
        let endpoint = lb.next().unwrap();

        let factory = ClientConn::with_opts(
            endpoint.user.clone(),
            endpoint.password.clone(),
            endpoint.addr.clone(),
        );

        pool.set_factory(factory);

        match pool.get_conn_with_opts(endpoint.addr.as_ref()).await {
            Ok(client_conn) => Ok((client_conn, Some(endpoint.clone()))),
            Err(err) => Err(Error::new(ErrorKind::Protocol(err))),
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
    ];
}

pub struct TransFsm {
    pub events: Vec<TransEvent>,
    pub current_state: TransState,
    pub current_event: TransEventName,
    pub lb: Arc<Mutex<Box<dyn LoadBalance + Send + Sync>>>,
    pub pool: Pool<ClientConn>,
    pub client_conn: Option<PoolConn<ClientConn>>,
    pub endpoint: Option<Endpoint>,
    pub db: Option<String>,
}

impl TransFsm {
    pub fn new_trans_fsm(
        lb: Arc<Mutex<Box<dyn LoadBalance + Send + Sync>>>,
        pool: Pool<ClientConn>,
    ) -> TransFsm {
        TransFsm {
            events: init_trans_events(),
            current_state: TransState::TransDummyState,
            current_event: TransEventName::DummyEvent,
            lb,
            pool,
            client_conn: None,
            endpoint: None,
            db: None,
        }
    }

    pub async fn trigger(&mut self, state_name: TransEventName) -> Result<(), Error> {
        for event in &self.events {
            if event.name == state_name && event.src_state == self.current_state {
                match event.src_state {
                    TransState::TransDummyState => {
                        let (mut client_conn, endpoint) = event
                            .driver
                            .as_ref()
                            .unwrap()
                            .get_driver_conn(self.lb.clone(), &mut self.pool)
                            .await?;
                        if let None = client_conn.get_db() {
                            if let Some(db) = &self.db {
                                client_conn.send_use_db(db).await.map_err(ErrorKind::Protocol)?;
                            }
                        }
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

    // Set current db.
    pub fn set_db(&mut self, db: String) {
        self.db = Some(db)
    }

    pub async fn get_conn(&mut self) -> Result<PoolConn<ClientConn>, Error> {
        let conn = self.client_conn.take();
        let addr = self.endpoint.as_ref().unwrap().addr.as_ref();
        match conn {
            Some(client_conn) => Ok(client_conn),
            None => match self.pool.get_conn_with_opts(addr).await {
                Ok(client_conn) => Ok(client_conn),
                Err(err) => Err(Error::new(ErrorKind::Protocol(err))),
            },
        }
    }

    pub fn put_conn(&mut self, conn: PoolConn<ClientConn>) {
        self.client_conn = Some(conn)
    }
}
