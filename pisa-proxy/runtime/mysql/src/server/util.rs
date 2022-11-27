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

use mysql_protocol::{column::ColumnInfo, mysql_const::ColumnType};
use strategy::sharding_rewrite::AvgChange;

pub fn filter_avg_column(change: &AvgChange, column_info: &ColumnInfo, is_added: bool) -> (Vec<u8>, Option<ColumnInfo>) {
    let avg_column = ColumnInfo {
         schema: None,
         table_name: None,
         column_name: change.ori_field.to_string(),
         charset: 0x3f,
         column_length: 0x46,
         column_type: ColumnType::MYSQL_TYPE_NEWDECIMAL,
         column_flag: 0x0080,
         decimals: 4,
    };

    if &column_info.column_name == &change.count_field || &column_info.column_name == &change.sum_field {
        if !is_added {
            let mut avg_column_buf = Vec::with_capacity(128);
            avg_column.encode(&mut avg_column_buf);
            return (avg_column_buf, Some(avg_column))
        } else {
            return (vec![], Some(avg_column))
        }
    }

    (vec![], None)
}