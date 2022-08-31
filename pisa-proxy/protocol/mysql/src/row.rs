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

use std::{ops::Range, sync::Arc};

use bytes::BytesMut;

use crate::{
    column::ColumnInfo,
    err::DecodeRowError,
    util::{BufExt, length_encode_int},
    value::{self, Value},
    mysql_const::*,
};

pub trait Row: BufExt {
    fn decode_row_with_idx<T: Value>(&mut self, idx: usize) -> value::Result<T> {
        for _ in 0..idx {
            let (length, ..) = self.get_lenc_int();
            self.advance(length as usize)
        }

        let (row_data, is_null) = self.get_lenc_str_bytes();
        if is_null {
            return Ok(None);
        }

        Value::from(&row_data)
    }

    fn decode_row_with_range_idx<T: Value>(
        &mut self,
        range: Range<usize>,
    ) -> Vec<value::Result<T>> {
        for _ in 1..range.start {
            let (length, ..) = self.get_lenc_int();
            self.advance(length as usize)
        }

        let mut values = vec![];

        for _ in range {
            let (row_data, is_null) = self.get_lenc_str_bytes();
            if is_null {
                values.push(Ok(None))
            } else {
                values.push(Value::from(&row_data))
            }
        }

        values
    }
}

impl Row for BytesMut {}
impl Row for &[u8] {}

pub trait RowData<T: Row> {
    fn with_buf(&mut self, buf: T);
    fn decode_with_name<V: Value>(&mut self, name: &str) -> value::Result<V>;
}

#[derive(Clone)]
pub enum RowDataTyp<T: Row + AsRef<[u8]>> {
    Text(RowDataText<T>),
    Binary(RowDataBinary<T>)
}

crate::gen_row_data!(RowDataTyp, Text(RowDataText), Binary(RowDataBinary));

#[derive(Debug, Clone)]
pub struct RowDataCommon {
    columns: Arc<[ColumnInfo]>,
    // Save consumed column idx, when call decode, buf data will removed,
    // so th data index will drift.
    consumed_idx: Vec<usize>,
}

impl RowDataCommon {
    fn new(columns: Arc<[ColumnInfo]>) -> RowDataCommon {
        RowDataCommon { columns, consumed_idx: vec![] }
    }

    fn get_idx(&mut self, name: &str) -> std::result::Result<usize, DecodeRowError> {
        let try_idx = self.columns.iter().position(|x| x.column_name == name);
        if try_idx.is_none() {
            return Err(DecodeRowError::ColumnNotFound(name.to_string()));
        }

        let idx = try_idx.unwrap();

        if self.consumed_idx.contains(&idx) {
            return Err(DecodeRowError::ColumnAlreadyConsumed(name.to_string()).into());
        }

        self.consumed_idx.push(idx);
        self.consumed_idx.sort_unstable();

        // Find all data less then idx in consumed_idx and save count.
        // The corrent idx should be minus lt_idx_count when `lt_idx_count > 0`.
        let lt_idx_count = self.consumed_idx.iter().filter(|x| *x < &idx).count();

        Ok(idx - lt_idx_count)
    }
}

// For ProtocolText::ResultsetRow
#[derive(Debug, Clone)]
pub struct RowDataText<T: Row + AsRef<[u8]>> {
    common: RowDataCommon,
    buf: T,
}

impl<T: Row + AsRef<[u8]>> RowDataText<T> {
    pub fn new(columns: Arc<[ColumnInfo]>, buf: T) -> RowDataText<T> {
        let common = RowDataCommon::new(columns);
        RowDataText { common, buf }
    }
}

impl<T: Row + AsRef<[u8]>> RowData<T> for RowDataText<T> {
    fn with_buf(&mut self, buf: T) {
        self.buf = buf;
    }
    // The method must be called in column order
    fn decode_with_name<V: Value>(&mut self, name: &str) -> value::Result<V> {
        self.buf.decode_row_with_idx::<V>(self.common.get_idx(name)?)
    }
}

#[derive(Debug, Clone)]
pub struct RowDataBinary<T: Row + AsRef<[u8]>> {
    common: RowDataCommon,
    null_map: Vec<u8>,
    buf: T,
}

impl< T: Row + AsRef<[u8]>> RowDataBinary<T> {
    pub fn new(columns: Arc<[ColumnInfo]>, mut buf: T) -> RowDataBinary<T> {
        let column_length = columns.len();
        let common = RowDataCommon::new(columns);
        // See https://dev.mysql.com/doc/internals/en/binary-protocol-resultset-row.html
        // Eat packet header
        let _ = buf.get_u8();
        let null_map_length = (column_length + 7 + 2) >> 3;
        // NULL Bitmap length: (column-count + 7 + 2) / 8

        let mut null_map = vec![0; null_map_length];
        buf.copy_to_slice(&mut null_map);

        RowDataBinary { common, null_map, buf }
    }

}

impl<T: Row + AsRef<[u8]>> RowData<T> for RowDataBinary<T> {
    fn with_buf(&mut self, buf: T) {
        self.buf = buf;
    }
    // The method must be called in column order
    fn decode_with_name<V: Value>(&mut self, name: &str) -> value::Result<V> {
        let mut start_pos = 0;

        for (idx, info) in self.common.columns.iter().enumerate() {
            if self.null_map[(idx + 2) / 8] & (1 << (idx + 2) as u8 % 8) > 0 {
                continue;
            }

            let (length, is_null, pos) = match info.column_type {
                ColumnType::MYSQL_TYPE_STRING | ColumnType::MYSQL_TYPE_VARCHAR | ColumnType::MYSQL_TYPE_VAR_STRING | ColumnType::MYSQL_TYPE_ENUM | ColumnType::MYSQL_TYPE_SET
                | ColumnType::MYSQL_TYPE_LONG_BLOB | ColumnType::MYSQL_TYPE_MEDIUM_BLOB | ColumnType::MYSQL_TYPE_BLOB | ColumnType::MYSQL_TYPE_TINY_BLOB | ColumnType::MYSQL_TYPE_GEOMETRY
                | ColumnType::MYSQL_TYPE_BIT | ColumnType::MYSQL_TYPE_DECIMAL | ColumnType::MYSQL_TYPE_NEWDECIMAL => {
                    let (length, is_null, pos) = length_encode_int(&self.buf.as_ref()[start_pos..]);
                    (length, is_null, pos)
                }

                ColumnType::MYSQL_TYPE_LONGLONG => {
                    (8, false, 0)
                }

                ColumnType::MYSQL_TYPE_LONG | ColumnType::MYSQL_TYPE_INT24 => {
                    (4, false, 0)
                }

                ColumnType::MYSQL_TYPE_SHORT | ColumnType::MYSQL_TYPE_YEAR => {
                    (2, false, 0)
                }

                ColumnType::MYSQL_TYPE_TINY => {
                    (1, false, 0)
                }

                ColumnType::MYSQL_TYPE_DOUBLE => {
                    (8, false, 0)
                }

                ColumnType::MYSQL_TYPE_FLOAT => {
                    (4, false, 0)
                }

                ColumnType::MYSQL_TYPE_DATE | ColumnType::MYSQL_TYPE_DATETIME | ColumnType::MYSQL_TYPE_TIMESTAMP => {
                    let (length, _, pos) = length_encode_int(&self.buf.as_ref()[start_pos..]);
                    (length, false, pos)
                }

                ColumnType::MYSQL_TYPE_TIME => {
                    let (length, _, pos) = length_encode_int(&self.buf.as_ref()[start_pos..]);
                    (length, false, pos)
                }

                ColumnType::MYSQL_TYPE_NULL => {
                    (0, false, 0)
                }

                _ => return Err(DecodeRowError::ColumnTypeNotFound(info.column_type.as_ref().to_string()).into())
            };

            if info.column_name != name {
                start_pos += (length + pos) as usize;
            } else {
                if is_null {
                    return Ok(None);
                }

                let row_data = &self.buf.as_ref()[(start_pos + pos as usize) .. (start_pos + pos as usize + length as usize)];
                return Value::from(row_data)
            }
        }

        return Ok(None)
    }

}


#[cfg(test)]
mod test {
    use std::sync::Arc;

    use bytes::BytesMut;
    use chrono::{Duration, naive::NaiveDateTime, NaiveDate, NaiveTime};

    use super::{Row, RowDataBinary};
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
            0x4c, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0xde,  0x00, 0x00, 0x00,   
            0x00, 0x00, 0x00, 0x00, 0x04, 0x72, 0x6f, 0x6f,  0x74, 0x10, 0x31, 0x30, 0x2e, 0x30, 0x2e, 0x32, 
            0x2e, 0x31, 0x30, 0x30, 0x3a, 0x35, 0x33, 0x32,  0x30, 0x38, 0x04, 0x74, 0x65, 0x73, 0x74, 0x07,   
            0x45, 0x78, 0x65, 0x63, 0x75, 0x74, 0x65, 0x00,  0x00, 0x00, 0x00, 0x08, 0x73, 0x74, 0x61, 0x72,   
            0x74, 0x69, 0x6e, 0x67, 0x10, 0x73, 0x68, 0x6f,  0x77, 0x20, 0x70, 0x72, 0x6f, 0x63, 0x65, 0x73,   
            0x73, 0x6c, 0x69, 0x73, 0x74
        ]
    }

    fn get_test_binary_row_columns_time() -> Vec<u8> {
        vec![
            0x1a, 0x00, 0x00, 0x02, 0x03, 0x64, 0x65, 0x66, 0x00, 0x00, 0x00, 0x04, 0x74, 0x69, 0x6d, 0x65,
            0x00, 0x0c, 0x3f, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x0b, 0x80, 0x00, 0x00, 0x00, 0x00
        ]
    } 

    fn get_test_binary_row_data_time() -> Vec<u8> {
        vec![
            0x0b, 0x00, 0x00, 0x03, 0x00, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x00,
            0x0a, 0x08, 0x15
        ]
    } 

    fn get_test_binary_row_data_datetime() -> Vec<u8> {
        vec![
            0x0a, 0x00, 0x00, 0x03, 0x00, 0x00, 0x07, 0xe6,
            0x07, 0x08, 0x1f, 0x07, 0x10, 0x10
        ]
    }

    fn get_test_binary_row_data_datetime_micro() -> Vec<u8> {
        vec![
            0x0e, 0x00, 0x00, 0x03, 0x00, 0x00, 0x0b, 0xd3, 0x07, 0x0c, 0x1f, 0x01, 0x02, 0x03, 0xf3, 0xe0,
            0x01, 0x00
        ]
    }

    #[test]
    fn test_decodec_row_with_idx() {
        let data = vec![
            0x14, 0x43, 0x6f, 0x6e, 0x6e, 0x65, 0x63, 0x74, 0x69, 0x6e, 0x67, 0x20, 0x74, 0x6f,
            0x20, 0x6d, 0x61, 0x73, 0x74, 0x65, 0x72, 0x0d, 0x31, 0x39, 0x32, 0x2e, 0x31, 0x36,
            0x38, 0x2e, 0x33, 0x33, 0x2e, 0x31, 0x30, 0x04, 0x72, 0x6f, 0x6f, 0x74, 0x04, 0x33,
            0x33, 0x30, 0x38, 0x02, 0x36, 0x30, 0x0e, 0x6c, 0x6f, 0x67, 0x2d, 0x62, 0x69, 0x6e,
            0x2e, 0x30, 0x30, 0x30, 0x30, 0x31, 0x35, 0x03, 0x31, 0x35, 0x34, 0x1e, 0x63, 0x65,
            0x6e, 0x74, 0x6f, 0x73, 0x2d, 0x64, 0x65, 0x76, 0x30, 0x30, 0x31, 0x2d, 0x72, 0x65,
            0x6c, 0x61, 0x79, 0x2d, 0x62, 0x69, 0x6e, 0x2e, 0x30, 0x30, 0x30, 0x30, 0x38, 0x33,
            0x01, 0x34, 0x0e, 0x6c, 0x6f, 0x67, 0x2d, 0x62, 0x69, 0x6e, 0x2e, 0x30, 0x30, 0x30,
            0x30, 0x31, 0x35, 0x0a, 0x43, 0x6f, 0x6e, 0x6e, 0x65, 0x63, 0x74, 0x69, 0x6e, 0x67,
            0x03, 0x59, 0x65, 0x73, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x30, 0x00, 0x01,
            0x30, 0x03, 0x31, 0x35, 0x34, 0x04, 0x31, 0x30, 0x32, 0x34, 0x04, 0x4e, 0x6f, 0x6e,
            0x65, 0x00, 0x01, 0x30, 0x02, 0x4e, 0x6f, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfb, 0x02,
            0x4e, 0x6f, 0x04, 0x31, 0x30, 0x34, 0x35, 0xac, 0x65, 0x72, 0x72, 0x6f, 0x72, 0x20,
            0x63, 0x6f, 0x6e, 0x6e, 0x65, 0x63, 0x74, 0x69, 0x6e, 0x67, 0x20, 0x74, 0x6f, 0x20,
            0x6d, 0x61, 0x73, 0x74, 0x65, 0x72, 0x20, 0x27, 0x72, 0x6f, 0x6f, 0x74, 0x40, 0x31,
            0x39, 0x32, 0x2e, 0x31, 0x36, 0x38, 0x2e, 0x33, 0x33, 0x2e, 0x31, 0x30, 0x3a, 0x33,
            0x33, 0x30, 0x38, 0x27, 0x20, 0x2d, 0x20, 0x72, 0x65, 0x74, 0x72, 0x79, 0x2d, 0x74,
            0x69, 0x6d, 0x65, 0x3a, 0x20, 0x36, 0x30, 0x20, 0x20, 0x6d, 0x61, 0x78, 0x69, 0x6d,
            0x75, 0x6d, 0x2d, 0x72, 0x65, 0x74, 0x72, 0x69, 0x65, 0x73, 0x3a, 0x20, 0x31, 0x30,
            0x30, 0x30, 0x30, 0x30, 0x20, 0x20, 0x6d, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65, 0x3a,
            0x20, 0x41, 0x63, 0x63, 0x65, 0x73, 0x73, 0x20, 0x64, 0x65, 0x6e, 0x69, 0x65, 0x64,
            0x20, 0x66, 0x6f, 0x72, 0x20, 0x75, 0x73, 0x65, 0x72, 0x20, 0x27, 0x72, 0x6f, 0x6f,
            0x74, 0x27, 0x40, 0x27, 0x31, 0x39, 0x32, 0x2e, 0x31, 0x36, 0x38, 0x2e, 0x33, 0x33,
            0x2e, 0x31, 0x30, 0x27, 0x20, 0x28, 0x75, 0x73, 0x69, 0x6e, 0x67, 0x20, 0x70, 0x61,
            0x73, 0x73, 0x77, 0x6f, 0x72, 0x64, 0x3a, 0x20, 0x59, 0x45, 0x53, 0x29, 0x01, 0x30,
            0x00, 0x00, 0x01, 0x30, 0x00, 0x00, 0x02, 0x4e, 0x6f, 0x00, 0x00, 0x00, 0x0a, 0x6f,
            0x70, 0x74, 0x69, 0x6d, 0x69, 0x73, 0x74, 0x69, 0x63, 0x01, 0x30, 0xfb, 0x36, 0x53,
            0x6c, 0x61, 0x76, 0x65, 0x20, 0x68, 0x61, 0x73, 0x20, 0x72, 0x65, 0x61, 0x64, 0x20,
            0x61, 0x6c, 0x6c, 0x20, 0x72, 0x65, 0x6c, 0x61, 0x79, 0x20, 0x6c, 0x6f, 0x67, 0x3b,
            0x20, 0x77, 0x61, 0x69, 0x74, 0x69, 0x6e, 0x67, 0x20, 0x66, 0x6f, 0x72, 0x20, 0x6d,
            0x6f, 0x72, 0x65, 0x20, 0x75, 0x70, 0x64, 0x61, 0x74, 0x65, 0x73, 0x01, 0x30, 0x01,
            0x30, 0x01, 0x30,
        ];

        let mut buf = BytesMut::from(&data[..]);
        let res = buf.decode_row_with_idx::<String>(33).unwrap();
        assert_eq!(res, Some("No".to_string()));

        let mut buf: &[u8] = &data;
        let res = buf.decode_row_with_idx::<String>(33).unwrap();
        assert_eq!(res, Some("No".to_string()));
    }

    #[test]
    fn test_decode_row() {
        let mut column_buf = &get_test_column_data()[5..];
        let columns = column_buf.decode_columns();
        println!("columns {:?}", columns);
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


        let mut row = RowDataBinary::new(columns.into_boxed_slice().into(), row_buf);
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

        let mut row = RowDataBinary::new(columns.clone(), row_buf);
        let res = row.decode_with_name::<Duration>("time");
        assert_eq!(res.unwrap().unwrap().num_seconds(), -122901);

        let row_buf = &get_test_binary_row_data_datetime()[4..];
        let mut row = RowDataBinary::new(columns.clone(), row_buf);
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
        let mut row = RowDataBinary::new(columns.clone(), row_buf);
        let res = row.decode_with_name::<NaiveDateTime>("time");
        let d = NaiveDate::from_ymd(2003, 12, 31);
        let dt = d.and_hms_micro(01, 02, 03, 123123);
        assert_eq!(res.unwrap().unwrap(), dt);
    }
}
