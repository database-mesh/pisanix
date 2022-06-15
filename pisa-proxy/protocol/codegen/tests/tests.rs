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

use mysql_protocol::client::{
    auth::ClientAuth,
    codec::CommonCodec,
    resultset::ResultsetCodec,
    stmt::Stmt,
    stream::{LocalStream, StreamWrapper},
};
use tokio_util::codec::Framed;

#[macro_use]
extern crate protocol_codegen;

#[derive(mysql_codec_convert)]
pub enum ClientCodec {
    ClientAuth(Framed<LocalStream, ClientAuth>),
    Resultset(Framed<LocalStream, ResultsetCodec>),
    Stmt(Framed<LocalStream, Stmt>),
    Common(Framed<LocalStream, CommonCodec>),
}

#[test]
fn test_mysql_codec_convert() {
    let stream = LocalStream::new(StreamWrapper::Plain(None));
    let framed = Framed::new(stream, Stmt::new());
    let stmt_codec = ClientCodec::Stmt(framed);
    let common_codec = stmt_codec.into_common();
    let res = format!("{:?}", common_codec);
    assert!(res.contains("CommonCodec"))
}
