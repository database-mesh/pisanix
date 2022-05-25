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

use std::io::Error;

use tokio::net::{TcpListener, TcpStream};
use tracing::info;

pub struct Listener {
    pub listen_addr: String,
    pub backend_type: String,
}

impl Listener {
    pub fn build_listener(&mut self) -> Result<TcpListener, Error> {
        info!("{:?} proxy listen on: {:?}", self.backend_type.clone(), self.listen_addr.clone());
        let listener = {
            let std_listenner = match std::net::TcpListener::bind(self.listen_addr.clone()) {
                Err(err) => return Err(err),
                Ok(listener) => listener,
            };
            if let Err(err) = std_listenner.set_nonblocking(true) {
                return Err(err);
            }
            TcpListener::from_std(std_listenner).expect("listener must be valid")
        };

        Ok(listener)
    }

    pub async fn accept(&mut self, listener: &TcpListener) -> Result<TcpStream, Error> {
        let (socket, addr) = match listener.accept().await {
            Ok((socket, addr)) => (socket, addr),
            Err(err) => return Err(err),
        };

        info!("[pisa] client_ip: {:?} - backend_type: {:?}", addr.ip(), self.backend_type);

        socket.set_nodelay(true).unwrap();
        Ok(socket)
    }
}
