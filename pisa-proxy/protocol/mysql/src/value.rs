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
use chrono::{Duration, NaiveDateTime, NaiveDate, NaiveTime};

use crate::err::DecodeRowError;

pub type Result<T> = std::result::Result<Option<T>, Box<dyn std::error::Error>>;

pub trait Value: Sized {
    type Item: Convert<Self>;
    fn from(val: &[u8]) -> Result<Self>;
}

impl Value for String {
    type Item = String;
    fn from(val: &[u8]) -> Result<Self> {
        <Self::Item as Convert<String>>::new(val)
    }
}

impl Value for u64 {
    type Item = u64;
    fn from(val: &[u8]) -> Result<Self> {
        <Self::Item as Convert<u64>>::new(val)
    }
}

impl Value for Duration {
   type Item = Duration; 
   fn from(val: &[u8]) -> Result<Self> {
        <Self::Item as Convert<Duration>>::new(val)
   }
}

impl Value for NaiveDateTime {
    type Item = NaiveDateTime;
   fn from(val: &[u8]) -> Result<Self> {
        <Self::Item as Convert<NaiveDateTime>>::new(val)
   }
}

impl Value for NaiveDate {
    type Item = NaiveDate;
   fn from(val: &[u8]) -> Result<Self> {
        <Self::Item as Convert<NaiveDate>>::new(val)
   }
}
impl Value for NaiveTime {
    type Item = NaiveTime;
   fn from(val: &[u8]) -> Result<Self> {
        <Self::Item as Convert<NaiveTime>>::new(val)
   }
}

pub trait Convert<T> {
    fn new(val: &[u8]) -> Result<T>;
}

impl Convert<String> for String {
    fn new(val: &[u8]) -> Result<String> {
        Ok(Some(String::from_utf8(val.to_vec())?))
    }
}

impl Convert<u64> for u64 {
    fn new(mut val: &[u8]) -> Result<u64> {
        Ok(Some(val.get_uint_le(val.len())))
    }
}

impl Convert<Duration> for Duration {
   fn new(mut val: &[u8]) -> Result<Duration> {
        let length = val.len();
        let is_neg = val.get_u8();
        match length {
            8 | 12 => {
                let day = val.get_uint_le(4) as i64;
                let hour = val.get_u8() as i64;
                let minute = val.get_u8() as i64;
                let second = val.get_u8() as i64;

                let mut total_micro_second: i64 = (day * 24 * 60 * 60 + hour * 60 * 60 + minute * 60 + second) * 1000 * 1000;

                if val.has_remaining() {
                    let micro_second = val.get_uint_le(4) as i64;
                    total_micro_second += micro_second;
                }

                if is_neg == 1 {
                    total_micro_second *= -1;
                }

                Ok(Some(Duration::microseconds(total_micro_second)))
            }

            0 => {
                Ok(Some(Duration::seconds(0)))
            }

            x => Err(DecodeRowError::ColumnTimeLengthInvalid(x).into())
        }
   }
}


impl Convert<NaiveDateTime> for NaiveDateTime  {
    fn new(mut val: &[u8]) -> Result<NaiveDateTime> {
        let length = val.len();
        match length {
            0 | 4  => {
                Ok(None)
            }

            7 | 11 => {
                let year = val.get_uint_le(2) as i32;
                let month = val.get_u8();
                let day = val.get_u8();
                let hour = val.get_u8();
                let minute = val.get_u8();
                let second = val.get_u8(); 

                let d = NaiveDate::from_ymd(year, month.into(), day.into());

                let dt = if val.has_remaining() {
                    let micro_second = val.get_u32_le();
                    d.and_hms_micro(hour.into(), minute.into(), second.into(), micro_second)
                } else {
                    d.and_hms(hour.into(), minute.into(), second.into())
                };

                Ok(Some(dt))
            }

            x => Err(DecodeRowError::ColumnDateTimeLengthInvalid(x).into())
        }
    }
}

impl Convert<NaiveDate> for NaiveDate  {
    fn new(mut val: &[u8]) -> Result<NaiveDate> {
        let length = val.len();
        match length {
            0 => {
                Ok(None)
            }

            4 | 7 | 11 => {
                let year = val.get_uint_le(2) as i32;
                let month = val.get_u8();
                let day = val.get_u8();
                let d = NaiveDate::from_ymd(year, month.into(), day.into());
                Ok(Some(d))
            }

            x => Err(DecodeRowError::ColumnDateTimeLengthInvalid(x).into())
        }
    }
}


impl Convert<NaiveTime> for NaiveTime  {
    fn new(mut val: &[u8]) -> Result<NaiveTime> {
        let length = val.len();
        match length {
            0 | 4  => {
                Ok(None)
            }

            7 | 11 => {
                val.advance(4);
                let hour = val.get_u8();
                let minute = val.get_u8();
                let second = val.get_u8(); 

                let t = if val.has_remaining() {
                    let micro_second = val.get_u32_le();
                    NaiveTime::from_hms_micro(hour.into(), minute.into(), second.into(), micro_second)
                } else {
                    NaiveTime::from_hms(hour.into(), minute.into(), second.into())
                };

                Ok(Some(t))
            }

            x => Err(DecodeRowError::ColumnDateTimeLengthInvalid(x).into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::Value;

    fn to_string<T: Value>(val: &[u8]) -> Option<T> {
        Value::from(val).unwrap()
    }

    #[test]
    fn test_to_string() {
        let data: Vec<u8> = vec![78, 111];

        let res = to_string::<String>(&data);
        assert_eq!(res, Some("No".to_string()));

    
    }
}
