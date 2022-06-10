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

use bytes::{Buf, BytesMut};

use crate::{mysql_const::ColumnType, util::BufExt};

#[derive(Debug)]
pub struct ColumnInfo {
    pub table_name: Option<String>,
    pub column_name: String,
    pub charset: u8,
    pub column_length: u32,
    pub column_type: ColumnType,
    pub column_flag: u16,
    pub decimals: u8,
}

/// ColumnBuf trait， Inherit BufExt
pub trait ColumnBuf: BufExt {
    fn parse_column_info(&mut self) -> ColumnInfo;
}

/// Implments ColumnBuf for BytesMut.
/// For example:
/// let mut buf = BytesMut::new(&[0x01,0x02]);
/// buf.parse_column_info();
impl ColumnBuf for BytesMut {
    // Parse column info, see https://dev.mysql.com/doc/internals/en/com-query-response.html#packet-Protocol::ColumnDefinition41
    fn parse_column_info(&mut self) -> ColumnInfo {
        //Catalog
        self.get_lenc_str_bytes();

        //Schema
        self.get_lenc_str_bytes();

        //Table -- virtual table-name
        let mut table_name: Option<String> = None;
        let (str_bytes, is_null) = self.get_lenc_str_bytes();
        // Exclude 0x00
        if !is_null && !(str_bytes.len() == 1 && str_bytes[0] == 0) {
            table_name = Some(String::from_utf8(str_bytes).unwrap());
        }

        //Org table -- physical table-name
        self.get_lenc_str_bytes();

        //Name -- virtual column name
        let (str_bytes, _) = self.get_lenc_str_bytes();
        let column_name = String::from_utf8(str_bytes).unwrap();

        //Org name -- physical column name
        self.get_lenc_str_bytes();

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

        ColumnInfo {
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

#[cfg(test)]
mod test {
    use bytes::BytesMut;

    use crate::{client::column::ColumnBuf, mysql_const::ColumnType};

    #[test]
    fn test_parse_column_info() {
        let data = [
            0x03, 0x64, 0x65, 0x66, 0x00, 0x00, 0x00, 0x01, 0x3f, 0x00, 0x0c, 0x3f, 0x00, 0x00,
            0x00, 0x00, 0x00, 0xfd, 0x80, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut buf = BytesMut::from(&data[..]);
        let info = buf.parse_column_info();
        assert_eq!(info.charset, 0x3f);
        assert_eq!(info.column_type, ColumnType::MYSQL_TYPE_VAR_STRING);
        assert_eq!(info.column_flag, 0x80);
        assert_eq!(info.column_length, 0);
    }
}
