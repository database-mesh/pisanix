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

pub trait Session {
    fn get_db(&self) -> Option<String>; 
    fn get_charset(&self) -> Option<String>;
    fn get_autocommit(&self) -> Option<String>;
}

pub trait SessionMut {
    fn set_db(&mut self, db: String);
    fn set_charset(&mut self, charset: String);
    fn set_autocommit(&mut self, autocommit: String);
}