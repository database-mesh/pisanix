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

#[macro_export]
macro_rules! get_meta_detail {
    ($meta:ident, $($meta_typ:ident),*) => {
        paste! {
            $(let $meta_typ = $meta.[<get_ $meta_typ>]();)*
        }

    }
}

#[macro_export]
macro_rules! is_inserts {
    ($self:ident, $try_tables:ident, $inserts:ident, $fields:ident) => {
        if !$inserts.is_empty() {
            if $fields.is_empty() {
                return Err(ShardingRewriteError::FieldsIsEmpty);
            }

            return $self.change_insert_sql($try_tables, $fields, $inserts);
        }
    };
}

#[macro_export]
macro_rules! is_wheres_empty {
    ($self:ident, $wheres:ident, $strategy:ident, $try_tables:ident, $avgs:ident, $fields:ident, $orders:ident, $groups:ident) => {
        if $wheres.is_empty() {
            return Ok($self.$strategy($try_tables, $avgs, $fields, $orders, $groups));
        }
    };
}

#[macro_export]
macro_rules! calc_shard_by_wheres {
    ($self: ident, $where_shards: ident, $strategy_typ:path, $strategy:ident, $try_tables:ident, $wheres:ident, $avgs:ident, $fields:ident, $orders:ident, $groups:ident) => {
        let $where_shards = Self::find_try_where($strategy_typ, &$try_tables, $wheres)?
            .into_iter()
            .filter_map(|x| match x {
                Some((idx, num, _)) => Some((idx, num)),
                None => None,
            })
            .collect::<Vec<_>>();

        if $where_shards.is_empty() {
            return Ok($self.$strategy($try_tables, $avgs, $fields, $orders, $groups));
        }

        let expect_sum = $where_shards[0].1 as usize * $where_shards.len();
        let sum: usize = $where_shards.iter().map(|x| x.1).sum::<u64>() as usize;

        if expect_sum != sum {
            return Ok($self.$strategy($try_tables, $avgs, $fields, $orders, $groups));
        }
    };
}

#[macro_export]
macro_rules! gen_data_source {
    ($self:ident, $rule:ident, $idx:ident) => {{
        if $self.has_rw {
            DataSource::NodeGroup($rule.actual_datanodes[0].clone())
        } else {
            let ep = $rule
                .get_endpoint(&self.endpoints, Some($idx as usize))
                .ok_or_else(|| ShardingRewriteError::EndpointNotFound)?;
            DataSource::Endpoint(ep.clone())
        };
    }};
}
