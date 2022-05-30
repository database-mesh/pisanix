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

use std::io;

use thiserror::Error;
use tokio_native_tls::native_tls;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ProtocolError {
    #[error("handshake error {0:?}")]
    HandshakeError(Vec<u8>),

    #[error("unsupport protocol version {0}")]
    ProtocolVersion(u8),

    #[error("mysql server can not support protocol 41 and above required by the client")]
    ServerProtocolVersion,

    #[error("mysql server does not support tls required by the client")]
    Tls,

    #[error("make tls error {0:?}")]
    MakeTls(#[from] native_tls::Error),

    #[error("method: {:?} invalid packet {:?}", .method, .data)]
    InvalidPacket { method: String, data: Vec<u8> },

    #[error("{0:?}")]
    AuthFailed(Vec<u8>),

    #[error("unsupport auth plugin {0:?}")]
    AuthPluginUnsupport(String),

    #[error("stdio error: {0:?}")]
    Io(#[from] io::Error),

    #[error("public key parse error: {0:?}")]
    PubKey(#[from] rsa::pkcs8::spki::Error),

    #[error("prepare stmt return error: {0:?}")]
    PrepareError(Vec<u8>),

    #[error("read error packet: {0:?}")]
    PacketError(Vec<u8>),

    #[error("stmt id not found: {0}")]
    StmtIdNotFound(u32),
}
