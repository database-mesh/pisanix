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

pub mod base;
pub use base::*;

pub mod dml;
pub use dml::*;

pub mod tcl;
pub use tcl::*;

pub mod ddl;
pub use ddl::*;

pub mod dal;
pub use dal::*;

#[macro_use]
pub mod api;
pub use api::*;

#[derive(Debug, Clone)]
pub enum SqlStmt {
    SelectStmt(SelectStmt),
    InsertStmt(Box<InsertStmt>),
    UpdateStmt(Box<UpdateStmt>),
    DeleteStmt(Box<DeleteStmt>),
    Prepare(Box<Prepare>),
    ExecuteStmt(Box<ExecuteStmt>),
    BeginStmt(Box<BeginStmt>),
    Set(Box<SetOptValues>),
    Deallocate(Box<Deallocate>),
    ShowDatabasesStmt(Box<ShowDatabasesStmt>),
    ShowTablesStmt(Box<ShowTablesStmt>),
    ShowColumnsStmt(Box<ShowColumnsStmt>),
    ShowCreateTableStmt(Box<ShowCreateTableStmt>),
    ShowKeysStmt(Box<ShowKeysStmt>),
    ShowVariablesStmt(Box<ShowVariablesStmt>),
    ShowCreateViewStmt(Box<ShowCreateViewStmt>),
    ShowMasterStatusStmt(Box<ShowDetailsStmt>),
    ShowEnginesStmt(Box<ShowEnginesStmt>),
    ShowPluginsStmt(Box<ShowDetailsStmt>),
    ShowPrivilegesStmt(Box<ShowDetailsStmt>),
    ShowProcessListStmt(Box<ShowProcessListStmt>),
    ShowReplicasStmt(Box<ShowDetailsStmt>),
    ShowReplicaStatusStmt(Box<ShowReplicaStatusStmt>),
    ShowGrantsStmt(Box<ShowGrantsStmt>),
    ShowCreateProcedureStmt(Box<ShowCreateSpStmt>),
    ShowCreateFunctionStmt(Box<ShowCreateSpStmt>),
    ShowCreateTriggerStmt(Box<ShowCreateSpStmt>),
    ShowCreateEventStmt(Box<ShowCreateSpStmt>),
    ShowCreateUserStmt(Box<ShowCreateUserStmt>),
    ShowStatusStmt(Box<ShowStatusStmt>),
    Start(Start),
    Commit(Commit),
    Rollback(Rollback),
    Create(Create),
    CreateIndexStmt(CreateIndexStmt),
    None,
}

impl SqlStmt {
    pub fn format(&self) -> String {
        match self {
            Self::SelectStmt(stmt) => {
                stmt.format()
            }

            Self::InsertStmt(stmt) => {
                stmt.format()
            }

            Self::UpdateStmt(stmt) => {
                stmt.format()
            }

            // Implements the format method when developing sharding in the future
            _x => todo!(),
        }
    }
}

impl Visitor for SqlStmt {
    fn visit<T>(&mut self, tf: &mut T) -> Self
    where
        T: Transformer,
    {
        match self {
            Self::SelectStmt(stmt) => {
                let mut node = Node::SelectStmt(stmt);
                tf.trans(&mut node);

                let new_node = node.into_select_stmt().unwrap().visit(tf);
                Self::SelectStmt(new_node)
            }

            Self::InsertStmt(stmt) => {
                let mut node = Node::InsertStmt(stmt);
                tf.trans(&mut node);

                let new_node = node.into_insert_stmt().unwrap().visit(tf);
                Self::InsertStmt(Box::new(new_node))
            }

            Self::UpdateStmt(stmt) => {
                let mut node = Node::UpdateStmt(stmt);
                tf.trans(&mut node);

                let new_node = node.into_update_stmt().unwrap().visit(tf);
                Self::UpdateStmt(Box::new(new_node))
            }

            Self::Set(stmt) => {
                let mut node = Node::SetOptValues(stmt);
                tf.trans(&mut node);

                let new_node = node.into_set_opt_values().unwrap().visit(tf);
                Self::Set(Box::new(new_node))
            }

            x => x.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use lrpar::Span;

    use crate::ast::{api::*, base::*};

    #[test]
    fn test_visit() {
        #[derive(Debug, Clone)]
        struct S {
            a: String,
        }

        impl Transformer for S {
            fn trans(&mut self, node: &mut Node<'_>) -> bool {
                match node {
                    Node::Value(Value::Text { span, value }) => {
                        *span = Span::new(1, 1);
                        *value = "transd".to_string();
                    }

                    Node::Value(Value::Num { span: _, value }) => {
                        *value = "2".to_string();
                    }

                    _ => {}
                };

                self.a = "11111".to_string();

                false
            }
        }

        use crate::parser::Parser;
        let input = "SELECT 1+1";
        let p = Parser::new();
        let mut res = p.parse(input).unwrap();
        //let v = Value::Text{ span: Span::new(0, 0), value: "test"};

        let mut s = S { a: "a".to_string() };
        let new_v = res[0].visit(&mut s);
        println!("new value {:?}", new_v.clone());
        assert_eq!(s.a, "11111")
        //println!("new s value {:?}", s);
    }
}
