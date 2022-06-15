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

// Load `ast_api`, `ast_api.rs` generated by build.rs,
// Exports a `Node` enum, there are all struct variants in the `Node enum`.
// When accessing the ast tree, you need to use the `Node enum` to match.
include!(concat!(env!("OUT_DIR"), "/ast_api.rs"));

/// Used to generate custom ast tree.
///
/// As an example:
/// ``` ignore
/// #[derive(Debug, Clone)]
/// struct S {
///     a: String,
/// }
///
///impl Transformer for S {
///     fn trans(&mut self, node: &mut Node) -> Self {
///         match node {
///            Node::Value(Value::Text { span, value }) => {
///                *span = Span::new(1, 1);
///                *value = "transd".to_string();
///            }
///
///            Node::Value(Value::Num { span: _, value }) => {
///                *value = "2".to_string();
///            }
///
///            _ => {},
///        };
///
///        self.a = "11111".to_string();
///
///        self.clone()
///    }
///}
///```
pub trait Transformer {
    fn trans(&mut self, node: &mut Node) -> Self
    where
        Self: Sized;
}

///Used to visit the structure, so that the visit ast tree can be traversed.
///Every struct should implement `Visitor` trait.
pub trait Visitor {
    fn visit<V>(&mut self, _tf: &mut V) -> Self
    where
        Self: Sized + Clone,
        V: Transformer,
    {
        self.clone()
    }
}
