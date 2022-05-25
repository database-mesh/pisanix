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

use std::{error::Error, fmt};

#[derive(Debug)]
pub struct MySQLError {
    code: u16,
    message: &'static str,
    state: &'static str,
}

impl MySQLError {
    pub fn new(code: u16, message: &'static str, state: &'static str) -> MySQLError {
        MySQLError { code, message, state }
    }

    pub fn set_code(&mut self, code: u16) {
        self.code = code;
    }

    pub fn set_message(&mut self, msg: &'static str) {
        self.message = msg;
    }

    pub fn set_status(&mut self, status: &'static str) {
        self.state = status;
    }
}

impl fmt::Display for MySQLError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for MySQLError {
    fn description(&self) -> &str {
        self.message
    }
}

#[derive(Debug)]
pub struct InvalidPacketError(pub String);

impl fmt::Display for InvalidPacketError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "error: {}", self.0)
    }
}

impl Error for InvalidPacketError {}
