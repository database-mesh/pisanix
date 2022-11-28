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

use bytes::{Buf, BufMut, BytesMut};

use crate::{mysql_const::ColumnType, util::{ BufExt, BufMutExt, get_length }};

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub schema: Option<String>,
    pub table_name: Option<String>,
    pub column_name: String,
    pub charset: u8,
    pub column_length: u32,
    pub column_type: ColumnType,
    pub column_flag: u16,
    pub decimals: u8,
}

pub fn decode_columns<T: AsRef<[u8]>>(buf: T) -> Vec<ColumnInfo> {
    let mut buf = buf.as_ref();
    let mut columns = vec![];
    while buf.has_remaining() {
        buf.advance(4);
        columns.push(buf.decode_column());
    }

    columns
}

pub fn decode_column<T: AsRef<[u8]>>(buf: T) -> ColumnInfo {
    let mut buf = buf.as_ref();
    buf.decode_column()
}

//Remove the column of consecutive indexes in the chunk
pub fn remove_column_by_idx(columns: &mut BytesMut, chunk: &[usize]) {
    let mut pos = 0;
    let start_idx = chunk.first().unwrap_or_else(|| &0);

    for _ in 0..*start_idx {
        let length = get_length(&columns[pos..]);
        pos += length + 4;
    }

    let mut remain_part = columns.split_off(pos);
    // get column idx length
    pos = 0;
    for _ in chunk.iter() {
        println!("{:?} {:?}", remain_part.len(), pos);
        if remain_part.len() <= pos {
            continue;
        }

        let length = get_length(&remain_part[pos..]);
        pos += length + 4
    }


    remain_part.advance(pos);
    columns.unsplit(remain_part);
}

pub fn add_column_by_idx<T: AsRef<[u8]>>(columns: &mut BytesMut, idx: usize, add: T) {
    let mut pos = 0;
    for _ in 0..idx {
        let length = get_length(&columns[pos..]);
        pos += length + 4;
    }

    let remain_part = columns.split_off(pos);
    columns.put_slice(add.as_ref());
    columns.put_slice(&remain_part);
}

/// ColumnBuf trait， Inherit BufExt
/// For example:
/// let mut buf = BytesMut::new(&[0x01,0x02]);
/// buf.decode_column();
pub trait Column: BufExt {
    fn decode_columns(&mut self) -> Vec<ColumnInfo> {
        let mut columns = vec![];
        while self.has_remaining() {
            self.advance(4);
            columns.push(self.decode_column())
        }
        columns
    }

    // Decode column , see https://dev.mysql.com/doc/internals/en/com-query-response.html#packet-Protocol::ColumnDefinition41
    fn decode_column(&mut self) -> ColumnInfo {
        //Catalog
        self.skip_lenc_length();

        //Schema
        let mut schema: Option<String> = None;
        let (str_bytes, is_null) = self.get_lenc_str_bytes();
        if !is_null {
            schema = Some(String::from_utf8(str_bytes).unwrap());
        }

        //Table -- virtual table-name
        let mut table_name: Option<String> = None;
        let (str_bytes, is_null) = self.get_lenc_str_bytes();

        if !is_null {
            table_name = Some(String::from_utf8(str_bytes).unwrap());
        }

        //Org table -- physical table-name
        self.skip_lenc_length();

        //Name -- virtual column name
        let (str_bytes, _) = self.get_lenc_str_bytes();
        let column_name = String::from_utf8(str_bytes).unwrap();

        //Org name -- physical column name
        self.skip_lenc_length();

        //Next length  -- length of the following fields (always 0x0c)
        self.get_u8();

        //Character set -- is the column character set and is defined in Protocol::CharacterSet.
        let charset = self.get_u8();
        self.get_u8();

        //Column length -- maximum length of the field
        let column_length = self.get_u32_le();

        //Column type
        let column_type = ColumnType::from(self.get_u8());

        //Flags -- flags
        let column_flag = self.get_u16_le();

        //decimals -- max shown decimal digits
        let decimals = self.get_u8();

        //filter - [00] [00]
        self.get_u16_le();

        ColumnInfo {
            schema,
            table_name,
            column_name,
            charset,
            column_length,
            column_type,
            column_flag,
            decimals,
        }
    }
}

impl ColumnInfo {
    pub fn encode<T: BufMutExt>(&self, buf: &mut T) {
        // Catalog
        buf.put_lenc_int(3, false);
        buf.put_slice(b"def");

        // Schema
        if let Some(schema) = &self.schema {
            buf.put_lenc_int(schema.len() as u64, false);
            buf.put_slice(schema.as_bytes());
        } else {
            buf.put_lenc_int(0, true);
        }

        // Table name
        if let Some(name) = &self.table_name {
            buf.put_lenc_int(name.len() as u64, false);
            buf.put_slice(name.as_bytes());
        } else {
            buf.put_lenc_int(0, true);
        }

        //Org table -- physical table-name
        buf.put_lenc_int(0, true);

        //Name -- virtual column name
        buf.put_lenc_int(self.column_name.len() as u64, false);
        buf.put_slice(self.column_name.as_bytes());

        //Org name -- physical column name
        buf.put_lenc_int(0, true);

        //Next length  -- length of the following fields (always 0x0c)
        buf.put_u8(0x0c);

        //Character set -- is the column character set and is defined in Protocol::CharacterSet.
        buf.put_u8(self.charset);
        buf.put_u8(0);

        //Column length -- maximum length of the field
        buf.put_u32_le(self.column_length);

        //Column type
        buf.put_u8(self.column_type as u8);

        //Flags -- flags
        buf.put_u16_le(self.column_flag);

        //decimals -- max shown decimal digits
        buf.put_u8(self.decimals);

        //filter - [00] [00]
        buf.put_u16_le(0);
    }
}

/// Implements Column for T.
impl Column for &[u8] {}
impl Column for BytesMut {}

#[cfg(test)]
mod test {
    use bytes::BytesMut;

    use crate::{column::Column, mysql_const::ColumnType};

    use super::*;

    #[test]
    fn test_decode_encode_column_info() {
        let data = [
            0x03, 0x64, 0x65, 0x66, 0x00, 0x00, 0x00, 0x01, 0x3f, 0x00, 0x0c, 0x3f, 0x00, 0x00,
            0x00, 0x00, 0x00, 0xfd, 0x80, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut buf = BytesMut::from(&data[..]);
        let info = buf.decode_column();
        assert_eq!(info.charset, 0x3f);
        assert_eq!(info.column_type, ColumnType::MYSQL_TYPE_VAR_STRING);
        assert_eq!(info.column_flag, 0x80);
        assert_eq!(info.column_length, 0);

        let mut encode_buf = vec![];
        info.encode(&mut encode_buf);
        assert_eq!(&data[..], encode_buf)
    }

    #[test]
    fn test_remove_column_by_idx() {
        let data = [
            1, 0, 0, 0, 1,
            2, 0, 0, 0, 2, 3,
            1, 0, 0, 0, 3,
        ];

        let expect_data = [
            1, 0, 0, 0, 1,
            2, 0, 0, 0, 2, 3
        ];
        let mut buf = BytesMut::from(&data[..]);
        remove_column_by_idx(&mut buf, &[2]);
        assert_eq!(&buf[..], expect_data);
    }

    #[test]
    fn test_add_column_by_idx() {
        let data = [
            1, 0, 0, 0, 1,
            2, 0, 0, 0, 2, 3,
        ];

        let expect_data = [
            1, 0, 0, 0, 1,
            2, 0, 0, 0, 2, 3,
            1, 0, 0, 0, 3,
        ];
        let mut buf = BytesMut::from(&data[..]);
        add_column_by_idx(&mut buf, 2, vec![1, 0, 0, 0, 3]);
        assert_eq!(&buf[..], expect_data);
    }
}
