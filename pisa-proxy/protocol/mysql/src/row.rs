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

use std::{sync::Arc, str::FromStr};

use crate::{
    column::ColumnInfo,
    err::DecodeRowError,
    mysql_const::*,
    util::{length_encode_int, BufExt},
    value::{self, Value},
};

pub trait RowData<T: AsRef<[u8]>> {
    fn with_buf(&mut self, buf: T);
    fn decode_with_name<V: Value>(&mut self, name: &str) -> value::Result<V>;
    fn get_row_data_with_name(&mut self, name: &str) -> value::Result<RowPartData>;
}

#[derive(Clone, Debug)]
pub enum RowDataTyp<T: AsRef<[u8]>> {
    Text(RowDataText<T>),
    Binary(RowDataBinary<T>),
}

#[derive(Clone, Debug)]
pub struct RowPartData {
    pub data: Box<[u8]>,
    pub start_idx: usize,
    pub part_encode_length: usize,
    pub part_data_length: usize,
}

crate::gen_row_data!(RowDataTyp, Text(RowDataText), Binary(RowDataBinary));

#[derive(Debug, Clone)]
pub struct RowDataCommon {
    columns: Arc<[ColumnInfo]>,
}

impl RowDataCommon {
    fn new(columns: Arc<[ColumnInfo]>) -> RowDataCommon {
        RowDataCommon { columns }
    }

    fn get_idx(&self, name: &str) -> std::result::Result<usize, DecodeRowError> {
        self.columns
            .iter()
            .position(|x| x.column_name == name)
            .ok_or_else(|| DecodeRowError::ColumnNotFound(name.to_string()))
    }
}

// For ProtocolText::ResultsetRow
#[derive(Debug, Clone)]
pub struct RowDataText<T: AsRef<[u8]>> {
    common: RowDataCommon,
    buf: T,
}

impl<T: AsRef<[u8]>> RowDataText<T> {
    pub fn new(columns: Arc<[ColumnInfo]>, buf: T) -> RowDataText<T> {
        let common = RowDataCommon::new(columns);
        RowDataText { common, buf }
    }
}

impl<T: AsRef<[u8]>> RowData<T> for RowDataText<T> {
    fn with_buf(&mut self, buf: T) {
        self.buf = buf;
    }
    // The method must be called in column order
    fn decode_with_name<V: Value>(&mut self, name: &str) -> value::Result<V> {
        let row_data = self.get_row_data_with_name(name)?;
        match row_data {
            Some(data) => Value::from(&data.data),
            _ => Ok(None),
        }
    }

    fn get_row_data_with_name(&mut self, name: &str) -> value::Result<RowPartData> {
        let name_idx = self.common.get_idx(name)?;
        let mut idx: usize = 0;
        for _ in 0..name_idx {
            let (length, _, pos) = length_encode_int(&self.buf.as_ref()[idx..]);
            idx += (length + pos) as usize;
        }

        let (length, is_null, pos) = length_encode_int(&self.buf.as_ref()[idx..]);
        if is_null || length == 0 {
            return Ok(None);
        }

        return Ok(Some(
            RowPartData {
                data: self.buf.as_ref()[idx + pos as usize .. idx + (pos + length) as usize].into(),
                start_idx: idx,
                part_encode_length: pos as usize,
                part_data_length: length as usize,
            }
        ));
    }
}

#[derive(Debug, Clone)]
pub struct RowDataBinary<T: AsRef<[u8]>> {
    common: RowDataCommon,
    null_map: Vec<u8>,
    buf: T,
    start_pos: usize,
}

impl<T: BufExt + AsRef<[u8]>> RowDataBinary<T> {
    pub fn new(columns: Arc<[ColumnInfo]>, buf: T) -> RowDataBinary<T> {
        let common = RowDataCommon::new(columns);
        RowDataBinary { common, null_map: vec![], buf, start_pos: 0 }
    }

    #[cfg(test)]
    pub fn test_new(columns: Arc<[ColumnInfo]>, mut buf: T) -> RowDataBinary<T> {
        let column_length = columns.len();
        let common = RowDataCommon::new(columns);
        // See https://dev.mysql.com/doc/internals/en/binary-protocol-resultset-row.html
        // Eat packet header
        let _ = buf.get_u8();
        let null_map_length = (column_length + 7 + 2) >> 3;
        // NULL Bitmap length: (column-count + 7 + 2) / 8

        let mut null_map = vec![0; null_map_length];
        buf.copy_to_slice(&mut null_map);
        RowDataBinary { common, null_map, buf, start_pos: 0, }
    }
}

use bytes::Buf;

impl<T: AsRef<[u8]>> RowData<T> for RowDataBinary<T> {
    fn with_buf(&mut self, buf: T) {
        let column_length = self.common.columns.len();
        // See https://dev.mysql.com/doc/internals/en/binary-protocol-resultset-row.html
        // NULL Bitmap length: (column-count + 7 + 2) / 8
        let null_map_length = (column_length + 7 + 2) >> 3;

        // Eat packet header
        //buf.as_ref().get_u8();

        let mut null_map = vec![0; null_map_length];
        buf.as_ref().copy_to_slice(&mut null_map);
        self.null_map = null_map;
        
        self.start_pos = 1 + null_map_length;
        self.buf = buf;
    }
    // The method must be called in column order
    fn decode_with_name<V: Value>(&mut self, name: &str) -> value::Result<V> {
        let row_data = self.get_row_data_with_name(name)?;
        match row_data {
            Some(data) => Value::from(&data.data),
            None => Ok(None),
        }
    }

    fn get_row_data_with_name(&mut self, name: &str) -> value::Result<RowPartData> {
        let mut start_pos = self.start_pos;
        for (idx, info) in self.common.columns.iter().enumerate() {
            if self.null_map[(idx + 2) / 8] & (1 << (idx + 2) as u8 % 8) > 0 {
                continue;
            }

            let (length, is_null, pos) = match info.column_type {
                ColumnType::MYSQL_TYPE_STRING
                | ColumnType::MYSQL_TYPE_VARCHAR
                | ColumnType::MYSQL_TYPE_VAR_STRING
                | ColumnType::MYSQL_TYPE_ENUM
                | ColumnType::MYSQL_TYPE_SET
                | ColumnType::MYSQL_TYPE_LONG_BLOB
                | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
                | ColumnType::MYSQL_TYPE_BLOB
                | ColumnType::MYSQL_TYPE_TINY_BLOB
                | ColumnType::MYSQL_TYPE_GEOMETRY
                | ColumnType::MYSQL_TYPE_BIT
                | ColumnType::MYSQL_TYPE_DECIMAL
                | ColumnType::MYSQL_TYPE_NEWDECIMAL => {
                    let (length, is_null, pos) = length_encode_int(&self.buf.as_ref()[start_pos..]);
                    (length, is_null, pos)
                }

                ColumnType::MYSQL_TYPE_LONGLONG => (8, false, 0),

                ColumnType::MYSQL_TYPE_LONG | ColumnType::MYSQL_TYPE_INT24 => (4, false, 0),

                ColumnType::MYSQL_TYPE_SHORT | ColumnType::MYSQL_TYPE_YEAR => (2, false, 0),

                ColumnType::MYSQL_TYPE_TINY => (1, false, 0),

                ColumnType::MYSQL_TYPE_DOUBLE => (8, false, 0),

                ColumnType::MYSQL_TYPE_FLOAT => (4, false, 0),

                ColumnType::MYSQL_TYPE_DATE
                | ColumnType::MYSQL_TYPE_DATETIME
                | ColumnType::MYSQL_TYPE_TIMESTAMP => {
                    let (length, _, pos) = length_encode_int(&self.buf.as_ref()[self.start_pos..]);
                    (length, false, pos)
                }

                ColumnType::MYSQL_TYPE_TIME => {
                    let (length, _, pos) = length_encode_int(&self.buf.as_ref()[self.start_pos..]);
                    (length, false, pos)
                }

                ColumnType::MYSQL_TYPE_NULL => (0, false, 0),

                _ => {
                    return Err(DecodeRowError::ColumnTypeNotFound(
                        info.column_type.as_ref().to_string(),
                    )
                    .into())
                }
            };

            if info.column_name != name {
                start_pos += (length + pos) as usize;
            } else {
                if is_null {
                    return Ok(None);
                }

                // Need to add packet header and null_map to returnd data
                let raw_data = &self.buf.as_ref()[start_pos + pos as usize..(start_pos + pos as usize + length as usize)];
                return Ok(Some(
                    RowPartData { 
                        data: raw_data.into(), 
                        start_idx: start_pos, 
                        part_encode_length: pos as usize, 
                        part_data_length: length as usize,
                    }
                )) 
            }
        }

        Ok(None)
    }
}


// Box has default 'static bound, use `'e` lifetime relax bound.
pub fn decode_with_name<'e, T: AsRef<[u8]>, V: Value + std::str::FromStr>(row_data: &mut RowDataTyp<T>, name: &str, is_binary: bool) -> Result<Option<V>,  Box<dyn std::error::Error + Send + Sync + 'e> > 
where 
    T: AsRef<[u8]>,
    V: Value + std::str::FromStr,
    <V as FromStr>::Err: std::error::Error + Sync + Send + 'e
{
    if is_binary {
        row_data.decode_with_name::<V>(name)
    } else {
        let new_value = row_data.decode_with_name::<String>(name)?;
        if let Some(new_value) = new_value {
            let new_value = new_value.parse::<V>()?;
            Ok(Some(new_value))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use chrono::{naive::NaiveDateTime, Duration, NaiveDate, NaiveTime};

    use super::RowDataBinary;
    use crate::{
        column::{Column, ColumnInfo},
        row::{RowData, RowDataText},
    };

    fn get_test_column_data() -> Vec<u8> {
        vec![
            0x01, 0x00, 0x00, 0x01, 0x08, 0x18, 0x00, 0x00, 0x02, 0x03, 0x64, 0x65, 0x66, 0x00,
            0x00, 0x00, 0x02, 0x49, 0x64, 0x00, 0x0c, 0x3f, 0x00, 0x15, 0x00, 0x00, 0x00, 0x08,
            0x81, 0x00, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x03, 0x03, 0x64, 0x65, 0x66, 0x00,
            0x00, 0x00, 0x04, 0x55, 0x73, 0x65, 0x72, 0x00, 0x0c, 0x08, 0x00, 0x20, 0x00, 0x00,
            0x00, 0xfd, 0x01, 0x00, 0x1f, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x04, 0x03, 0x64, 0x65,
            0x66, 0x00, 0x00, 0x00, 0x04, 0x48, 0x6f, 0x73, 0x74, 0x00, 0x0c, 0x08, 0x00, 0x40,
            0x00, 0x00, 0x00, 0xfd, 0x01, 0x00, 0x1f, 0x00, 0x00, 0x18, 0x00, 0x00, 0x05, 0x03,
            0x64, 0x65, 0x66, 0x00, 0x00, 0x00, 0x02, 0x64, 0x62, 0x00, 0x0c, 0x08, 0x00, 0x40,
            0x00, 0x00, 0x00, 0xfd, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x1d, 0x00, 0x00, 0x06, 0x03,
            0x64, 0x65, 0x66, 0x00, 0x00, 0x00, 0x07, 0x43, 0x6f, 0x6d, 0x6d, 0x61, 0x6e, 0x64,
            0x00, 0x0c, 0x08, 0x00, 0x10, 0x00, 0x00, 0x00, 0xfd, 0x01, 0x00, 0x1f, 0x00, 0x00,
            0x1a, 0x00, 0x00, 0x07, 0x03, 0x64, 0x65, 0x66, 0x00, 0x00, 0x00, 0x04, 0x54, 0x69,
            0x6d, 0x65, 0x00, 0x0c, 0x3f, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x81, 0x00, 0x00,
            0x00, 0x00, 0x1b, 0x00, 0x00, 0x08, 0x03, 0x64, 0x65, 0x66, 0x00, 0x00, 0x00, 0x05,
            0x53, 0x74, 0x61, 0x74, 0x65, 0x00, 0x0c, 0x08, 0x00, 0x1e, 0x00, 0x00, 0x00, 0xfd,
            0x00, 0x00, 0x1f, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x09, 0x03, 0x64, 0x65, 0x66, 0x00,
            0x00, 0x00, 0x04, 0x49, 0x6e, 0x66, 0x6f, 0x00, 0x0c, 0x08, 0x00, 0x64, 0x00, 0x00,
            0x00, 0xfd, 0x00, 0x00, 0x1f, 0x00, 0x00,
        ]
    }

    fn get_test_row_data() -> Vec<u8> {
        vec![
            0x3d, 0x00, 0x00, 0x0a, 0x03, 0x32, 0x31, 0x39, 0x04, 0x72, 0x6f, 0x6f, 0x74, 0x10,
            0x31, 0x30, 0x2e, 0x30, 0x2e, 0x32, 0x2e, 0x31, 0x30, 0x30, 0x3a, 0x35, 0x33, 0x31,
            0x37, 0x32, 0xfb, 0x05, 0x51, 0x75, 0x65, 0x72, 0x79, 0x01, 0x30, 0x08, 0x73, 0x74,
            0x61, 0x72, 0x74, 0x69, 0x6e, 0x67, 0x10, 0x73, 0x68, 0x6f, 0x77, 0x20, 0x70, 0x72,
            0x6f, 0x63, 0x65, 0x73, 0x73, 0x6c, 0x69, 0x73, 0x74,
        ]
    }

    fn get_test_binary_row_data() -> Vec<u8> {
        vec![
            0x4c, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0xde, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x04, 0x72, 0x6f, 0x6f, 0x74, 0x10, 0x31, 0x30, 0x2e, 0x30, 0x2e, 0x32, 0x2e,
            0x31, 0x30, 0x30, 0x3a, 0x35, 0x33, 0x32, 0x30, 0x38, 0x04, 0x74, 0x65, 0x73, 0x74,
            0x07, 0x45, 0x78, 0x65, 0x63, 0x75, 0x74, 0x65, 0x00, 0x00, 0x00, 0x00, 0x08, 0x73,
            0x74, 0x61, 0x72, 0x74, 0x69, 0x6e, 0x67, 0x10, 0x73, 0x68, 0x6f, 0x77, 0x20, 0x70,
            0x72, 0x6f, 0x63, 0x65, 0x73, 0x73, 0x6c, 0x69, 0x73, 0x74,
        ]
    }

    fn get_test_binary_row_columns_time() -> Vec<u8> {
        vec![
            0x1a, 0x00, 0x00, 0x02, 0x03, 0x64, 0x65, 0x66, 0x00, 0x00, 0x00, 0x04, 0x74, 0x69,
            0x6d, 0x65, 0x00, 0x0c, 0x3f, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x0b, 0x80, 0x00, 0x00,
            0x00, 0x00,
        ]
    }

    fn get_test_binary_row_data_time() -> Vec<u8> {
        vec![
            0x0b, 0x00, 0x00, 0x03, 0x00, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x08,
            0x15,
        ]
    }

    fn get_test_binary_row_data_datetime() -> Vec<u8> {
        vec![0x0a, 0x00, 0x00, 0x03, 0x00, 0x00, 0x07, 0xe6, 0x07, 0x08, 0x1f, 0x07, 0x10, 0x10]
    }

    fn get_test_binary_row_data_datetime_micro() -> Vec<u8> {
        vec![
            0x0e, 0x00, 0x00, 0x03, 0x00, 0x00, 0x0b, 0xd3, 0x07, 0x0c, 0x1f, 0x01, 0x02, 0x03,
            0xf3, 0xe0, 0x01, 0x00,
        ]
    }

    fn get_test_binary_row_column_null() -> Vec<u8> {
        vec![
            0x26, 0x00, 0x00, 0x02, 0x03, 0x64, 0x65, 0x66, 0x04, 0x74, 0x65, 0x73, 0x74, 0x04,
            0x75, 0x73, 0x65, 0x72, 0x04, 0x75, 0x73, 0x65, 0x72, 0x02, 0x69, 0x64, 0x02, 0x69,
            0x64, 0x0c, 0x3f, 0x00, 0x14, 0x00, 0x00, 0x00, 0x08, 0x03, 0x42, 0x00, 0x00, 0x00,
            0x36, 0x00, 0x00, 0x03, 0x03, 0x64, 0x65, 0x66, 0x04, 0x74, 0x65, 0x73, 0x74, 0x04,
            0x75, 0x73, 0x65, 0x72, 0x04, 0x75, 0x73, 0x65, 0x72, 0x0a, 0x64, 0x65, 0x6c, 0x65,
            0x74, 0x65, 0x64, 0x5f, 0x61, 0x74, 0x0a, 0x64, 0x65, 0x6c, 0x65, 0x74, 0x65, 0x64,
            0x5f, 0x61, 0x74, 0x0c, 0x3f, 0x00, 0x13, 0x00, 0x00, 0x00, 0x0c, 0x80, 0x00, 0x00,
            0x00, 0x00,
        ]
    }

    fn get_test_binary_row_data_null() -> Vec<u8> {
        vec![0x0a, 0x00, 0x00, 0x04, 0x00, 0x08, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    }

    #[test]
    fn test_decode_row() {
        let mut column_buf = &get_test_column_data()[5..];
        let columns = column_buf.decode_columns();
        assert_eq!(columns[0].column_name, "Id");
        assert_eq!(columns[1].column_name, "User");

        let row_buf = &get_test_row_data()[4..];

        let mut row = RowDataText::new(columns.into_boxed_slice().into(), row_buf);

        let res = row.decode_with_name::<String>("Id");
        assert_eq!(res.unwrap().unwrap(), "219");
        let res = row.decode_with_name::<String>("User");
        assert_eq!(res.unwrap().unwrap(), "root");
        let res = row.decode_with_name::<String>("Host");
        assert_eq!(res.unwrap().unwrap(), "10.0.2.100:53172");
        let res = row.decode_with_name::<String>("db");
        assert_eq!(res.unwrap(), None)
    }

    #[test]
    fn test_decode_binary_row() {
        let mut column_buf = &get_test_column_data()[5..];
        let columns = column_buf.decode_columns();
        let row_buf = &get_test_binary_row_data()[4..];

        let mut row = RowDataBinary::test_new(columns.into_boxed_slice().into(), row_buf);
        let res = row.decode_with_name::<String>("Command");
        assert_eq!(res.unwrap().unwrap(), "Execute");

        let res = row.decode_with_name::<String>("Info");
        assert_eq!(res.unwrap().unwrap(), "show processlist");

        let res = row.decode_with_name::<String>("Host");
        assert_eq!(res.unwrap().unwrap(), "10.0.2.100:53208");

        let res = row.decode_with_name::<u64>("Id");
        assert_eq!(res.unwrap().unwrap(), 222);
    }

    #[test]
    fn test_decode_binary_row_time() {
        let mut column_buf = &get_test_binary_row_columns_time()[..];
        let columns: Arc<[ColumnInfo]> = column_buf.decode_columns().into_boxed_slice().into();
        let row_buf = &get_test_binary_row_data_time()[4..];

        let mut row = RowDataBinary::test_new(columns.clone(), row_buf);
        let res = row.decode_with_name::<Duration>("time");
        assert_eq!(res.unwrap().unwrap().num_seconds(), -122901);

        let row_buf = &get_test_binary_row_data_datetime()[4..];
        let mut row = RowDataBinary::test_new(columns.clone(), row_buf);
        // Test decode as NaiveDateTime
        let res = row.decode_with_name::<NaiveDateTime>("time");

        let d = NaiveDate::from_ymd(2022, 8, 31);
        let dt = d.and_hms(7, 16, 16);
        assert_eq!(res.unwrap().unwrap(), dt);

        // Test decode as NaiveDate
        let res = row.decode_with_name::<NaiveDate>("time");

        let d = NaiveDate::from_ymd(2022, 8, 31);
        assert_eq!(res.unwrap().unwrap(), d);

        // Test decode as NaiveTime
        let res = row.decode_with_name::<NaiveTime>("time");

        let dt = NaiveTime::from_hms(7, 16, 16);
        assert_eq!(res.unwrap().unwrap(), dt);

        // Test decode as NaiveTime (include microseconds)
        let row_buf = &get_test_binary_row_data_datetime_micro()[4..];
        let mut row = RowDataBinary::test_new(columns.clone(), row_buf);
        let res = row.decode_with_name::<NaiveDateTime>("time");
        let d = NaiveDate::from_ymd(2003, 12, 31);
        let dt = d.and_hms_micro(01, 02, 03, 123123);
        assert_eq!(res.unwrap().unwrap(), dt);
    }

    #[test]
    fn test_decode_binary_row_null() {
        let mut column_buf = &get_test_binary_row_column_null()[..];
        let columns: Arc<[ColumnInfo]> = column_buf.decode_columns().into_boxed_slice().into();
        let row_buf = &get_test_binary_row_data_null()[4..];

        let mut row = RowDataBinary::test_new(columns.clone(), row_buf);
        let res = row.decode_with_name::<NaiveDateTime>("deleted_at");
        assert_eq!(res.unwrap(), None);
        let res = row.decode_with_name::<u64>("id");
        assert_eq!(res.unwrap().unwrap(), 1);
    }
}
