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

use bytes::Buf;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub trait Value: Sized {
    type Item: Convert<Self>;
    fn from(val: Vec<u8>) -> Result<Self>;
}

impl Value for String {
    type Item = String;
    fn from(val: Vec<u8>) -> Result<Self> {
        <Self::Item as Convert<String>>::new(val)
    }
}

impl Value for u64 {
    type Item = u64;
    fn from(val: Vec<u8>) -> Result<Self> {
        <Self::Item as Convert<u64>>::new(val)
    }
}

impl Value for Option<u64> {
    type Item = Option<u64>;
    fn from(val: Vec<u8>) -> Result<Self> {
        <Self::Item as Convert<Option<u64>>>::new(val)
    }
}

pub trait Convert<T> {
    fn new(val: Vec<u8>) -> Result<T>;
}

impl Convert<String> for String {
    fn new(val: Vec<u8>) -> Result<String> {
        String::from_utf8(val).map_err(|e| e.into())
    }
}

impl Convert<u64> for u64 {
    fn new(val: Vec<u8>) -> Result<u64> {
        let mut buf = val.as_slice();
        Ok(buf.get_uint_le(val.len()))
    }
}

impl Convert<Option<u64>> for Option<u64> {
    fn new(val: Vec<u8>) -> Result<Option<u64>> {
        let buf = val.as_slice();
        if buf[0] == 0 {
            return Ok(None);
        }
        let v = String::from_utf8(val)?;
        let result = v.parse::<u64>()?;
        Ok(Some(result))
    }
}

#[cfg(test)]
mod test {
    use super::Value;

    fn to_string<T: Value>(val: Vec<u8>) -> T {
        Value::from(val).unwrap()
    }

    #[test]
    fn test_to_string() {
        let data: Vec<u8> = vec![78, 111];

        let res = to_string::<String>(data);
        assert_eq!(res, "No");
    }
}
