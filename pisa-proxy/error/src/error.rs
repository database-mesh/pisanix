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

use std::{error::Error as StdError, io::Error as IoError};

use mysql_parser::parser::ParseError;
use mysql_protocol::err::ProtocolError;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ErrorKind {
    #[error("runtime error: {0:?}")]
    Runtime(#[from] Box<dyn StdError + Send + Sync>),

    #[error("stdio error: {0:?}")]
    Io(#[from] IoError),

    #[error("parser error: {0:?}")]
    Parser(#[from] ParseError),

    #[error("protocol error: {0:?}")]
    Protocol(#[from] ProtocolError),
}

#[derive(Debug, ThisError)]
#[error("{kind}")]
pub struct Error {
    kind: ErrorKind,
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Error::new(kind)
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Error {
        Error { kind }
    }
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}
