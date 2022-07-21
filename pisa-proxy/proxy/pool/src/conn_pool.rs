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
use dashmap::DashMap;
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
    // Get current charset
    fn get_charset(&self) -> Option<String>;
    // Get current autocommit status
    fn get_autocommit(&self) -> Option<String>;
}

#[async_trait]
pub trait ConnAttrMut {
    type Item: Send;
    async fn init(&mut self, _items: Vec<Self::Item>) {}
}

#[derive(Debug)]
pub struct PoolInner<T: ConnLike + ConnAttr + ConnAttrMut> {
    pub inner: ArrayQueue<T>,
}

impl<T: ConnLike + ConnAttr + ConnAttrMut> PoolInner<T> {
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
pub struct PoolConn<T>
where
    T: ConnLike + ConnAttr + ConnAttrMut,
{
    pub pool: Arc<DashMap<String, PoolInner<T>>>,
    pub conn: Option<T>,
}

#[derive(Debug, Clone)]
pub struct Pool<T>
where
    T: ConnLike + ConnAttr + ConnAttrMut,
{
    factory: Option<T>,
    size: usize,
    pool: Arc<DashMap<String, PoolInner<T>>>,
}

impl<T> Pool<T>
where
    T: ConnLike + ConnAttr + ConnAttrMut + std::default::Default,
{
    pub fn new(size: usize) -> Pool<T> {
        //let pool_inner = PoolInner::new(size);

        Pool { factory: None, size, pool: Arc::new(DashMap::<String, PoolInner<T>>::new()) }
    }

    pub fn set_factory(&mut self, factory: T) {
        self.factory = Some(factory)
    }

    // Get connection by endpoint attribute
    pub async fn get_conn_with_endpoint(&self, endpoint: &str) -> Result<PoolConn<T>, T::Error> {
        let conn = self.pool.get(endpoint);
        let conn = match conn {
            Some(val) => val.get_conn(),
            None => None,
        };

        let conn = match conn {
            Some(conn) => conn,
            None => {
                if !self.pool.contains_key(endpoint) {
                    self.pool.insert(endpoint.to_string(), PoolInner::new(self.size));
                }

                self.factory.as_ref().unwrap().build_conn().await?
            }
        };

        Ok(PoolConn { pool: Arc::clone(&self.pool), conn: Some(conn) })
    }

    pub fn len(&self, endpoint: &str) -> usize {
        match self.pool.get(endpoint) {
            Some(inner) => inner.inner.len(),
            None => 0,
        }
    }
}

impl<T> Deref for PoolConn<T>
where
    T: ConnLike + ConnAttr + ConnAttrMut,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.conn.as_ref().unwrap()
    }
}

impl<T> DerefMut for PoolConn<T>
where
    T: ConnLike + ConnAttr + ConnAttrMut,
{
    fn deref_mut(&mut self) -> &mut T {
        self.conn.as_mut().unwrap()
    }
}

impl<T> Drop for PoolConn<T>
where
    T: ConnLike + ConnAttr + ConnAttrMut,
{
    fn drop(&mut self) {
        if self.conn.is_some() {
            debug!("put conn {:?}", &self.conn);
            let conn = self.conn.take().unwrap();
            let endpoint = conn.get_endpoint();
            self.pool.get_mut(&endpoint).unwrap().put_conn(conn);
        }
    }
}
