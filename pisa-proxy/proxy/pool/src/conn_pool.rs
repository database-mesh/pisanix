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

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use async_trait::async_trait;
use crossbeam_queue::ArrayQueue;
use tracing::debug;

/// In order to be managed by the connection pool, Both the `ConnLike` and `ConnAttr` trait
/// needs to be implemented.
#[async_trait]
pub trait ConnLike: Sized + Send + Sync + std::fmt::Debug + 'static {
    type Error: Send + std::fmt::Debug + 'static;

    // Method for create connection
    async fn build_conn(&self) -> Result<Self, Self::Error>;
}

/// `ConnAttr` traits is used to get attribute of current connection  
pub trait ConnAttr {
    fn get_host(&self) -> String;
    fn get_port(&self) -> u16;
    fn get_user(&self) -> String;
    fn get_endpoint(&self) -> String;
    // Get current db on conn
    fn get_db(&self) -> Option<String>;
}

#[derive(Debug)]
pub struct PoolInner<T: ConnLike + ConnAttr> {
    pub inner: ArrayQueue<T>,
}

impl<T: ConnLike + ConnAttr> PoolInner<T> {
    fn new(size: usize) -> PoolInner<T> {
        PoolInner { inner: ArrayQueue::new(size) }
    }

    fn get_conn(&self) -> Option<T> {
        self.inner.pop()
    }

    fn put_conn(&self, conn: T) {
        if !self.inner.is_full() {
            let _ = self.inner.push(conn);
        }
    }
}

#[derive(Debug)]
pub struct PoolConn<T: ConnLike + ConnAttr>
where
    T: ConnLike,
{
    pub pool: Arc<PoolInner<T>>,
    pub conn: Option<T>,
}

#[derive(Clone)]
pub struct Pool<T: ConnLike + ConnAttr> {
    factory: Option<T>,
    pool: Arc<PoolInner<T>>,
}

impl<T> Pool<T>
where
    T: ConnLike + ConnAttr + std::default::Default,
{
    pub fn new(size: usize) -> Pool<T> {
        let pool_inner = PoolInner::new(size);

        Pool { factory: None, pool: Arc::new(pool_inner) }
    }

    pub fn set_factory(&mut self, factory: T) {
        self.factory = Some(factory)
    }

    // Get connection by endpoint attribute
    pub async fn get_conn_with_opts(&self, endpoint: &str) -> Result<PoolConn<T>, T::Error> {
        let pool_length = self.pool.inner.len();
        let mut conn: Option<T> = None;

        for _ in 0..pool_length {
            let curr_conn = self.pool.get_conn();
            match curr_conn {
                Some(curr_conn) => {
                    if endpoint == curr_conn.get_endpoint() {
                        conn = Some(curr_conn);
                        break;
                    } else {
                        self.pool.put_conn(curr_conn);
                        continue;
                    }
                }
                None => break,
            }
        }

        let conn = match conn {
            Some(conn) => conn,
            None => self.factory.as_ref().unwrap().build_conn().await?,
        };
        Ok(PoolConn { pool: self.pool.clone(), conn: Some(conn) })
    }

    pub async fn get_conn(&self) -> Result<PoolConn<T>, T::Error> {
        let conn = self.pool.get_conn();

        let conn = match conn {
            Some(conn) => conn,
            None => self.factory.as_ref().unwrap().build_conn().await?,
        };

        Ok(PoolConn { pool: self.pool.clone(), conn: Some(conn) })
    }

    pub fn len(&self) -> usize {
        self.pool.inner.len()
    }
}

impl<T: ConnLike + ConnAttr> Deref for PoolConn<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.conn.as_ref().unwrap()
    }
}

impl<T: ConnLike + ConnAttr> DerefMut for PoolConn<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.conn.as_mut().unwrap()
    }
}

impl<T: ConnLike + ConnAttr> Drop for PoolConn<T> {
    fn drop(&mut self) {
        futures::executor::block_on(async {
            if self.conn.is_some() {
                debug!("put conn {:?}", &self.conn);
                self.pool.put_conn(self.conn.take().unwrap())
            }
        })
    }
}
