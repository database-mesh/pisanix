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

use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    /// Charsets supported of mysql, used to check whether token is `UNDERSCORE_CHARSET`
    pub static ref CHARSETS: HashMap<&'static str, ()> = {
        let mut m = HashMap::new();
        m.insert("_UTF8", ());
        m.insert("_UTF8MB4", ());
        m.insert("_UTF16", ());
        m.insert("_UTF16LE", ());
        m.insert("_UTF32", ());
        m.insert("_BINARY", ());
        m.insert("_LATIN1", ());
        m.insert("_LATIN2", ());
        m.insert("_GB18030", ());
        m
    };
}

