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

// Thanks to https://github.com/mysql/mysql-server/blob/8.0/sql/sql_yacc.yy

// KNOWN: INTERVAL function has a shift/reduce conflict
%expect 1
%token EQ
%token OR
%token OR_OR
%token SET_VAR
%token NEG_PREC
%token SYSTEM
%token BETWEEN
%token ROW
%token ROWS
%token EMPTY_FROM_CLAUSE
%token CONDITIONLESS_JOIN
%token SUBQUERY_AS_EXPR
%token LOWER_THEN_INTERVAL
%token LOWER_THEN_TEXT_STRING
%token LOWER_THEN_SAVEPOINT
%token LOWER_THEN_COMMA

%nonassoc 'EMPTY'
%left 'CONDITIONLESS_JOIN'
%left 'JOIN' 'INNER' 'CROSS' 'STRAIGHT_JOIN' 'NATURAL' 'LEFT' 'RIGHT' 'ON' 'USING'
%left 'SET_VAR'
%left OR OR_OR
%left 'XOR'
%left 'AND' 'AND_AND'
%left 'BETWEEN'
%left 'CASE' 'WHEN' 'THEN' 'ELSE'
%left 'EQ' 'ASSIGN_EQ' 'GT' 'GE' 'LT' 'LE' 'NE' 'LIKE' 'SOUNDS' 'REGEXP' 'IS' 'IN'
%left 'ESCAPE'
%left '|'
%left '&'
%left 'SHIFT_LEFT' 'SHIFT_RIGHT'
%left '-' '+' 
%left '*' '/' '%' 'DIV' 'MOD'
%left '^'
%left 'NEG_PREC' '~'
%right 'NOT' 'NOT2'
%right 'BINARY' 'COLLATE'
%nonassoc 'LOWER_THEN_INTERVAL'
%nonassoc 'INTERVAL'
%left 'SUBQUERY_AS_EXPR'
%left '(' ')'
%right 'MEMBER'
%nonassoc 'LOWER_THEN_TEXT_STRING'
%nonassoc 'TEXT_STRING'
%nonassoc 'QUICK'
%nonassoc LOWER_THEN_SAVEPOINT
%nonassoc SAVEPOINT
%nonassoc LOWER_THEN_COMMA
%nonassoc ','


%left 'EMPTY_FROM_CLAUSE'
%right 'INTO'


%start StartRule

%%
StartRule -> Vec<SqlStmt>:
  multi_sql_stmt
  {
    $1
  }
  ;

multi_sql_stmt -> Vec<SqlStmt>:
    sql_stmt 
    { 
      vec![$1] 
    }
  | multi_sql_stmt ';' sql_stmt 
    {
      $1.push($3);
      $1
    } 
  ;

sql_stmt -> SqlStmt:
    end_of_input  { $1 }
  | commit              { SqlStmt::Commit($1) }
  | rollback_stmt           { SqlStmt::Rollback($1) }
  | select_stmt   { SqlStmt::SelectStmt($1) }
  | insert_stmt   { SqlStmt::InsertStmt($1) }
  | update_stmt   { SqlStmt::UpdateStmt($1) }
  | delete_stmt   { SqlStmt::DeleteStmt($1) }
  | prepare       { SqlStmt::Prepare($1) }
  | execute       { SqlStmt::ExecuteStmt($1) }
  | begin_stmt	  { SqlStmt::BeginStmt($1) }
  | set           { SqlStmt::Set($1) }
  | deallocate    { SqlStmt::Deallocate($1) }
  | show_databases_stmt { SqlStmt::ShowDatabasesStmt($1) }
  | show_tables_stmt    { SqlStmt::ShowTablesStmt($1) }
  | show_columns_stmt   { SqlStmt::ShowColumnsStmt($1) }
  | show_create_table_stmt  { SqlStmt::ShowCreateTableStmt($1) }
  | show_keys_stmt     { SqlStmt::ShowKeysStmt($1) }
  | show_variables_stmt     { SqlStmt::ShowVariablesStmt($1) }
  | show_create_view_stmt     { SqlStmt::ShowCreateViewStmt($1) }
  | show_master_status_stmt { SqlStmt::ShowMasterStatusStmt($1) }
  | show_engines_stmt { SqlStmt::ShowEnginesStmt($1) }
  | show_plugins_stmt { SqlStmt::ShowPluginsStmt($1) }
  | show_privileges_stmt { SqlStmt::ShowPrivilegesStmt($1) }
  | show_processlist_stmt { SqlStmt::ShowProcessListStmt($1) }
  | show_replicas_stmt { SqlStmt::ShowReplicasStmt($1) }
  | show_replica_status_stmt { SqlStmt::ShowReplicaStatusStmt($1) }
  | show_grants_stmt { SqlStmt::ShowGrantsStmt($1) }
  | show_create_procedure_stmt { SqlStmt::ShowCreateProcedureStmt($1) }
  | show_create_function_stmt { SqlStmt::ShowCreateFunctionStmt($1) }
  | show_create_trigger_stmt { SqlStmt::ShowCreateTriggerStmt($1) }
  | show_create_event_stmt { SqlStmt::ShowCreateEventStmt($1) }
  | show_create_user_stmt { SqlStmt::ShowCreateUserStmt($1) }
  | show_status_stmt     { SqlStmt::ShowStatusStmt($1) }
  | start               { SqlStmt::Start($1) }
  | create        { SqlStmt::Create($1) }
  | create_index_stmt  { SqlStmt::CreateIndexStmt($1) }
  
  ;

end_of_input -> SqlStmt:
    {
      SqlStmt::None
    }
  ;

select_stmt -> SelectStmt:
    query_expression
    {
      $1
    }
  | query_expression locking_clause_list
    {
      match $1 {
        SelectStmt::Query(mut q) => {
          q.lock_clauses = $2;
          SelectStmt::Query(q)
        }

        _ => $1
      }
    }
  | query_expression_parens
    {
      $1
    }
  | select_stmt_with_into
    {
      $1
    }
  ;

select_stmt_with_into -> SelectStmt:
    '(' select_stmt_with_into ')'
    {
      $2
    }
  | query_expression into_clause
    {
      match $1 {
        SelectStmt::Query(mut q) => {
          q.into_clause = Some($2);
          SelectStmt::Query(q)
        }

        _ => $1
      }
    }
  | query_expression into_clause locking_clause_list
    {
      match $1 {
        SelectStmt::Query(mut q) => {
          q.into_clause = Some($2);
          q.lock_clauses = $3;
          SelectStmt::Query(q)
        }

        _ => $1
      }
    }
  | query_expression locking_clause_list into_clause
    {
      match $1 {
        SelectStmt::Query(mut q) => {
          q.lock_clauses = $2;
          q.into_clause = Some($3);
          SelectStmt::Query(q)
        }

        _ => $1
      }
    }
  | query_expression_parens into_clause
    {
      match $1 {
        SelectStmt::SubQuery(mut q) => {
          q.into_clause = Some($2);
          SelectStmt::SubQuery(q)
        }

        _ => $1
      }
    }
  ;

query_expression -> SelectStmt:
    query_expression_body opt_order_clause opt_limit_clause
    {
      match $1 {
        SelectStmt::Query(mut q) => {
          q.order_clause = $2;
          q.limit_clause = $3;
          SelectStmt::Query(q)
        }

        _ => $1
      }
    }
  | with_clause query_expression_body opt_order_clause opt_limit_clause
    {
      let newq = match $2 {
        SelectStmt::Query(mut q) => {
          q.order_clause = $3;
          q.limit_clause = $4;
          SelectStmt::Query(q)
        }
        _ => $2
      };

      SelectStmt::With(
        Box::new(WithQuery {
          with_clause: $1,
          expr_body: newq,
        })
      )
    }
  | query_expression_parens order_clause opt_limit_clause
    {
      match $1 {
        SelectStmt::SubQuery(mut q) => {
          q.order_clause = Some($2);
          q.limit_clause = $3;
          SelectStmt::SubQuery(q)
        }

        _ => $1
      }
    }
  | with_clause query_expression_parens order_clause opt_limit_clause
    {
      let newq = match $2 {
        SelectStmt::SubQuery(mut q) => {
          q.order_clause = Some($3);
          q.limit_clause = $4;
          SelectStmt::SubQuery(q)
        }

        _ => $2
      };

      SelectStmt::With(
        Box::new(WithQuery {
          with_clause: $1,
          expr_body: newq,
        })
      )
    }
  | query_expression_parens limit_clause
    {
      match $1 {
        SelectStmt::SubQuery(mut q) => {
          q.limit_clause = Some($2);
          SelectStmt::SubQuery(q)
        }

        _ => $1
      }
    }
  | with_clause query_expression_parens limit_clause
    {
      let newq = match $2 {
        SelectStmt::SubQuery(mut q) => {
          q.limit_clause = Some($3);
          SelectStmt::SubQuery(q)
        }

        _ => $2
      };

      SelectStmt::With(
        Box::new(WithQuery {
          with_clause: $1,
          expr_body: newq,
        })
      )
    }
  | with_clause query_expression_parens
    {
      SelectStmt::With(
        Box::new(WithQuery {
          with_clause: $1,
          expr_body: $2,
        })
      )
    }
  ;

query_expression_body -> SelectStmt:
    query_primary
    {
      $1
    }
  | query_expression_body 'UNION' union_option query_primary
    {
      match $1 {
        SelectStmt::Query(mut q) => {
          q.union_opt = $3;
          q.union_query = Some(Box::new($4));
          SelectStmt::Query(q)
        }
        _ => $1
      }
    }
  | query_expression_parens 'UNION' union_option query_primary
    {
      match $1 {
        SelectStmt::SubQuery(mut q) => {
          q.union_opt = $3;
          q.union_query = Some(Box::new($4));
          SelectStmt::SubQuery(q)
        }
        _ => $1
      }
    }
  | query_expression_body 'UNION' union_option query_expression_parens
    {
      match $1 {
        SelectStmt::Query(mut q) => {
          q.union_opt = $3;
          q.union_query = Some(Box::new($4));
          SelectStmt::Query(q)
        }
        _ => $1
      }
    }
  | query_expression_parens 'UNION' union_option query_expression_parens
    {
      match $1 {
        SelectStmt::SubQuery(mut q) => {
          q.union_opt = $3;
          q.union_query = Some(Box::new($4));
          SelectStmt::SubQuery(q)
        }
        _ => $1
      }
    }
  ;

query_expression_parens -> SelectStmt:
    '(' query_expression_parens ')'
    { 
    	SelectStmt::SubQuery(Box::new(
		SubQuery {
			span: $span,
			query: $2,
			into_clause: None,
			union_opt: None,
			union_query: None,
			order_clause: None,
			limit_clause: None,
		}

	))
    }
  | '(' query_expression ')'
    { 
    	SelectStmt::SubQuery(Box::new(
		SubQuery {
			span: $span,
			query: $2,
			into_clause: None,
			union_opt: None,
			union_query: None,
			order_clause: None,
			limit_clause: None,
		}

	))
    }
  | '(' query_expression locking_clause_list ')'
    {
      let newq = match $2 {
        SelectStmt::Query(mut q) => {
      	  q.lock_clauses = $3;
          SelectStmt::Query(q)
        },

        _ => $2
      };

      SelectStmt::SubQuery(Box::new(
	SubQuery {
		span: $span,
		query: newq,
		into_clause: None,
		union_opt: None,
		union_query: None,
		order_clause: None,
		limit_clause: None,
	}
      ))
    }
  ;

query_primary -> SelectStmt:
    query_specification
    {
	    $1	
    }
  | table_value_constructor
    {
      SelectStmt::ValueConstructor($1)
    }
  | 'TABLE' table_ident
    {
      SelectStmt::ExplicitTable($2)
    }
  ;

query_specification -> SelectStmt:
    "SELECT" 
    select_options
    select_items 
    into_clause 
    opt_from_clause 
    opt_where_clause 
    opt_group_clause 
    opt_having_clause 
    opt_window_clause
    {
      SelectStmt::Query(Box::new(Query {
        span: $span,
        opts: $2,
        items: $3,
        into_clause: Some($4),
        from_clause: $5,
        where_clause: $6,
        group_clause: $7,
        having_clause: $8,
        window_clause: $9,
        limit_clause: None,
        lock_clauses: vec![],
        order_clause: None,
        union_opt: None,
        union_query: None,
      }))
    }
  | "SELECT"
    select_options 
    select_items 
    opt_from_clause 
    opt_where_clause 
    opt_group_clause 
    opt_having_clause 
    opt_window_clause
    {
      SelectStmt::Query(Box::new(Query {
        span: $span,
        opts: $2,
        items: $3,
        from_clause: $4,
        where_clause: $5,
        group_clause: $6,
        having_clause: $7,
        window_clause: $8,
        limit_clause: None,
        lock_clauses: vec![],
        into_clause: None,
        order_clause: None,
        union_opt: None,
        union_query: None,
      }))
    }
  ;

opt_from_clause -> Option<FromClause>:
    /* empty. */ %prec 'EMPTY_FROM_CLAUSE'
    { 
      None
    }
  | 'FROM' from_tables
    {
      Some($2)
    }
  ;

from_tables -> FromClause:
    'DUAL'
    {
      FromClause::Dual
    }
  | table_reference_list
    {
      FromClause::TableRefs($1)
    }
  ;

table_reference_list -> Vec<TableRef>:
    table_reference
      {
        vec![$1]
      }
    | table_reference_list ',' table_reference
      {
        $1.push($3);
        $1
      }
  ;


table_value_constructor -> Vec<Vec<Expr>>:
    "VALUES" values_row_list
      {
        $2
      }
  ;

explicit_table -> TableIdent:
    "TABLE" table_ident
    {
      $2
    }
  ;

select_options -> Vec<QuerySpecOpt>:
   /* empty*/
    {
	    vec![]
    }
  | select_option_list
    {
	    $1
    }
  ;

select_option_list -> Vec<QuerySpecOpt>:
    select_option_list select_option
    {
	    $1.push($2);
	    $1
    }
  | select_option
    {
	    vec![$1]
    }
  ;

select_option -> QuerySpecOpt:
    query_spec_option
    {
      $1
    }
  //| "SQL_NO_CACHE"
  //  {
  //    $span
  //  }
  ;

locking_clause_list -> Vec<LockClause>:
    locking_clause_list locking_clause
    {
      $1.push($2);
      $1
    }
  | locking_clause
    {
      vec![$1]
    }
  ;

locking_clause -> LockClause:
    'FOR' 'UPDATE' opt_locked_row_action
    {
      LockClause {
        lock_strength: LockStrength::Update,
        locked_row_action: $3,
        tables: vec![],
      }
    }
  | 'FOR' 'SHARE' opt_locked_row_action
    {
      LockClause {
        lock_strength: LockStrength::Share,
        locked_row_action: $3,
        tables: vec![],
      }
    }
  | "FOR" 'UPDATE' 'OF' table_alias_ref_list opt_locked_row_action
    {
      LockClause {
        lock_strength: LockStrength::Update,
        tables: $4,
        locked_row_action: $5,
      }
    }
  | "FOR" 'SHARE' 'OF' table_alias_ref_list opt_locked_row_action
    { 
      LockClause {
        lock_strength: LockStrength::Share,
        tables: $4,
        locked_row_action: $5,
      }
    }
  | "LOCK" "IN" "SHARE" "MODE"
    {
      LockClause {
        lock_strength: LockStrength::InShare,
        tables: vec![],
        locked_row_action: None,
      }
    }
  ;

opt_locked_row_action -> Option<LockedRowAction>:
    /* Empty */ 
    { 
      None
    }
  | locked_row_action
    {
      Some($1)
    }
  ;

locked_row_action -> LockedRowAction:
    "SKIP" "LOCKED" 
    { 
      LockedRowAction::SkipLocked
    }
  | "NOWAIT" 
    { 
      LockedRowAction::NoWait
    }
  ;

select_items -> Items:
    select_items ',' select_item
    {
      if let Items::Items(mut items) = $1 {
        items.push($3);
        Items::Items(items)
      } else {
        $1
      }
    }
  | select_item
    {
      Items::Items(vec![$1])
    }
  | '*'
    {
      Items::Wild( ItemWild { span: $span } )
    }
  ;

select_item -> Item:
    table_wild 
    { 
      Item::TableWild($1)
    }
  | expr select_alias
    {
        Item::ItemExpr(Box::new(ItemExpr {
          span: $span,
          expr: $1,
          alias_name: $2
        }))
    }
  ;

select_alias -> Option<String>:
    /* empty */ 
    { 
      None
    }
  | 'AS' ident 
    { 
      Some($2.0)
    }
  | 'AS' 'TEXT_STRING'
    { 
      Some(String::from($lexer.span_str($2.as_ref().unwrap().span())))
    }
  | ident 
    {
      Some($1.0) 
    }
  | 'TEXT_STRING' 
    { 
      Some(String::from($lexer.span_str($1.as_ref().unwrap().span())))
    }
  ;

optional_braces -> Option<String>:
    /* empty */
    {
      None
    }
  | '(' ')'
    {
      Some(String::from("()"))
    }
  ;

expr -> Expr:
    expr or expr %prec 'OR_OR'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::OR
      }
    }
  | expr 'XOR' expr %prec 'XOR'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::XOR
      }
    }
  | expr 'AND' expr %prec 'AND'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::AND
      }
    }
  | 'NOT' expr %prec 'NOT'
    {
      Expr::UnaryOperationExpr {
        span: $span,
        expr: Box::new($2),
        operator: Op::NOT
      }
    }
  | bool_pri 'IS' 'TRUE' %prec 'IS'
    {
      Expr::IsExpr {
        span: $span,
        operator: Op::TRUE,
        expr: Box::new($1),
        is_not: false
      }
    }
  | bool_pri 'IS' not 'TRUE' %prec 'IS'
    {
      Expr::IsExpr {
        span: $span,
        operator: Op::TRUE,
        expr: Box::new($1),
        is_not: true
      }
    }
  | bool_pri 'IS' 'FALSE' %prec 'IS'
    {
      Expr::IsExpr {
        span: $span,
        operator: Op::FALSE,
        expr: Box::new($1),
        is_not: false
      }
    }
  | bool_pri 'IS' not 'FALSE' %prec 'IS'
    {
      Expr::IsExpr {
        span: $span,
        operator: Op::FALSE,
        expr: Box::new($1),
        is_not: true
      }
    }
  | bool_pri 'IS' 'UNKNOWN' %prec 'IS'
    {
      Expr::IsExpr {
        span: $span,
        operator: Op::UNKNOWN,
        expr: Box::new($1),
        is_not: false
      }
    }
  | bool_pri 'IS' not 'UNKNOWN' %prec 'IS'
    {
      Expr::IsExpr {
        span: $span,
        operator: Op::UNKNOWN,
        expr: Box::new($1),
        is_not: true
      }
    }
  | bool_pri %prec 'SET_VAR'
    {
      $1
    }
  ;

bool_pri -> Expr:
    bool_pri 'IS' 'NULL' %prec 'IS'
    {
      Expr::IsExpr {
        span: $span,
        operator: Op::NULL,
        expr: Box::new($1),
        is_not: false
      }
    }
  | bool_pri 'IS' not 'NULL' %prec 'IS'
    {
      Expr::IsExpr {
        span: $span,
        operator: Op::NULL,
        expr: Box::new($1),
        is_not: true
      }
    }
  | bool_pri comp_op predicate
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: $2
      }
    }
    // grmtool compatibility for 'COMP_OP'
  | bool_pri comp_op all_or_any table_subquery %prec 'EQ'
    {
      Expr::CompSubQueryExpr(Box::new(CompSubQueryExpr {
        span: $span,
        expr: Box::new($1),
        operator: $2,
        opt: $3,
        subquery: $4
      }))
    }
  | predicate
    {
      $1
    }
  ;

predicate -> Expr:
    bit_expr 'IN' table_subquery
    {
      Expr::InExpr {
        span: $span,
        is_not: false,
        expr: Box::new($1),
        exprs: vec![Expr::SubQueryExpr(Box::new($3))]
      }
    }
  | bit_expr not 'IN' table_subquery
    {
      Expr::InExpr {
        span: $span,
        is_not: true,
        expr: Box::new($1),
        exprs: vec![Expr::SubQueryExpr(Box::new($4))]
      }
    }
  | bit_expr 'IN' '(' expr ')'
    {
      Expr::InExpr {
        span: $span,
        is_not: false,
        expr: Box::new($1),
        exprs: vec![$4]
      }
    }
  | bit_expr 'IN' '(' expr ',' expr_list ')'
    {
      let mut exprs = vec![$4];
      exprs.extend($6);
      Expr::InExpr {
        span: $span,
        is_not: false,
        expr: Box::new($1),
        exprs,
      }
    }
  | bit_expr not 'IN' '(' expr ')'
    {
      Expr::InExpr {
        span: $span,
        is_not: true,
        expr: Box::new($1),
        exprs: vec![$5] 
      }
    }
  | bit_expr not 'IN' '(' expr ',' expr_list ')'
    {
      let mut exprs = vec![$5];
      exprs.extend($7);
      Expr::InExpr {
        span: $span,
        is_not: true,
        expr: Box::new($1),
        exprs,
      }
    }
  | bit_expr 'MEMBER' 'OF' '(' simple_expr ')' %prec 'MEMBER'
    {
      Expr::MemberOfExpr {
        span: $span,
        expr: Box::new($1),
        of_expr: Box::new($5)
      }
    }
  | bit_expr 'BETWEEN' bit_expr 'AND' predicate
    {
      Expr::BetweenExpr {
        span: $span,
        is_not: false,
        expr: Box::new($1),
        left: Box::new($3),
        right: Box::new($5)
      }
    }
  | bit_expr not 'BETWEEN' bit_expr 'AND' predicate
    {
      Expr::BetweenExpr {
        span: $span,
        is_not: true,
        expr: Box::new($1),
        left: Box::new($4),
        right: Box::new($6)
      }
    }
  | bit_expr 'SOUNDS' 'LIKE' bit_expr
    {
      Expr::SoundsExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($4)
      }
    }
  | bit_expr 'LIKE' simple_expr
    {
      Expr::LikeExpr {
        span: $span,
        expr: Box::new($1),
        pattern_expr: Box::new($3),
        escape_expr: None,
        is_not: false,
      }
    }
  | bit_expr 'LIKE' simple_expr 'ESCAPE' simple_expr %prec 'ESCAPE'
    {
      Expr::LikeExpr {
        span: $span,
        expr: Box::new($1),
        pattern_expr: Box::new($3),
        escape_expr: Some(Box::new($5)),
        is_not: false,
      }
    }
  | bit_expr not 'LIKE' simple_expr
    {
      Expr::LikeExpr {
        span: $span,
        expr: Box::new($1),
        pattern_expr: Box::new($4),
        escape_expr: None,
        is_not: true,
      }
    }
  | bit_expr not 'LIKE' simple_expr 'ESCAPE' simple_expr %prec 'ESCAPE'
    {
      Expr::LikeExpr {
        span: $span,
        expr: Box::new($1),
        pattern_expr: Box::new($4),
        escape_expr: Some(Box::new($6)),
        is_not: true,
      }
    }
  | bit_expr 'REGEXP' bit_expr
    {
      Expr::RegexpExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        is_not: false,
      }
    }
  | bit_expr not 'REGEXP' bit_expr
    {
      Expr::RegexpExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($4),
        is_not: true,
      }
    }
  | bit_expr %prec 'SET_VAR'
    {
      $1
    }
  ;

bit_expr -> Expr:
    bit_expr '|' bit_expr %prec '|'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::OR
      }
    }
  | bit_expr '&' bit_expr %prec '&'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::AND
      }
    }
  | bit_expr 'SHIFT_LEFT' bit_expr %prec 'SHIFT_LEFT'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::ShiftLeft
      }
    }
  | bit_expr 'SHIFT_RIGHT' bit_expr %prec 'SHIFT_RIGHT'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: 	Box::new($1),
        right: Box::new($3),
        operator: Op::ShiftRight
      }
    }
  | bit_expr '+' bit_expr %prec '+'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::PLUS
      }
    }
  | bit_expr '-' bit_expr %prec '-'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::MINUS
      }
    }
  | bit_expr '+' 'INTERVAL' expr interval %prec '+'
    {
      Expr::FuncCallExpr {
        span: $span,
        name: String::from("DATE_ADD"),
        args: vec![$1, $4, $5]
      }
    }
  | bit_expr '-' 'INTERVAL' expr interval %prec '-'
    {
      Expr::FuncCallExpr {
        span: $span,
        name: String::from("DATE_SUB"),
        args: vec![$1, $4, $5]
      }
    }
  | bit_expr '*' bit_expr %prec '*'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::MUL
      }
    }
  | bit_expr '/' bit_expr %prec '/'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::DIV
      }
    }
  | bit_expr '%' bit_expr %prec '%'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::MOD
      }
    }
  | bit_expr 'DIV' bit_expr %prec 'DIV'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::DIV
      }
    }
  | bit_expr 'MOD' bit_expr %prec 'MOD'
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::MOD
      }
    }
  | bit_expr '^' bit_expr
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::XOR
      }
    }
  | simple_expr %prec 'SET_VAR'
    {
      $1
    }
  ;

or -> Op:
    OR
    {
      Op::OR
    }
  | OR_OR
    {
      Op::OR
    }
  ;

and -> Span:
    'AND' 
    {
      $span
    }
 | 'AND_AND'
   {
     $span
   }
 ;

not -> Span:
    'NOT'
    {
      $span
    }
  ;

not2 -> Span:
    '!' { $span }
  | 'NOT2' { $span }
  ;

comp_op -> Op:
    'EQ'     { Op::EQ }
  | 'ASSIGN_EQ' { Op::AssignEQ }
  | 'GE' { Op::GE }
  | 'GT' { Op::GT }
  | 'LE' { Op::LE }
  | 'LT' { Op::LT }
  | 'NE' { Op::NE }
  ;

all_or_any -> String:
    'ALL'  { String::from("ALL")  }
  | 'ANY'  { String::from("ANY")  }
  | 'SOME' { String::from("SOME") }
  ;

simple_expr -> Expr:
    simple_ident
    {
      Expr::SimpleIdentExpr($1)
    }
  | func_call_keyword
    {
      Expr::Ori(String::from($lexer.span_str($1)))
    }
  | func_call_nonkeyword
    {
      Expr::Ori(String::from($lexer.span_str($1)))
    }
  | func_call_generic
    {
      Expr::Ori(String::from($lexer.span_str($1)))
    }
  | func_call_conflict
    {
      Expr::Ori(String::from($lexer.span_str($1)))
    }
  | simple_expr 'COLLATE' ident_or_text %prec 'NEG_PREC'
    {
      Expr::CollateExpr {
        span: $span,
        expr: Box::new($1),
        name: $3
      }
    }
  | literal_or_null
    {
      Expr::LiteralExpr($1)  
    }
  | param_marker
    {
      Expr::ParamMarkerExpr
    }
  | variable
    {
      Expr::VarExpr($1)
    }
  | set_func_specification
    {
      Expr::SetFuncSpecExpr(Box::new($1))
    }
  | window_func_call
    {
      Expr::Ori(String::from($lexer.span_str($1)))
    }
  | simple_expr OR_OR simple_expr
    {
      Expr::BinaryOperationExpr {
        span: $span,
        left: Box::new($1),
        right: Box::new($3),
        operator: Op::OR,
      }
    }
  | '+' simple_expr %prec 'NEG_PREC'
    {
      Expr::UnaryOperationExpr {
        expr: Box::new($2),
        span: $span,
        operator: Op::PLUS
      }
    }
  | '-' simple_expr %prec 'NEG_PREC'
    {
      Expr::UnaryOperationExpr {
        span: $span,
        expr: Box::new($2),
        operator: Op::MINUS
      }
    }
  | '~' simple_expr %prec 'NEG_PREC'
    {
      Expr::UnaryOperationExpr {
        span: $span,
        expr: Box::new($2),
        operator: Op::NEG
      }
    }
  | not2 simple_expr %prec 'NEG_PREC'
    {
      Expr::UnaryOperationExpr {
        span: $span,
        expr: Box::new($2),
        operator: Op::NEG
      }
    }
  | row_subquery
    {
      Expr::SubQueryExpr(Box::new($1))
    }
  | '(' expr ')'  //%prec 'LOWER_THEN_INTERVAL'
    { 
      Expr::RowExpr(vec![$2])
    }
  | '(' expr_list ',' expr ')' 
    {
      $2.push($4);
      
      Expr::RowExpr($2)
    }
  | 'ROW' '(' expr_list ',' expr ')' 
    {
      $3.push($5);
      Expr::RowExpr($3)
    }
  | 'EXISTS' table_subquery
    {
      Expr::ExistsSubQuery(Box::new($2))
    }
  | '{' ident expr '}'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  | 'MATCH' ident_list_arg 'AGAINST' '(' bit_expr fulltext_options ')'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  | 'BINARY' simple_expr %prec 'NEG_PREC'
    {
      Expr::BinaryExpr(Box::new($2))
    }
  | 'CAST' '(' expr AS cast_type opt_array_cast ')'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  | 'CAST' '(' expr 'AT' 'LOCAL' 'AS' cast_type opt_array_cast ')'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  | 'CAST' '(' expr 'AT' 'TIME' 'ZONE' opt_interval 'TEXT_STRING' 'AS' 'DATETIME' type_datetime_precision ')'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  | 'CASE' opt_expr when_list opt_else 'END'
    {
      Expr::CaseExpr {
        span: $span,
        expr: Box::new($2),
        when_exprs: $3,
        else_expr: Box::new($4)
      }
    }
  | 'CONVERT' '(' expr ',' cast_type ')'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  | 'CONVERT' '(' expr 'USING' charset_name ')'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  | 'DEFAULT' '(' simple_ident ')'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  | 'VALUES' '(' simple_ident_nospvar ')'
    {
      Expr::ValuesExpr($3)
    }
  | 'INTERVAL' expr interval '+' expr //%prec 'INTERVAL'
    {
      Expr::FuncCallExpr {
        span: $span,
        name: String::from("DATE_ADD"),
        args: vec![$2, $3, $5]
      }
    }
  | simple_ident 'JSON_SEPARATOR' 'TEXT_STRING'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
   | simple_ident 'JSON_UNQUOTED_SEPARATOR' 'TEXT_STRING'
    {
      Expr::Ori(String::from($lexer.span_str($span)))
    }
  ;

opt_array_cast -> Option<String>:
  /* empty */ { None }
  | 'ARRAY' { Some(String::from("ARRAY")) }
  ;

func_call_keyword -> Span:
    char '(' expr_list ')'
    {
      $span
    }
  | char '(' expr_list 'USING' charset_name ')'
    {
      $span
    }
  | 'CURRENT_USER' optional_braces
    {
      $span
    }
  | 'DATE' '(' expr ')'
    {
      $span
    }
  | 'DAY' '(' expr ')'
    {
      $span
    }
  | 'HOUR' '(' expr ')'
    {
      $span
    }
  | 'INSERT' '(' expr ',' expr ',' expr ',' expr ')'
    {
      $span
    }
  | 'INTERVAL' '(' expr ',' expr ')'  
    {
      $span
    }
  | 'INTERVAL' '(' expr ',' expr ',' expr_list ')' 
    {
      $span
    }
  | 'JSON_VALUE' '(' simple_expr ',' text_literal opt_returning_type opt_on_empty_or_error ')'
    {
      $span
    }
  | 'LEFT' '(' expr ',' expr ')'
    {
      $span
    }
  | 'MINUTE' '(' expr ')'
    {
      $span
    }
  | 'MONTH' '(' expr ')'
    {
      $span
    }
  | 'RIGHT' '(' expr ',' expr ')'
    {
      $span
    }
  | 'SECOND' '(' expr ')'
    {
      $span
    }
  | 'TIME' '(' expr ')'
    {
      $span
    }
  | 'TIMESTAMP' '(' expr ')'
    {
      $span
    }
  | 'TIMESTAMP' '(' expr ',' expr ')'
    {
      $span
    }
  | 'DATE' 'TEXT_STRING'
    {
      $span
    }
  | 'TIME' 'TEXT_STRING'
    {
      $span
    }
  | 'TIMESTAMP' 'TEXT_STRING'
    {
      $span
    }
  | 'TRIM' '(' expr ')'
    {
      $span
    }
  | 'TRIM' '(' 'LEADING' expr 'FROM' expr ')'
    {
      $span
    }
  | 'TRIM' '(' 'TRAILING' expr 'FROM' expr ')'
    {
      $span
    }
  | 'TRIM' '(' 'BOTH' expr 'FROM' expr ')'
    {
      $span
    }
  | 'TRIM' '(' 'LEADING' 'FROM' expr ')'
    {
      $span
    }
  | 'TRIM' '(' 'TRAILING' 'FROM' expr ')'
    {
      $span
    }
  | 'TRIM' '(' BOTH 'FROM' expr ')'
    {
      $span
    }
  | 'TRIM' '(' expr 'FROM' expr ')'
    {
      $span
    }
  | 'USER' '(' ')'
    {
      $span
    }
  | 'YEAR' '(' expr ')'
    {
      $span
    }
  | 'PASSWORD' '(' opt_expr_list ')'
    {
      $span
    }

  ;

func_call_nonkeyword -> Span:
    'ADDDATE' '(' expr ',' expr ')'
    {
      $span
    }
  | 'ADDDATE' '(' expr ',' 'INTERVAL' expr interval ')'
    {
      $span
    }
  | 'CURDATE' optional_braces
    {
      $span
    }
  | 'CURTIME' func_datetime_precision
    {
      $span
    }
  | 'DATE_ADD' '(' expr ',' 'INTERVAL' expr interval ')'
    {
      $span
    }
  | 'DATE_SUB' '(' expr ',' 'INTERVAL' expr interval ')'
    {
      $span
    }
  | 'EXTRACT' '(' interval FROM expr ')'
    {
      $span
    }
  | 'GET_FORMAT' '(' date_time_type  ',' expr ')'
    {
      $span
    }
  | now
    {
      $span
    }
  | 'POSITION' '(' bit_expr 'IN' expr ')'
    {
      $span
    }
  | 'SUBDATE' '(' expr ',' expr ')'
    {
      $span
    }
  | 'SUBDATE' '(' expr ',' 'INTERVAL' expr interval ')'
    {
      $span
    }
  | 'SUBSTRING' '(' expr ',' expr ',' expr ')'
    {
      $span
    }
  | 'SUBSTRING' '(' expr ',' expr ')'
    {
      $span
    }
  | 'SUBSTRING' '(' expr FROM expr ')'
    {
      $span
    }

  | 'SUBSTRING' '(' expr 'FROM' expr 'FOR' expr ')'
    {
      $span
    }
  | 'SYSDATE' func_datetime_precision
    {
      $span
    }
  | 'TIMESTAMP_ADD' '(' interval_time_stamp ',' expr ',' expr ')'
    {
      $span
    }
  | 'TIMESTAMP_DIFF' '(' interval_time_stamp ',' expr ',' expr ')'
    {
      $span
    }
  | 'UTC_DATE' optional_braces
    {
      $span
    }
  | 'UTC_TIME' func_datetime_precision
    {
      $span
    }
  | 'UTC_TIMESTAMP' func_datetime_precision
    {
      $span
    }
  ;

opt_returning_type -> Span:
    {
      $span
    }
  | 'RETURNING' cast_type
    {
      $span
    }
  ;

func_call_conflict -> Span:
    'ASCII' '(' expr ')'
    {
      $span
    }
  | 'CHARSET' '(' expr ')'
    {
      $span
    }
  | 'COALESCE' '(' expr_list ')'
    {
      $span
    }
  | 'COLLATION' '(' expr ')'
    {
      $span
    }
  | 'DATABASE' '(' ')'
    {
      $span
    }
  | 'IF' '(' expr ',' expr ',' expr ')'
    {
      $span
    }
  | 'FORMAT' '(' expr ',' expr ')'
    {
      $span
    }
  | 'FORMAT' '(' expr ',' expr ',' expr ')'
    {
      $span
    }
  | 'MICROSECOND' '(' expr ')'
    {
      $span
    }
  | 'MOD' '(' expr ',' expr ')'
    {
      $span
    }
  | 'QUARTER' '(' expr ')'
    {
      $span
    }
  | 'REPEAT' '(' expr ',' expr ')'
    {
      $span
    }
  | 'REPLACE' '(' expr ',' expr ',' expr ')'
    {
      $span
    }
  | 'REVERSE' '(' expr ')'
    {
      $span
    }
  | 'ROW_COUNT' '(' ')'
    {
      $span
    }
  | 'TRUNCATE' '(' expr ',' expr ')'
    {
      $span
    }
  | 'WEEK' '(' expr ')'
    {
      $span
    }
  | 'WEEK' '(' expr ',' expr ')'
    {
      $span
    }
  | 'WEIGHT_STRING' '(' expr ')'
    {
      $span
    }
  | 'WEIGHT_STRING' '(' expr 'AS' char ws_num_codepoints ')'
    {
      $span
    }
  | 'WEIGHT_STRING' '(' expr 'AS' 'BINARY' ws_num_codepoints ')'
    {
      $span
    }
  | 'WEIGHT_STRING' '(' expr ',' ulong_num ',' ulong_num ',' ulong_num ')'
    {
      $span
    }
  | geometry_func 
    {
      $span
    }
  ;

geometry_func -> Span:
    'GEOMETRYCOLLECTION' '(' opt_expr_list ')'
    {
      $span
    }
  | 'LINESTRING' '(' expr_list ')'
    {
      $span
    }
  | 'MULTILINESTRING' '(' expr_list ')'
    {
      $span
    }
  | 'MULTIPOINT' '(' expr_list ')'
    {
      $span
    }
  | 'MULTIPOLYGON' '(' expr_list ')'
    {
      $span
    }
  | 'POINT' '(' expr ',' expr ')'
    {
      $span
    }
  | 'POLYGON' '(' expr_list ')'
    {
      $span
    }
  ;

func_call_generic -> Span:
    IDENT_sys '(' opt_udf_expr_list ')'
    {
      $span
    }
  | ident '.' ident '(' opt_expr_list ')'
    {
      $span
    }
  ;

fulltext_options -> String:
    opt_natural_language_mode opt_query_expansion
    { 
      let mut s = String::new();
      if let Some(mode) = $1 {
        s.push_str(mode.as_str());
      }

      if let Some(exp) = $2 {
        s.push_str(exp.as_str());
      }

      s
    }
  | 'IN' 'BOOLEAN' 'MODE'
    {
      String::from("IN BOOLEAN MODE")
    }
  ;

opt_natural_language_mode -> Option<String>:
      /* nothing */                     { None }
    | 'IN' 'NATURAL' 'LANGUAGE' 'MODE'  { Some(String::from("IN NATURAL LANGUAGE MODE")) }
    ;

opt_query_expansion -> Option<String>:
      /* nothing */                      { None }
    | 'WITH' 'QUERY' 'EXPANSION'         { Some(String::from("WITH QUERY EXPANSION")) }
    ;

opt_udf_expr_list -> Expr:
    /* empty */     { Expr::None }
    | udf_expr_list { Expr::Ori(String::from($lexer.span_str($span))) }
    ;

udf_expr_list -> Vec<Expr>:
      udf_expr
      {
        vec![$1]
      }
    | udf_expr_list ',' udf_expr
      {
        $1.push($3);
        $1
      }
    ;

udf_expr -> Expr:
      expr select_alias
      {
        Expr::Ori(String::from($lexer.span_str($span)))
      }
    ;

set_func_specification -> Expr:
      sum_expr            
      { 
        $1
      }
    | grouping_operation  
      { 
        Expr::Ori(String::from($lexer.span_str($span)))
      }
    ;

sum_expr -> Expr:
    'AVG' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Avg,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'AVG' '(' 'DISTINCT' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Avg,
        distinct: true,
        exprs: vec![$4],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'BIT_AND'  '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::BitAnd,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'BIT_OR'  '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::BitOr,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'JSON_ARRAYAGG' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::JsonArrayAgg,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'JSON_OBJECTAGG' '(' in_sum_expr ',' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::JsonObjectAgg,
        distinct: false,
        exprs: vec![$3, $5],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'ST_COLLECT' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::StCollect,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'ST_COLLECT' '(' 'DISTINCT' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::StCollect,
        distinct: true,
        exprs: vec![$4],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'BIT_XOR' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::BitXor,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'COUNT' '(' opt_all '*' ')' opt_windowing_clause
    {
      let mut opt = String::with_capacity(4);
      if $3 {
        opt.push_str("ALL ");
      }

      opt.push('*');

      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Count,
        distinct: false,
        exprs: vec![Expr::Ori(opt)],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'COUNT' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Count,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'COUNT' '(' 'DISTINCT' expr_list ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Count,
        distinct: true,
        exprs: $4,
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'MIN' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Min,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'MIN' '(' 'DISTINCT' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Min,
        distinct: true,
        exprs: vec![$4],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'MAX' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Max,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'MAX' '(' 'DISTINCT' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Max,
        distinct: true,
        exprs: vec![$4],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'STD' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Std,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'VARIANCE' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Variance,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'STDDEV' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::StdDev,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'STDDEV_POP' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::StdDevPop,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'STDDEV_SAMP' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::StdDevSamp,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'VAR_POP' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::VarPop,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'VAR_SAMP' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::VarSamp,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'SUM' '(' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Sum,
        distinct: false,
        exprs: vec![$3],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'SUM' '(' 'DISTINCT' in_sum_expr ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::Sum,
        distinct: true,
        exprs: vec![$4],
        group_concat_distinct: None,
        gorder_clause: vec![],
        gconcat_separator: None,
      })
    }
  | 'GROUP_CONCAT' '(' opt_distinct expr_list opt_gorder_clause opt_gconcat_separator ')' opt_windowing_clause
    {
      Expr::AggExpr( AggExpr {
        span: $span,
        name: AggFuncName::GroupConcat,
        distinct: false,
        group_concat_distinct: $3,
        exprs: $4,
        gorder_clause: $5,
        gconcat_separator: $6,
      })
    }
  ;

window_func_call -> Span: 
    'ROW_NUMBER' '(' ')' windowing_clause
    {
      $span
    }
  | 'RANK' '(' ')' windowing_clause
    {
      $span
    }
  | 'DENSE_RANK' '(' ')' windowing_clause
    {
      $span
    }
  | 'CUME_DIST' '(' ')' windowing_clause
    {
      $span
    }
  | 'PERCENT_RANK' '(' ')' windowing_clause
    {
      $span
    }
  | 'NTILE' '(' stable_integer ')' windowing_clause
    {
      $span
    }
  | 'LEAD' '(' expr opt_lead_lag_info ')' opt_null_treatment windowing_clause
    {
      $span
    }
  | 'LAG' '(' expr opt_lead_lag_info ')' opt_null_treatment windowing_clause
    {
      $span
    }
  | 'FIRST_VALUE' '(' expr ')' opt_null_treatment windowing_clause
    {
      $span
    }
  | 'LAST_VALUE'  '(' expr ')' opt_null_treatment windowing_clause
    {
      $span
    }
  | 'NTH_VALUE' '(' expr ',' simple_expr ')' opt_from_first_last opt_null_treatment windowing_clause
    {
      $span
    }
  ;

opt_lead_lag_info -> Span:
    /* Nothing */
    {
      $span
    }
  | ',' stable_integer opt_ll_default
    {
      $span
    }
  ;

stable_integer -> Span:
    int64_literal  { $span }
  | param_or_var   { $span }
  ;

param_or_var -> Span:
    param_marker { $span }
  | ident        { $span }
  | '@' ident_or_text     { $span }
  ;

opt_ll_default -> Span:
    /* Nothing */
    {
      $span
    }
  | ',' expr
    {
      $span
    }
  ;

opt_null_treatment -> Span:
    /* Nothing */
    {
      $span
    }
  | 'RESPECT' 'NULLS'
    {
      $span
    }
  | 'IGNORE'  'NULLS'
    {
      $span
    }
  ;

opt_from_first_last -> Span:
    /* Nothing */
    {
      $span
    }
  | 'FROM' 'FIRST'
    {
      $span
    }
  | 'FROM' 'LAST'
    {
      $span
    }
  ;

opt_windowing_clause -> Span:
    /* Nothing */
    {
      $span
    }
  | windowing_clause
    {
      $span
    }
  ;

windowing_clause -> Span:
    'OVER' window_name_or_spec
    {
      $span
    }
  ;

window_name_or_spec -> Span:
    window_name
    {
      $span
    }
  | window_spec
    {
      $span
    }
  ;

window_name -> String:
    ident
    {
	    $1.0
    }
  ;

window_spec -> String:
    '(' window_spec_details ')'
    {
      String::from($lexer.span_str($span))
    }
  ;

window_spec_details -> Span:
    opt_existing_window_name opt_partition_clause opt_window_order_by_clause opt_window_frame_clause
    {
      $span
    }
  ;

opt_existing_window_name -> Span:
    /* Nothing */
    {
      $span
    }
  | window_name
    {
      $span
    }
  ;

opt_partition_clause -> Span:
    /* Nothing */
    {
      $span
    }
  | 'PARTITION' 'BY' group_list
    {
      $span
    }
  ;

opt_window_order_by_clause -> Span:
    /* Nothing */
    {
      $span
    }
  | 'ORDER' 'BY' order_list
    {
      $span
    }
  ;

opt_window_frame_clause -> Span:
    /* Nothing*/
    {
      $span
    }
  | window_frame_units window_frame_extent opt_window_frame_exclusion
    {
      $span
    }
  ;

window_frame_extent -> Span:
    window_frame_start
    {
      $span
    }
  | window_frame_between
    {
      $span
    }
  ;

window_frame_start -> Span:
    'UNBOUNDED' 'PRECEDING'
    {
      $span
    }
  | NUM_literal 'PRECEDING'
    {
      $span
    }
  | param_marker 'PRECEDING'
    {
      $span
    }
  | 'INTERVAL' expr interval 'PRECEDING'
    {
      $span
    }
  | 'CURRENT' 'ROW'
    {
      $span
    }
  ;

window_frame_between -> Span:
    'BETWEEN' window_frame_bound 'AND' window_frame_bound
    {
      $span
    }
  ;

window_frame_bound -> Span:
    window_frame_start
    {
      $span
    }
  | 'UNBOUNDED' 'FOLLOWING'
    {
      $span
    }
  | NUM_literal 'FOLLOWING'
    {
      $span
    }
  | param_marker 'FOLLOWING'
    {
      $span
    }
  | 'INTERVAL' expr interval 'FOLLOWING'
    {
      $span
    }
  ;

opt_window_frame_exclusion -> Span:
    /* Nothing */
    {
      $span
    }
  | 'EXCLUDE' 'CURRENT' 'ROW'
    {
      $span
    }
  | 'EXCLUDE' 'GROUP'
    {
      $span
    }
  | 'EXCLUDE' 'TIES'
    {
      $span
    }
  | 'EXCLUDE' 'NO' 'OTHERS'
    {
      $span
    }
  ;

window_frame_units -> Span:
    'ROWS'    { $span }
  | 'RANGE'   { $span }
  | 'GROUPS'  { $span }
  ;

grouping_operation -> Vec<Expr>:
    'GROUPING' '(' expr_list ')'
    {
      $3
    }
  ;

variable -> String:
    '@' variable_aux 
    {  
      let mut s = String::with_capacity(1+$2.len());
      s.push('@');
      s.push_str(&$2);
      s
    }
  ;

variable_aux -> String:
    ident_or_text 'SET_VAR' expr
    {
      $1
    }
  | ident_or_text
    {
      $1
    }
  | '@' opt_var_ident_type ident_or_text opt_component
    {
      let mut s = String::new();
      s.push('@');
      if let Some(var) = $2 {
        s.push_str(&var);
      }
      s.push_str(&$3);
      if let Some(com) = $4 {
        s.push_str(&com);
      }

      s
    }
  ;

opt_distinct -> Option<String>:
    /* empty */ { None }
  | 'DISTINCT'    { Some(String::from("DISTINCT")) }
  | 'DISTINCTROW' { Some(String::from("DISTINCTROW")) }
  ;

opt_gconcat_separator -> Option<String>:
    /* empty */
    {
      None
    }
  | 'SEPARATOR' text_binary_string
    { 
      Some($2)
    }
  ;

opt_gorder_clause -> Vec<OrderExpr>:
      /* empty */               { vec![] }
    | 'ORDER' 'BY' gorder_list  { $3   }
    ;

gorder_list -> Vec<OrderExpr>:
    gorder_list ',' order_expr
    {
      $1.push($3);
      $1
    }
  | order_expr
    {
      vec![$1]
    }
  ;

in_sum_expr -> Expr:
    opt_all expr
    {
      Expr::InSumExpr {
        span: $span,
        opt: $1,
        expr: Box::new($2)
      }
    }
  ;

cast_type -> FieldType:
    'BINARY' opt_field_length
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | char opt_field_length opt_charset_with_opt_binary
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | nchar opt_field_length
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'SIGNED'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'SIGNED' 'INT'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'UNSIGNED'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'UNSIGNED' 'INT'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'DATE'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'YEAR'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'TIME' type_datetime_precision
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'DATETIME' type_datetime_precision
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'DECIMAL' float_options
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'JSON'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | real_type
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'FLOAT' standard_float_options
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'POINT'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'LINESTRING'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'POLYGON'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'MULTIPOINT'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'MULTILINESTRING'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'MULTIPOLYGON'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  | 'GEOMETRYCOLLECTION'
    {
      FieldType::FieldCastType(String::from($lexer.span_str($span)))
    }
  ;

opt_expr_list -> Vec<Expr>:
    /* empty */ { vec![] }
  | expr_list   { $1     }
  ;

expr_list -> Vec<Expr>:
    expr %prec LOWER_THEN_COMMA
    {
      vec![$1]
    }
  | expr_list ',' expr
    {
      $1.push($3);
      $1
    }
  ;


ident_list_arg -> Vec<Value>:
    ident_list          { $1 }
  | '(' ident_list ')'  { $2 }
  ;

ident_list -> Vec<Value>:
    simple_ident
    {
      vec![$1]
    }
  | ident_list ',' simple_ident
    {
      $1.push($3);
      $1
    }
  ;

opt_expr -> Option<Expr>:
    /* empty */    { None   }
  | expr           { Some($1) }
  ;

opt_else -> Option<Expr>:
    /* empty */  { None  }
  | 'ELSE' expr  { Some($2) }
  ;

when_list -> Vec<WhenExpr>:
    'WHEN' expr 'THEN' expr
    {
      vec![
        WhenExpr{
          span: $span,
          when: Box::new($2),
          then: Box::new($4)
      }] 
    }
  | when_list 'WHEN' expr 'THEN' expr
    {
      $1.push(
        WhenExpr {
          span: $span,
          when: Box::new($3),
          then: Box::new($5)
        }
      );

      $1
    }
  ;

table_reference -> TableRef:
    table_factor 
    { 
      TableRef::TableFactor(Box::new($1))
    }
  | joined_table
    {
      TableRef::JoinedTable(Box::new($1))
    }
  | '{' 'OJ' esc_table_reference '}'
    {
      $3
    }
  ;

esc_table_reference -> TableRef:
    table_factor 
    {
      TableRef::TableFactor(Box::new($1))
    }
  | joined_table 
    {
      TableRef::JoinedTable(Box::new($1))
    }
  ;

joined_table -> JoinedTable:
    table_reference inner_join_type table_reference 'ON' expr
    {
      JoinedTable {
        span: $span,
        left: $1,
        join_type: String::from($2),
        right: $3,
        on_cond: Some(Box::new($5)),
	      using_list: vec![]
      }
    }
  | table_reference inner_join_type table_reference 'USING' '(' using_list ')'
    {
      JoinedTable {
        span: $span,
        left: $1,
        join_type: $2.to_string(),
        right: $3,
        using_list: $6,
	      on_cond: None
      }
    }
  | table_reference outer_join_type table_reference 'ON' expr
    {
      JoinedTable {
        span: $span,
        left: $1,
        join_type: $2,
        right: $3,
        on_cond: Some(Box::new($5)),
        using_list: vec![]
      }
    }
  | table_reference outer_join_type table_reference 'USING' '(' using_list ')'
    {
      JoinedTable {
        span: $span,
        left: $1,
        join_type: $2,
        right: $3,
        using_list: $6,
	      on_cond: None
      }
    }
  | table_reference inner_join_type table_reference %prec CONDITIONLESS_JOIN
    {
      JoinedTable {
        span: $span,
        left: $1,
        join_type: String::from($2),
        right: $3,
	      on_cond: None,
	      using_list: vec![]
      }
    }
  | table_reference natural_join_type table_factor
    {
      JoinedTable {
        span: $span,
        left: $1,
        join_type: $2,
        right: TableRef::TableFactor(Box::new($3)),
	      on_cond: None,
	      using_list: vec![]
      }
    }
  ;

natural_join_type -> String:
    'NATURAL' opt_inner 'JOIN'      { 
      let mut join_type = String::with_capacity(20);
      join_type.push_str("NATURAL ");
      if let Some(inner) = $2 {
	      join_type.push_str(inner);
      }
      join_type.push_str(" JOIN");
      join_type
    }
  | 'NATURAL' 'RIGHT' opt_outer 'JOIN' 
    { 
      let mut join_type = String::with_capacity(30);
      join_type.push_str("NATURAL RIGHT ");
      if let Some(outer) = $3 {
	      join_type.push_str(outer);
      }
      join_type.push_str(" JOIN");
      join_type
    }
  | 'NATURAL' 'LEFT' opt_outer 'JOIN'
    { 
      let mut join_type = String::with_capacity(30);
      join_type.push_str("NATURAL LEFT ");
      if let Some(outer) = $3 {
	      join_type.push_str(outer);
      }
      join_type.push_str(" JOIN");
      join_type
    }
  ;

inner_join_type -> &'input str:
    'JOIN'                      { "JOIN" }
  | 'INNER' 'JOIN'              { "INNER JOIN" }
  | 'CROSS' 'JOIN'              { "CROSS JOIN" }
  | 'STRAIGHT_JOIN'             { "STRAIGHT_JOIN" }
  ;

outer_join_type -> String:
    'LEFT' opt_outer 'JOIN'         
    {
      let mut s = String::with_capacity(15);
      s.push_str("LEFT ");
      if let Some(outer) = $2 {
        s.push_str(outer);
      }
      s.push_str(" JOIN");
      s
    }
  | 'RIGHT' opt_outer 'JOIN'         
    { 
      let mut s = String::with_capacity(16);
      s.push_str("RIGHT ");
      if let Some(outer) = $2 {
        s.push_str(outer);
      }
      s.push_str(" JOIN");
      s
    }
  ;

opt_inner -> Option<&'input str>:
    /* empty */ { None }
  | 'INNER' { Some("INNER") }
  ;

opt_outer -> Option<&'input str>:
    /* empty */ { None }
  | 'OUTER' { Some("OUTER") }
  ;

opt_use_partition -> Vec<String>:
    /* empty */ { vec![] }
  | use_partition { $1 }
  ;

use_partition -> Vec<String>:
    'PARTITION' '(' using_list ')'
    {
      $3
    }
  ;

table_factor -> TableFactor:
    single_table  	
    { 
      TableFactor::SingleTable($1)
    }
  | single_table_parens
    { 
      $1 
    }
  | derived_table 
    { 
      $1.is_parens = false; 
      TableFactor::DerivedTable($1)
    }
  | '(' derived_table ')'
    { 
      $2.is_parens = true;
      TableFactor::DerivedTable($2)
    }
  | joined_table_parens
    { 
      $1 
    }
  | table_reference_list_parens
    { 
      $1 
    }
  | table_func 
    { 
      TableFactor::TableFunc($1) 
    }
  ;

table_reference_list_parens -> TableFactor:
    '(' table_reference_list_parens ')' 
    { 
	    $2 
    }
  | '(' table_reference_list ',' table_reference ')'
    {
      $2.push($4);
      TableFactor::TableRefsParens($2)	
    }
  ;

single_table_parens -> TableFactor:
    '(' single_table_parens ')' { $2 }
  | '(' single_table ')' { TableFactor::SingleTableParens($2) }
  ;

single_table -> SingleTable:
    table_ident opt_use_partition opt_table_alias opt_key_definition
    {
	    SingleTable {
        span: $span,
	      table_name: $1,
	      partition_names: $2,
	      alias_name: $3,
	      index_hints: $4,
	      is_parens: false
	    }
    }
  ;

joined_table_parens -> TableFactor:
    '(' joined_table_parens ')' { $2 }
  | '(' joined_table ')' { TableFactor::JoinedTableParens(Box::new($2)) }
  ;

derived_table -> DerivedTable:
    table_subquery opt_table_alias opt_derived_column_list
    {
      DerivedTable {
        span: $span,
    	  subquery: $1,
        alias_name: $2,
    	  columns: $3,
      	lateral: false,
	      is_parens: false
      }
    }
  | 'LATERAL' table_subquery opt_table_alias opt_derived_column_list
    {
      DerivedTable {
        span: $span,
    	  subquery: $2,
    	  alias_name: $3,
    	  columns: $4,
    	  lateral: true,
	      is_parens: false
      }
    }
  ;

table_func -> TableFunc:
    'JSON_TABLE' '(' expr ',' text_literal columns_clause ')' opt_table_alias
    {
      TableFunc {
        span: $span,
        expr: $3,
        text: $5,
        columns_clause: $6,
        alias_name: $8
      }
    }
  ;

columns_clause -> Vec<String>:
    'COLUMNS' '(' columns_list ')'
    {
      $3
    }
  ;

columns_list -> Vec<String>:
    jt_column
    {
      vec![$1]
    }
  | columns_list ',' jt_column
    {
      $1.push($3);
      $1
    }
  ;

jt_column -> String:
    ident 'FOR' 'ORDINALITY'
    {
      String::from($lexer.span_str($span))
    }
  | ident type opt_collate jt_column_type 'PATH' text_literal opt_on_empty_or_error_json_table
    {
      String::from($lexer.span_str($span))
    }
  | 'NESTED' 'PATH' text_literal columns_clause
    {
      String::from($lexer.span_str($span))
    }
  ;

jt_column_type -> Option<String>:
    {
      None
    }
  | 'EXISTS'
    {
      Some(String::from("EXISTS"))
    }
  ;

opt_on_empty_or_error -> Option<String>:
    /* empty */
    {
      None
    }
  | on_empty
    {
      Some($1)
    }
  | on_error
    {
      Some($1)
    }
  | on_empty on_error
    {
      let mut s = String::with_capacity($1.len()+1+$2.len());
      s.push_str($1.as_str());
      s.push(' ');
      s.push_str($2.as_str());
      Some(s)
    }
  ;

// JSON_TABLE extends the syntax by allowing ON ERROR to come before ON EMPTY.
opt_on_empty_or_error_json_table -> String:
    opt_on_empty_or_error { $1.unwrap_or_else(|| "".to_string()) }
  | on_error on_empty
    {
      let mut s = String::with_capacity($1.len()+1+$2.len());
      s.push_str($1.as_str());
      s.push(' ');
      s.push_str($2.as_str());
      s
    }
  ;

on_empty -> String:
    json_on_response 'ON' 'EMPTY' 
    { 
      //let mut s = String::with_capacity($1.len()+2+7);
      //s.push_str($1.as_str());
      //s.push_str(" ON EMPTY");
      //s
      String::from(" ON ERROR")
    }
  ;

on_error -> String:
    json_on_response 'ON' 'ERROR' 
    { 
      //let mut s = String::with_capacity($1.len()+2+7);
      //s.push_str($1.as_str());
      //s.push_str(" ON ERROR");
      //s
      String::from(" ON ERROR")
    }
  ;

json_on_response -> Value:
    'ERROR'
    {
      Value::Error
    }
  | 'NULL'
    {
      Value::Null
    }
  | 'DEFAULT' signed_literal
    {
      Value::Default {
        span: $span,
        value: Box::new($2)
      }
    }
  ;

index_hint_clause -> Option<String>:
    /* empty */
    {
      None
    }
  | 'FOR' 'JOIN'        { Some(String::from("FOR JOIN")) }
  | 'FOR' 'ORDER' 'BY'  { Some(String::from("FOR ORDER BY")) }
  | 'FOR' 'GROUP' 'BY'  { Some(String::from("FOR GROUP BY")) }
  ;

index_hint_type -> String:
    'FORCE'  { String::from("FORCE") }
  | 'IGNORE' { String::from("IGNORE") }
  ;

index_hint_definition -> Span:
     index_hint_type key_or_index index_hint_clause '(' key_usage_list ')'
     {
       $span
     }
   | 'USE' key_or_index index_hint_clause '(' opt_key_usage_list ')'
     {
       $span
     }
  ;

index_hints_list -> Span:
    index_hint_definition
    {
      $span
    }
  | index_hints_list index_hint_definition
    {
      $span
    }
  ;

opt_index_hints_list -> Option<String>:
    /* empty */       { None }
  | index_hints_list  { Some(String::from($lexer.span_str($1))) }
  ;

opt_key_definition -> Option<String>:
  opt_index_hints_list { $1 }
  ;

opt_key_usage_list -> Span:
    /* empty */
    {
      $span
    }
  | key_usage_list { $span }
  ;

key_usage_element -> Span:
  ident
  {
    $span
  }
| 'PRIMARY'
  {
    $span
  }
;

key_usage_list -> Span:
  key_usage_element
  {
    $span
  }
| key_usage_list ',' key_usage_element
  {
    $span
  }
;

using_list -> Vec<String>:
  ident_string_list { $1 }
;

ident_string_list -> Vec<String>:
  ident
  {
    vec![$1.0]
  }
  | ident_string_list ',' ident
  {
    $1.push($3.0);
    $1
  }
;

interval -> Expr:
    interval_time_stamp  { $1 }
  | 'DAY_HOUR'           { Expr::TimeIntervalUnit(String::from("DAY_HOUR")) }
  | 'DAY_MICROSECOND'    { Expr::TimeIntervalUnit(String::from("DAY_MICROSECOND")) }
  | 'DAY_MINUTE'         { Expr::TimeIntervalUnit(String::from("DAY_MINUTE")) }
  | 'DAY_SECOND'         { Expr::TimeIntervalUnit(String::from("DAY_SECOND")) }
  | 'HOUR_MICROSECOND'   { Expr::TimeIntervalUnit(String::from("HOUR_MICROSECOND")) }
  | 'HOUR_MINUTE'        { Expr::TimeIntervalUnit(String::from("HOUR_MINUTE")) }
  | 'HOUR_SECOND'        { Expr::TimeIntervalUnit(String::from("HOUR_SECOND")) }
  | 'MINUTE_MICROSECOND' { Expr::TimeIntervalUnit(String::from("MINUTE_MICROSECOND")) }
  | 'MINUTE_SECOND'      { Expr::TimeIntervalUnit(String::from("MINUTE_SECOND")) }
  | 'SECOND_MICROSECOND' { Expr::TimeIntervalUnit(String::from("SECOND_MICROSECOND")) }
  | 'YEAR_MONTH'         { Expr::TimeIntervalUnit(String::from("YEAR_MONTH")) }
  ;

interval_time_stamp -> Expr:
    'DAY'         { Expr::TimeIntervalUnit(String::from("DAY")) }
  | 'WEEK'        { Expr::TimeIntervalUnit(String::from("WEEK")) }
  | 'HOUR'        { Expr::TimeIntervalUnit(String::from("HOUR")) }
  | 'MINUTE'      { Expr::TimeIntervalUnit(String::from("MINUTE")) }
  | 'MONTH'       { Expr::TimeIntervalUnit(String::from("MONTH")) }
  | 'QUARTER'     { Expr::TimeIntervalUnit(String::from("QUARTER")) }
  | 'SECOND'      { Expr::TimeIntervalUnit(String::from("SECOND")) }
  | 'MICROSECOND' { Expr::TimeIntervalUnit(String::from("MICROSECOND")) }
  | 'YEAR'        { Expr::TimeIntervalUnit(String::from("YEAR")) }
  ;

date_time_type -> String:
    'DATE'      { String::from("DATE") }
  | 'TIME'      { String::from("TIME") }
  | 'TIMESTAMP' { String::from("TIMESTAMP") }
  | 'DATETIME'  { String::from("DATETIME") }
  ;

opt_as -> Option<&'input str>:
    /* empty */ { None }
  | 'AS'        { Some("AS") }
  ;

opt_table_alias -> Option<String>:
    /* empty */  { None }
  | opt_as ident { Some($2.0) }
  ;

opt_all -> bool :
  /* empty */ { false }
| 'ALL' { true }
;

opt_where_clause -> Option<WhereClause>:
    /* empty */   { None }
  | where_clause  { Some($1)   }
  ;

where_clause -> WhereClause:
    'WHERE' expr  
    {  
      WhereClause {
        span: $span,
        expr: Box::new($2)
      }
    }
  ;

opt_having_clause -> Option<HavingClause>:
    /* empty */ { None }
  | 'HAVING' expr
    {
      Some(HavingClause {
        span: $span,
        expr: Box::new($2)
      })
    }
  ;

with_clause -> WithClause:
    'WITH' with_list
    {
      WithClause{
        span: $span,
        with_exprs: $2,
        recursive: false,
      }
    }
  | 'WITH' 'RECURSIVE' with_list
    {
      WithClause{
        span: $span,
        with_exprs: $3,
        recursive: true,
      }
    }
  ;

with_list -> Vec<CommonTableExpr>:
    with_list ',' common_table_expr
    {
      $1.push($3);
      $1
    }
  | common_table_expr
    {
      vec![$1]
    }
  ;


common_table_expr -> CommonTableExpr:
  ident opt_derived_column_list 'AS' table_subquery
  {
    CommonTableExpr {
      span: $span,
      ident: $1.0,
      columns: $2,
      subquery: $4
    }
  }
;

opt_derived_column_list -> Vec<Value>:
    /* empty */
    {
      vec![]
    }
  | '(' simple_ident_list ')'
    {
      $2
    }
  ;

simple_ident_list -> Vec<Value>:
    ident
    {
      vec![Value::Ident{
        span: $span,
        value: $1.0,
        quoted: $1.1,
      }]
    }
  | simple_ident_list ',' ident
    {
      $1.push(Value::Ident{
        span: $span,
        value: $3.0,
        quoted: $3.1, 
      });
      $1
    }
  ;

opt_window_clause -> Option<WindowClause>:
    /* Nothing */
    {
      None
    }
  | 'WINDOW' window_definition_list
    {
      Some(WindowClause{
        span: $span,
        defs: $2
      })
    }
  ;

window_definition_list -> Vec<WindowDef>:
    window_definition
    {
      vec![$1]
    }
  | window_definition_list ',' window_definition
    {
      $1.push($3);
      $1
    }
  ;

window_definition -> WindowDef:
  window_name 'AS' window_spec
  {
    WindowDef {
      span: $span,
      name: $1,
      spec: $3,
    }
  }
;

opt_group_clause -> Option<GroupClause>:
    /* empty */   { None }
  | 'GROUP' 'BY' group_list olap_opt
    {
      Some(GroupClause{
        span: $span,
        exprs: $3,
        olap: $4
      })
    }
  ;

group_list -> Vec<Expr>:
    group_list ',' grouping_expr
    {
      $1.push($3);
      $1
    }
  | grouping_expr
    {
      vec![$1]
    }
  ;


olap_opt -> bool:
   /* empty */      { false }
 | 'WITH' 'ROLLUP'  { true  }
 ;

opt_order_clause -> Option<OrderClause>:
    /* empty */   { None }
  | order_clause  { Some($1) }
  ;

order_clause -> OrderClause:
    'ORDER' 'BY' order_list
    {
      OrderClause {
        span: $span,
        orders: $3
      }
    }
  ;

order_list -> Vec<OrderExpr>:
    order_list ',' order_expr
    {
      $1.push($3);
      $1
    }
  | order_expr
    {
      vec![$1]
    }
  ;

opt_ordering_direction -> Option<String>:
    /* empty */         { None }
  | ordering_direction  { Some(String::from($1)) }
  ;

ordering_direction -> &'input str:
    'ASC'         { "ASC" }
  | 'DESC'        { "DESC" }
  ;

opt_limit_clause -> Option<LimitClause>:
    /* empty */ { None }
  | limit_clause { Some($1) }
  ;

limit_clause -> LimitClause:
    'LIMIT' limit_options
    {
      $2
    }
  ;

limit_options -> LimitClause:
    limit_option
    {
      
      LimitClause {
        span: $span,
        opts: vec![$1],
	      offset: false
      }
    }
  | limit_option ',' limit_option
    {
      LimitClause {
        span: $span,
        opts: vec![$1, $3],
	      offset: false
      }
    }
  | limit_option 'OFFSET' limit_option
    {
      LimitClause {
        span: $span,
        opts: vec![$1, $3],
        offset: true
      }
    }
  ;

limit_option -> LimitOption:
    ident
    {
      LimitOption {
        span: $span,
        opt: $1.0,
      }
    }
  | param_marker
    {
      LimitOption {
        span: $span,
        opt: $1,
      }
    }
  | 'ULONGLONG_NUM'
    {
      LimitOption {
        span: $span,
        opt: String::from($lexer.span_str($1.as_ref().unwrap().span())),
      }
      
    }
  | 'LONG_NUM'
    {
      LimitOption {
        span: $span,
        opt: String::from($lexer.span_str($1.as_ref().unwrap().span())),
      }
    }
  | 'NUM'
    {
      LimitOption {
        span: $span,
        opt: String::from($lexer.span_str($1.as_ref().unwrap().span())),
      }
    }
  ;

opt_simple_limit -> Option<LimitOption>:
    /* empty */        { None }
  | 'LIMIT' limit_option { Some($2)  }
  ;

ulong_num -> String:
    'NUM'           { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'HEX_NUM'       { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'LONG_NUM'      { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'ULONGLONG_NUM' { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'DECIMAL_NUM'   { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'FLOAT_NUM'     { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  ;

real_ulong_num -> String:
    'NUM'           { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'HEX_NUM'       { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'LONG_NUM'      { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'ULONGLONG_NUM' { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | dec_num_error   { $1 }
  ;

ulonglong_num -> String:
    'NUM'           { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'ULONGLONG_NUM' { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'LONG_NUM'      { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'DECIMAL_NUM'   { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  | 'FLOAT_NUM'     { String::from($lexer.span_str($1.as_ref().unwrap().span()))  }
  ;

real_ulonglong_num -> String:
    'NUM'           { String::from($lexer.span_str($1.as_ref().unwrap().span())) }
  | 'HEX_NUM'       { String::from($lexer.span_str($1.as_ref().unwrap().span())) } 
  | 'ULONGLONG_NUM' { String::from($lexer.span_str($1.as_ref().unwrap().span())) }
  | 'LONG_NUM'      { String::from($lexer.span_str($1.as_ref().unwrap().span())) }
  | dec_num_error { $1 }
  ;

dec_num_error -> String:
    dec_num
    { $1 }
  ;

dec_num -> String:
    'DECIMAL_NUM'
    {
	    String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  | 'FLOAT_NUM'
    { 
	    String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  ;

select_var_list -> Vec<Value>:
    select_var_list ',' select_var_ident
    {
	    $1.push($3);
	    $1
    }
  | select_var_ident
    {
	    vec![$1]
    }
  ;

select_var_ident -> Value:
    '@' ident_or_text
    {
        Value::SelectVarIdent {
          span: $span,
          value: $2,
          is_at: true,
        }
    }
  | ident_or_text
    {
        Value::SelectVarIdent {
          span: $span,
          value: $1,
          is_at: false
        }
    }
  ;

into_clause -> IntoClause:
    'INTO' into_destination
    {
	    $2
    }
  ; 

into_destination -> IntoClause:
    'OUTFILE' 'TEXT_STRING' opt_load_data_charset opt_field_term opt_line_term
    {
      IntoClause::OutFile(OutFile { 
        span: $span,
        name:  String::from($lexer.span_str($2.as_ref().unwrap().span())),
        character_set: $3,
        field_term: $4,
        line_term: $5
      })
    }
  | 'DUMPFILE' 'TEXT_STRING'
    {
      IntoClause::DumpFile(DumpFile {
       span: $span,
       name: String::from($lexer.span_str($2.as_ref().unwrap().span())),
      })
    }
  | select_var_list 
    { 
	    IntoClause::Vars($1)
    }
  ;

union_option -> Option<UnionOpt>:
    /* empty */ { None }
  | 'DISTINCT'  { Some(UnionOpt::Distinct) }
  | 'ALL'       { Some(UnionOpt::All) }
  ;

row_subquery -> SelectStmt:
    subquery { $1 } 
  ;

table_subquery -> SelectStmt:
    subquery 
    { 
      $1
    }
  ;

subquery -> SelectStmt:
    query_expression_parens  %prec SUBQUERY_AS_EXPR
    {
      $1
    }
  ;

query_spec_option -> QuerySpecOpt:
    'STRAIGHT_JOIN'       { QuerySpecOpt::StraightJoin }
  | 'HIGH_PRIORITY'       { QuerySpecOpt::HighPriority }
  | 'LOW_PRIORITY'        { QuerySpecOpt::LowPriority  }
  | 'DELAYED'             { QuerySpecOpt::Delayed }
  | 'DISTINCT'            { QuerySpecOpt::Distinct }
  | 'DISTINCTROW'         { QuerySpecOpt::Distinctrow }
  | 'SQL_SMALL_RESULT'    { QuerySpecOpt::SqlSmallResult }
  | 'SQL_BIG_RESULT'      { QuerySpecOpt::SqlBigResult }
  | 'SQL_BUFFER_RESULT'   { QuerySpecOpt::SqlBufferResult }
  | 'SQL_CACHE'           { QuerySpecOpt::SqlCache }
  | 'SQL_NO_CACHE'        { QuerySpecOpt::SqlNoCache }
  | 'SQL_CALC_FOUND_ROWS' { QuerySpecOpt::SqlCalcFoundRows }
  | 'ALL'                 { QuerySpecOpt::All }
  ;

values_row_list -> Vec<Vec<Expr>>:
    values_row_list ',' row_value_explicit
    {
      $1.push($3);
      $1
    }
  | row_value_explicit
    {
      vec![$1]
    }
  ;

equal -> &'input str:
    'EQ'      { "=" }
  | 'SET_VAR' { "SET_VAR" }
  ;

opt_equal -> Option<&'input str>:
    /* empty */ { None }
  | equal       { Some($1) }
  ;

row_value -> RowValue:
  '(' opt_values ')' 
  { 
    RowValue {
      span: $span,
      values: $2,
    }
  }
  ;

opt_values -> Vec<Expr>:
    /* empty */
    {
      vec![]
    }
  | values 
    { 
      $1
    }
  ;

values -> Vec<Expr>:
    values ','  expr_or_default
    {
      $1.push($3);
      $1
    }
  | expr_or_default
    {
      vec![$1]
    }
  ;

expr_or_default -> Expr:
    expr 
    { 
      $1
    }
  | 'DEFAULT'
    {
      Expr::DefaultExpr
    }
  ;

row_value_explicit -> Vec<Expr>:
    'ROW' '(' opt_values ')' 
    { 
      $3
    }
  ;

table_ident -> TableIdent:
    ident
    {
    	TableIdent {
	  span: $span,
	  schema: None,
	  name: $1.0,
	}
    }
  | ident '.' ident
    {
    	TableIdent {
          span: $span,
          schema: Some($1.0),
          name: $3.0,
        }
    }
  ;

table_alias_ref_list -> Vec<String>:
    table_ident_opt_wild
    {
      vec![$1]
    }
  | table_alias_ref_list ',' table_ident_opt_wild
    {
      $1.push($3);
      $1
    }
  ;

table_ident_opt_wild -> String:
    ident opt_wild
    {
      let mut s = String::with_capacity($1.0.len()+2);
      s.push_str(&$1.0);
      if let Some(wild) = $2 {
        s.push_str(wild);
      }
      s
    }
  | ident '.' ident opt_wild
    {
      let mut s = String::with_capacity($1.0.len()+$3.0.len()+3);
      s.push_str(&$1.0);
      s.push('.');
      s.push_str(&$3.0);
      if let Some(wild) = $4 {
        s.push_str(wild);
      }
      s
    }
  ;

opt_wild -> Option<&'input str>:
    /* empty */ { None } 
  | '.' '*'     { Some(".*") }
  ;


IDENT_sys -> (String, bool):
  'IDENT' 
  {
    (String::from($lexer.span_str($1.as_ref().unwrap().span())), false)
  }
| 'IDENT_QUOTED'
  {
    (String::from($lexer.span_str($1.as_ref().unwrap().span())), true)
  }
;

table_wild -> TableWild:
    ident '.' '*'
    {
      TableWild {
        span: $span,
        table: $1.0,
        schema: None
      }
    }
  | ident '.' ident '.' '*'
    {
      TableWild {
        span: $span,
        schema: Some($1.0),
        table: $3.0
      }
    }
  ;

ident -> (String, bool):
    IDENT_sys    { $1 }
  | non_reserved_keyword { (String::from($1), false) }
  ;

/* TODO */
role_ident -> (String, bool):
    IDENT_sys      { $1 }
  | non_reserved_keyword { (String::from($1), false) }
;

/*
  non-reserved keywords
*/
non_reserved_keyword -> &'input str:
    'ACCOUNT'                                          { "ACCOUNT" }
  | 'ACTION'                                           { "ACTION" }
  | 'ACTIVE'                                           { "ACTIVE" }
  | 'ADDDATE'                                          { "ADDDATE" }
  | 'ADMIN'                                            { "ADMIN" }
  | 'AFTER'                                            { "AFTER" }
  | 'AGAINST'                                          { "AGAINST" }
  | 'AGGREGATE'                                        { "AGGREGATE" }
  | 'ALGORITHM'                                        { "ALGORITHM" }
  | 'ALWAYS'                                           { "ALWAYS" }
  | 'ANY'                                              { "ANY" }
  | 'ARRAY'                                            { "ARRAY" }
  | 'ASCII'                                            { "ASCII" }
  | 'ASSIGN_GTIDS_TO_ANONYMOUS_TRANSACTIONS'           { "ASSIGN_GTIDS_TO_ANONYMOUS_TRANSACTIONS" }
  | 'AT'                                               { "AT" }
  | 'ATTRIBUTE'                                        { "ATTRIBUTE" }
  | 'AUTHENTICATION'                                   { "AUTHENTICATION" }
  | 'AUTOEXTEND_SIZE'                                  { "AUTOEXTEND_SIZE" }
  | 'AUTO_INC'                                         { "AUTO_INC" }
  | 'AVG'                                              { "AVG" }
  | 'AVG_ROW_LENGTH'                                   { "AVG_ROW_LENGTH" }
  | 'BACKUP'                                           { "BACKUP" }
  | 'BEGIN'                                            { "BEGIN" }
  | 'BINLOG'                                           { "BINLOG" }
  | 'BIT'                                              { "BIT" }
  | 'BLOCK'                                            { "BLOCK" }
  | 'BOOL'                                             { "BOOL" }
  | 'BOOLEAN'                                          { "BOOLEAN" }
  | 'BTREE'                                            { "BTREE" }
  | 'BUCKETS'                                          { "BUCKETS" }
  | 'BYTE'                                             { "BYTE" }
  | 'CACHE'                                            { "CACHE" }
  | 'CASCADED'                                         { "CASCADED" }
  | 'CATALOG_NAME'                                     { "CATALOG_NAME" }
  | 'CHAIN'                                            { "CHAIN" }
  | 'CHALLENGE_RESPONSE'                               { "CHALLENGE_RESPONSE" }
  | 'CHANGED'                                          { "CHANGED" }
  | 'CHANNEL'                                          { "CHANNEL" }
  | 'CHARSET'                                          { "CHARSET" }
  | 'CHECKSUM'                                         { "CHECKSUM" }
  | 'CIPHER'                                           { "CIPHER" }
  | 'CLASS_ORIGIN'                                     { "CLASS_ORIGIN" }
  | 'CLIENT'                                           { "CLIENT" }
  | 'CLONE'                                            { "CLONE" }
  | 'CLOSE'                                            { "CLOSE" }
  | 'COALESCE'                                         { "COALESCE" }
  | 'CODE'                                             { "CODE" }
  | 'COLLATION'                                        { "COLLATION" }
  | 'COLUMNS'                                          { "COLUMNS" }
  | 'COLUMN_FORMAT'                                    { "COLUMN_FORMAT" }
  | 'COLUMN_NAME'                                      { "COLUMN_NAME" }
  | 'COMMENT'                                          { "COMMENT" }
  | 'COMMIT'                                           { "COMMIT" }
  | 'COMMITTED'                                        { "COMMITTED" }
  | 'COMPACT'                                          { "COMPACT" }
  | 'COMPLETION'                                       { "COMPLETION" }
  | 'COMPONENT'                                        { "COMPONENT" }
  | 'COMPRESSED'                                       { "COMPRESSED" }
  | 'COMPRESSION'                                      { "COMPRESSION" }
  | 'CONCURRENT'                                       { "CONCURRENT" }
  | 'CONNECTION'                                       { "CONNECTION" }
  | 'CONSISTENT'                                       { "CONSISTENT" }
  | 'CONSTRAINT_CATALOG'                               { "CONSTRAINT_CATALOG" }
  | 'CONSTRAINT_NAME'                                  { "CONSTRAINT_NAME" }
  | 'CONSTRAINT_SCHEMA'                                { "CONSTRAINT_SCHEMA" }
  | 'CONTAINS'                                         { "CONTAINS" }
  | 'CONTEXT'                                          { "CONTEXT" }
  | 'COUNT'                                            { "COUNT" }
  | 'CPU'                                              { "CPU" }
  | 'CURRENT'                                          { "CURRENT" }
  | 'CURSOR_NAME'                                      { "CURSOR_NAME" }
  | 'DATA'                                             { "DATA" }
  | 'DATAFILE'                                         { "DATAFILE" }
  | 'DATE'         %prec LOWER_THEN_TEXT_STRING        { "DATE" }
  | 'DATETIME'                                         { "DATETIME" }
  | 'DAY'                                              { "DAY" }
  | 'DEALLOCATE'                                       { "DEALLOCATE" }
  | 'DEFAULT_AUTH'                                     { "DEFAULT_AUTH" }
  | 'DEFINER'                                          { "DEFINER" }
  | 'DEFINITION'                                       { "DEFINITION" }
  | 'DELAY_KEY_WRITE'                                  { "DELAY_KEY_WRITE" }
  | 'DESC'                                             { "DESC" }
  | 'DESCRIPTION'                                      { "DESCRIPTION" }
  | 'DIAGNOSTICS'                                      { "DIAGNOSTICS" }
  | 'DIRECTORY'                                        { "DIRECTORY" }
  | 'DISABLE'                                          { "DISABLE" }
  | 'DISCARD'                                          { "DISCARD" }
  | 'DISK'                                             { "DISK" }
  | 'DO'                                               { "DO" }
  | 'DUMPFILE'                                         { "DUMPFILE" }
  | 'DUPLICATE'                                        { "DUPLICATE" }
  | 'DYNAMIC'                                          { "DYNAMIC" }
  | 'ENABLE'                                           { "ENABLE" }
  | 'ENCRYPTION'                                       { "ENCRYPTION" }
  | 'END'                                              { "END" }
  | 'ENDS'                                             { "ENDS" }
  | 'ENFORCED'                                         { "ENFORCED" }
  | 'ENGINE'                                           { "ENGINE" }
  | 'ENGINES'                                          { "ENGINES" }
  | 'ENGINE_ATTRIBUTE'                                 { "ENGINE_ATTRIBUTE" }
  | 'ENUM'                                             { "ENUM" }
  | 'ERROR'                                            { "ERROR" }
  | 'ERRORS'                                           { "ERRORS" }
  | 'ESCAPE'                                           { "ESCAPE" }
  | 'EVENT'                                            { "EVENT" }
  | 'EVENTS'                                           { "EVENTS" }
  | 'EVERY'                                            { "EVERY" }
  | 'EXCHANGE'                                         { "EXCHANGE" }
  | 'EXCLUDE'                                          { "EXCLUDE" }
  | 'EXECUTE'                                          { "EXECUTE" }
  | 'EXPANSION'                                        { "EXPANSION" }
  | 'EXPIRE'                                           { "EXPIRE" }
  | 'EXPORT'                                           { "EXPORT" }
  | 'EXTENDED'                                         { "EXTENDED" }
  | 'EXTENT_SIZE'                                      { "EXTENT_SIZE" }
  | 'FACTOR'                                           { "FACTOR" }
  | 'FAILED_LOGIN_ATTEMPTS'                            { "FAILED_LOGIN_ATTEMPTS" }
  | 'FAST'                                             { "FAST" }
  | 'FAULTS'                                           { "FAULTS" }
  | 'FILE'                                             { "FILE" }
  | 'FILE_BLOCK_SIZE'                                  { "FILE_BLOCK_SIZE" }
  | 'FILTER'                                           { "FILTER" }
  | 'FINISH'                                           { "FINISH" }
  | 'FIRST'                                            { "FIRST" }
  | 'FIXED'                                            { "FIXED" }
  | 'FLUSH'                                            { "FLUSH" }
  | 'FOLLOWING'                                        { "FOLLOWING" }
  | 'FOLLOWS'                                          { "FOLLOWS" }
  | 'FORMAT'                                           { "FORMAT" }
  | 'FOUND'                                            { "FOUND" }
  | 'FULL'                                             { "FULL" }
  | 'GENERAL'                                          { "GENERAL" }
  | 'GEOMETRY'                                         { "GEOMETRY" }
  | 'GEOMETRYCOLLECTION'                               { "GEOMETRYCOLLECTION" }
  | 'GET_FORMAT'                                       { "GET_FORMAT" }
  | 'GET_MASTER_PUBLIC_KEY'                            { "GET_MASTER_PUBLIC_KEY" }
  | 'GET_SOURCE_PUBLIC_KEY'                            { "GET_SOURCE_PUBLIC_KEY" }
  | 'GRANTS'                                           { "GRANTS" }
  | 'GROUP_REPLICATION'                                { "GROUP_REPLICATION" }
  | 'GTID_ONLY'                                        { "GTID_ONLY" }
  | 'HANDLER'                                          { "HANDLER" }
  | 'HASH'                                             { "HASH" }
  | 'HELP'                                             { "HELP" }
  | 'HISTOGRAM'                                        { "HISTOGRAM" }
  | 'HISTORY'                                          { "HISTORY" }
  | 'HOST'                                             { "HOST" }
  | 'HOSTS'                                            { "HOSTS" }
  | 'HOUR'                                             { "HOUR" }
  | 'IDENTIFIED'                                       { "IDENTIFIED" }
  | 'IGNORE_SERVER_IDS'                                { "IGNORE_SERVER_IDS" }
  | 'IMPORT'                                           { "IMPORT" }
  | 'INACTIVE'                                         { "INACTIVE" }
  | 'INDEXES'                                          { "INDEXES" }
  | 'INITIAL'                                          { "INITIAL" }
  | 'INITIAL_SIZE'                                     { "INITIAL_SIZE" }
  | 'INITIATE'                                         { "INITIATE" }
  | 'INSERT_METHOD'                                    { "INSERT_METHOD" }
  | 'INSTALL'                                          { "INSTALL" }
  | 'INSTANCE'                                         { "INSTANCE" }
  //| 'INTERVAL'     %prec LOWER_THEN_INTERVAL           { "INTERVAL" }
  | 'INVISIBLE'                                        { "INVISIBLE" }
  | 'INVOKER'                                          { "INVOKER" }
  | 'IO'                                               { "IO" }
  | 'IPC'                                              { "IPC" }
  | 'ISOLATION'                                        { "ISOLATION" }
  | 'ISSUER'                                           { "ISSUER" }
  | 'JSON'                                             { "JSON" }
  | 'JSON_VALUE'                                       { "JSON_VALUE" }
  | 'KEYRING'                                          { "KEYRING" }
  | 'KEY_BLOCK_SIZE'                                   { "KEY_BLOCK_SIZE" }
  | 'LANGUAGE'                                         { "LANGUAGE" }
  | 'LAST'                                             { "LAST" }
  | 'LEAVES'                                           { "LEAVES" }
  | 'LESS'                                             { "LESS" }
  | 'LEVEL'                                            { "LEVEL" }
  | 'LINESTRING'                                       { "LINESTRING" }
  | 'LIST'                                             { "LIST" }
  | 'LOCKED'                                           { "LOCKED" }
  | 'LOCKS'                                            { "LOCKS" }
  | 'LOGFILE'                                          { "LOGFILE" }
  | 'LOGS'                                             { "LOGS" }
  | 'MASTER'                                           { "MASTER" }
  | 'MASTER_AUTO_POSITION'                             { "MASTER_AUTO_POSITION" }
  | 'MASTER_COMPRESSION_ALGORITHM'                     { "MASTER_COMPRESSION_ALGORITHM" }
  | 'MASTER_CONNECT_RETRY'                             { "MASTER_CONNECT_RETRY" }
  | 'MASTER_DELAY'                                     { "MASTER_DELAY" }
  | 'MASTER_HEARTBEAT_PERIOD'                          { "MASTER_HEARTBEAT_PERIOD" }
  | 'MASTER_HOST'                                      { "MASTER_HOST" }
  | 'MASTER_LOG_FILE'                                  { "MASTER_LOG_FILE" }
  | 'MASTER_LOG_POS'                                   { "MASTER_LOG_POS" }
  | 'MASTER_PASSWORD'                                  { "MASTER_PASSWORD" }
  | 'MASTER_PORT'                                      { "MASTER_PORT" }
  | 'MASTER_PUBLIC_KEY_PATH'                           { "MASTER_PUBLIC_KEY_PATH" }
  | 'MASTER_RETRY_COUNT'                               { "MASTER_RETRY_COUNT" }
  | 'MASTER_SSL'                                       { "MASTER_SSL" }
  | 'MASTER_SSL_CA'                                    { "MASTER_SSL_CA" }
  | 'MASTER_SSL_CAPATH'                                { "MASTER_SSL_CAPATH" }
  | 'MASTER_SSL_CERT'                                  { "MASTER_SSL_CERT" }
  | 'MASTER_SSL_CIPHER'                                { "MASTER_SSL_CIPHER" }
  | 'MASTER_SSL_CRL'                                   { "MASTER_SSL_CRL" }
  | 'MASTER_SSL_CRLPATH'                               { "MASTER_SSL_CRLPATH" }
  | 'MASTER_SSL_KEY'                                   { "MASTER_SSL_KEY" }
  | 'MASTER_TLS_CIPHERSUITES'                          { "MASTER_TLS_CIPHERSUITES" }
  | 'MASTER_TLS_VERSION'                               { "MASTER_TLS_VERSION" }
  | 'MASTER_USER'                                      { "MASTER_USER" }
  | 'MASTER_ZSTD_COMPRESSION_LEVEL'                    { "MASTER_ZSTD_COMPRESSION_LEVEL" }
  | 'MAX'                                              { "MAX" }
  | 'MAX_CONNECTIONS_PER_HOUR'                         { "MAX_CONNECTIONS_PER_HOUR" }
  | 'MAX_QUERIES_PER_HOUR'                             { "MAX_QUERIES_PER_HOUR" }
  | 'MAX_ROWS'                                         { "MAX_ROWS" }
  | 'MAX_SIZE'                                         { "MAX_SIZE" }
  | 'MAX_UPDATES_PER_HOUR'                             { "MAX_UPDATES_PER_HOUR" }
  | 'MAX_USER_CONNECTIONS'                             { "MAX_USER_CONNECTIONS" }
  | 'MEDIUM'                                           { "MEDIUM" }
  | 'MEMORY'                                           { "MEMORY" }
  | 'MERGE'                                            { "MERGE" }
  | 'MESSAGE_TEXT'                                     { "MESSAGE_TEXT" }
  | 'MICROSECOND'                                      { "MICROSECOND" }
  | 'MIGRATE'                                          { "MIGRATE" }
  | 'MIN'                                              { "MIN" }
  | 'MINUTE'                                           { "MINUTE" }
  | 'MIN_ROWS'                                         { "MIN_ROWS" }
  | 'MODE'                                             { "MODE" }
  | 'MODIFY'                                           { "MODIFY" }
  | 'MONTH'                                            { "MONTH" }
  | 'MULTILINESTRING'                                  { "MULTILINESTRING" }
  | 'MULTIPOINT'                                       { "MULTIPOINT" }
  | 'MULTIPOLYGON'                                     { "MULTIPOLYGON" }
  | 'MUTEX'                                            { "MUTEX" }
  | 'MYSQL_ERRNO'                                      { "MYSQL_ERRNO" }
  | 'NAME'                                             { "NAME" }
  | 'NAMES'                                            { "NAMES" }
  | 'NATIONAL'                                         { "NATIONAL" }
  | 'NCHAR'                                            { "NCHAR" }
  | 'NDBCLUSTER'                                       { "NDBCLUSTER" }
  | 'NESTED'                                           { "NESTED" }
  | 'NETWORK_NAMESPACE'                                { "NETWORK_NAMESPACE" }
  | 'NEVER'                                            { "NEVER" }
  | 'NEW'                                              { "NEW" }
  | 'NEXT'                                             { "NEXT" }
  | 'NO'                                               { "NO" }
  | 'NODEGROUP'                                        { "NODEGROUP" }
  | 'NONE'                                             { "NONE" }
  | 'NOWAIT'                                           { "NOWAIT" }
  | 'NO_WAIT'                                          { "NO_WAIT" }
  | 'NULLS'                                            { "NULLS" }
  | 'NUMBER'                                           { "NUMBER" }
  | 'NVARCHAR'                                         { "NVARCHAR" }
  | 'OFF'                                              { "OFF" }
  | 'OFFSET'                                           { "OFFSET" }
  | 'OJ'                                               { "OJ" }
  | 'OLD'                                              { "OLD" }
  | 'ONE'                                              { "ONE" }
  | 'ONLY'                                             { "ONLY" }
  | 'OPEN'                                             { "OPEN" }
  | 'OPTIONAL'                                         { "OPTIONAL" }
  | 'OPTIONS'                                          { "OPTIONS" }
  | 'ORDINALITY'                                       { "ORDINALITY" }
  | 'ORGANIZATION'                                     { "ORGANIZATION" }
  | 'OTHERS'                                           { "OTHERS" }
  | 'OWNER'                                            { "OWNER" }
  | 'PACK_KEYS'                                        { "PACK_KEYS" }
  | 'PAGE'                                             { "PAGE" }
  | 'PARSER'                                           { "PARSER" }
  | 'PARTIAL'                                          { "PARTIAL" }
  | 'PARTITIONING'                                     { "PARTITIONING" }
  | 'PARTITIONS'                                       { "PARTITIONS" }
  | 'PASSWORD'                                         { "PASSWORD" }
  | 'PASSWORD_LOCK_TIME'                               { "PASSWORD_LOCK_TIME" }
  | 'PATH'                                             { "PATH" }
  | 'PHASE'                                            { "PHASE" }
  | 'PLUGIN'                                           { "PLUGIN" }
  | 'PLUGINS'                                          { "PLUGINS" }
  | 'PLUGIN_DIR'                                       { "PLUGIN_DIR" }
  | 'POINT'                                            { "POINT" }
  | 'POLYGON'                                          { "POLYGON" }
  | 'PORT'                                             { "PORT" }
  | 'POSITION'                                         { "POSITION" }
  | 'PRECEDES'                                         { "PRECEDES" }
  | 'PRECEDING'                                        { "PRECEDING" }
  | 'PREPARE'                                          { "PREPARE" }
  | 'PRESERVE'                                         { "PRESERVE" }
  | 'PREV'                                             { "PREV" }
  | 'PRIVILEGES'                                       { "PRIVILEGES" }
  | 'PRIVILEGE_CHECKS_USER'                            { "PRIVILEGE_CHECKS_USER" }
  | 'PROCESS'                                          { "PROCESS" }
  | 'PROCESSLIST'                                      { "PROCESSLIST" }
  | 'PROFILE'                                          { "PROFILE" }
  | 'PROFILES'                                         { "PROFILES" }
  | 'PROXY'                                            { "PROXY" }
  | 'QUARTER'                                          { "QUARTER" }
  | 'QUERY'                                            { "QUERY" }
  | 'QUICK'                                            { "QUICK" }
  | 'RANDOM'                                           { "RANDOM" }
  | 'RANK'                                             { "RANK" }
  | 'READ_ONLY'                                        { "READ_ONLY" }
  | 'REBUILD'                                          { "REBUILD" }
  | 'RECOVER'                                          { "RECOVER" }
  | 'REDO_BUFFER_SIZE'                                 { "REDO_BUFFER_SIZE" }
  | 'REDUNDANT'                                        { "REDUNDANT" }
  | 'REFERENCE'                                        { "REFERENCE" }
  | 'REGISTRATION'                                     { "REGISTRATION" }
  | 'RELAY'                                            { "RELAY" }
  | 'RELAYLOG'                                         { "RELAYLOG" }
  | 'RELAY_LOG_FILE'                                   { "RELAY_LOG_FILE" }
  | 'RELAY_LOG_POS'                                    { "RELAY_LOG_POS" }
  | 'RELAY_THREAD'                                     { "RELAY_THREAD" }
  | 'RELOAD'                                           { "RELOAD" }
  | 'REMOVE'                                           { "REMOVE" }
  | 'REORGANIZE'                                       { "REORGANIZE" }
  | 'REPAIR'                                           { "REPAIR" }
  | 'REPEATABLE'                                       { "REPEATABLE" }
  | 'REPLICA'                                          { "REPLICA" }
  | 'REPLICAS'                                         { "REPLICAS" }
  | 'REPLICATE_DO_DB'                                  { "REPLICATE_DO_DB" }
  | 'REPLICATE_DO_TABLE'                               { "REPLICATE_DO_TABLE" }
  | 'REPLICATE_IGNORE_DB'                              { "REPLICATE_IGNORE_DB" }
  | 'REPLICATE_IGNORE_TABLE'                           { "REPLICATE_IGNORE_TABLE" }
  | 'REPLICATE_REWRITE_DB'                             { "REPLICATE_REWRITE_DB" }
  | 'REPLICATE_WILD_DO_TABLE'                          { "REPLICATE_WILD_DO_TABLE" }
  | 'REPLICATE_WILD_IGNORE_TABLE'                      { "REPLICATE_WILD_IGNORE_TABLE" }
  | 'REPLICATION'                                      { "REPLICATION" }
  | 'REQUIRE_ROW_FORMAT'                               { "REQUIRE_ROW_FORMAT" }
  | 'REQUIRE_TABLE_PRIMARY_KEY_CHECK'                  { "REQUIRE_TABLE_PRIMARY_KEY_CHECK" }
  | 'RESET'                                            { "RESET" }
  | 'RESOURCE'                                         { "RESOURCE" }
  | 'RESOURCES'                                        { "RESOURCES" }
  | 'RESPECT'                                          { "RESPECT" }
  | 'RESTART'                                          { "RESTART" }
  | 'RESTORE'                                          { "RESTORE" }
  | 'RESUME'                                           { "RESUME" }
  | 'RETAIN'                                           { "RETAIN" }
  | 'RETURNED_SQLSTATE'                                { "RETURNED_SQLSTATE" }
  | 'RETURNING'                                        { "RETURNING" }
  | 'RETURNS'                                          { "RETURNS" }
  | 'REUSE'                                            { "REUSE" }
  | 'REVERSE'                                          { "REVERSE" }
  | 'ROLE'                                             { "ROLE" }
  | 'ROLLBACK'                                         { "ROLLBACK" }
  | 'ROLLUP'                                           { "ROLLUP" }
  | 'ROTATE'                                           { "ROTATE" }
  | 'ROUTINE'                                          { "ROUTINE" }
  | 'ROW_COUNT'                                        { "ROW_COUNT" }
  | 'ROW_FORMAT'                                       { "ROW_FORMAT" }
  | 'RTREE'                                            { "RTREE" }
  | 'SAVEPOINT'                                        { "SAVEPOINT" }
  | 'SCHEDULE'                                         { "SCHEDULE" }
  | 'SCHEMA_NAME'                                      { "SCHEMA_NAME" }
  | 'SECOND'                                           { "SECOND" }
  | 'SECONDARY'                                        { "SECONDARY" }
  | 'SECONDARY_ENGINE'                                 { "SECONDARY_ENGINE" }
  | 'SECONDARY_ENGINE_ATTRIBUTE'                       { "SECONDARY_ENGINE_ATTRIBUTE" }
  | 'SECONDARY_LOAD'                                   { "SECONDARY_LOAD" }
  | 'SECONDARY_UNLOAD'                                 { "SECONDARY_UNLOAD" }
  | 'SECURITY'                                         { "SECURITY" }
  | 'SERIAL'                                           { "SERIAL" }
  | 'SERIALIZABLE'                                     { "SERIALIZABLE" }
  | 'SERVER'                                           { "SERVER" }
  | 'SHARE'                                            { "SHARE" }
  | 'SHUTDOWN'                                         { "SHUTDOWN" }
  | 'SIGNED'                                           { "SIGNED" }
  | 'SIMPLE'                                           { "SIMPLE" }
  | 'SKIP'                                             { "SKIP" }
  | 'SLAVE'                                            { "SLAVE" }
  | 'SLOW'                                             { "SLOW" }
  | 'SNAPSHOT'                                         { "SNAPSHOT" }
  | 'SOCKET'                                           { "SOCKET" }
  | 'SONAME'                                           { "SONAME" }
  | 'SOUNDS'                                           { "SOUNDS" }
  | 'SOURCE'                                           { "SOURCE" }
  | 'SOURCE_AUTO_POSITION'                             { "SOURCE_AUTO_POSITION" }
  | 'SOURCE_BIND'                                      { "SOURCE_BIND" }
  | 'SOURCE_COMPRESSION_ALGORITHM'                     { "SOURCE_COMPRESSION_ALGORITHM" }
  | 'SOURCE_CONNECTION_AUTO_FAILOVER'                  { "SOURCE_CONNECTION_AUTO_FAILOVER" }
  | 'SOURCE_CONNECT_RETRY'                             { "SOURCE_CONNECT_RETRY" }
  | 'SOURCE_DELAY'                                     { "SOURCE_DELAY" }
  | 'SOURCE_HEARTBEAT_PERIOD'                          { "SOURCE_HEARTBEAT_PERIOD" }
  | 'SOURCE_HOST'                                      { "SOURCE_HOST" }
  | 'SOURCE_LOG_FILE'                                  { "SOURCE_LOG_FILE" }
  | 'SOURCE_LOG_POS'                                   { "SOURCE_LOG_POS" }
  | 'SOURCE_PASSWORD'                                  { "SOURCE_PASSWORD" }
  | 'SOURCE_PORT'                                      { "SOURCE_PORT" }
  | 'SOURCE_PUBLIC_KEY_PATH'                           { "SOURCE_PUBLIC_KEY_PATH" }
  | 'SOURCE_RETRY_COUNT'                               { "SOURCE_RETRY_COUNT" }
  | 'SOURCE_SSL'                                       { "SOURCE_SSL" }
  | 'SOURCE_SSL_CA'                                    { "SOURCE_SSL_CA" }
  | 'SOURCE_SSL_CAPATH'                                { "SOURCE_SSL_CAPATH" }
  | 'SOURCE_SSL_CERT'                                  { "SOURCE_SSL_CERT" }
  | 'SOURCE_SSL_CIPHER'                                { "SOURCE_SSL_CIPHER" }
  | 'SOURCE_SSL_CRL'                                   { "SOURCE_SSL_CRL" }
  | 'SOURCE_SSL_CRLPATH'                               { "SOURCE_SSL_CRLPATH" }
  | 'SOURCE_SSL_KEY'                                   { "SOURCE_SSL_KEY" }
  | 'SOURCE_SSL_VERIFY_SERVER_CERT'                    { "SOURCE_SSL_VERIFY_SERVER_CERT" }
  | 'SOURCE_TLS_CIPHERSUITES'                          { "SOURCE_TLS_CIPHERSUITES" }
  | 'SOURCE_TLS_VERSION'                               { "SOURCE_TLS_VERSION" }
  | 'SOURCE_USER'                                      { "SOURCE_USER" }
  | 'SOURCE_ZSTD_COMPRESSION_LEVEL'                    { "SOURCE_ZSTD_COMPRESSION_LEVEL" }
  | 'SQL_AFTER_GTIDS'                                  { "SQL_AFTER_GTIDS" }
  | 'SQL_AFTER_MTS_GAPS'                               { "SQL_AFTER_MTS_GAPS" }
  | 'SQL_BEFORE_GTIDS'                                 { "SQL_BEFORE_GTIDS" }
  | 'SQL_THREAD'                                       { "SQL_THREAD" }
  | 'SRID'                                             { "SRID" }
  | 'STACKED'                                          { "STACKED" }
  | 'START'                                            { "START" }
  | 'STARTS'                                           { "STARTS" }
  | 'STATS_AUTO_RECALC'                                { "STATS_AUTO_RECALC" }
  | 'STATS_PERSISTENT'                                 { "STATS_PERSISTENT" }
  | 'STATS_SAMPLE_PAGES'                               { "STATS_SAMPLE_PAGES" }
  | 'STATUS'                                           { "STATUS" }
  | 'STD'                                              { "STD" }
  | 'STDDEV'                                           { "STDDEV" }
  | 'STDDEV_POP'                                       { "STDDEV_POP" }
  | 'STDDEV_SAMP'                                      { "STDDEV_SAMP" }
  | 'STOP'                                             { "STOP" }
  | 'STORAGE'                                          { "STORAGE" }
  | 'STREAM'                                           { "STREAM" }
  | 'ST_COLLECT'                                       { "ST_COLLECT" }
  | 'SUBCLASS_ORIGIN'                                  { "SUBCLASS_ORIGIN" }
  | 'SUBDATE'                                          { "SUBDATE" }
  | 'SUBJECT'                                          { "SUBJECT" }
  | 'SUBPARTITION'                                     { "SUBPARTITION" }
  | 'SUBPARTITIONS'                                    { "SUBPARTITIONS" }
  | 'SUM'                                              { "SUM" }
  | 'SUPER'                                            { "SUPER" }
  | 'SUSPEND'                                          { "SUSPEND" }
  | 'SWAPS'                                            { "SWAPS" }
  | 'SWITCHES'                                         { "SWITCHES" }
  | 'TABLES'                                           { "TABLES" }
  | 'TABLESPACE'                                       { "TABLESPACE" }
  | 'TABLE_CHECKSUM'                                   { "TABLE_CHECKSUM" }
  | 'TABLE_NAME'                                       { "TABLE_NAME" }
  | 'TEMPORARY'                                        { "TEMPORARY" }
  | 'TEMPTABLE'                                        { "TEMPTABLE" }
  | 'TEXT'                                             { "TEXT" }
  | 'THAN'                                             { "THAN" }
  | 'THREAD_PRIORITY'                                  { "THREAD_PRIORITY" }
  | 'TIES'                                             { "TIES" }
  | 'TIME'  %prec LOWER_THEN_TEXT_STRING               { "TIME" }
  | 'TIMESTAMP_ADD'                                    { "TIMESTAMP_ADD" }
  | 'TIMESTAMP_DIFF'                                   { "TIMESTAMP_DIFF" }
  | 'TLS'                                              { "TLS" }
  | 'TRANSACTION'                                      { "TRANSACTION" }
  | 'TRIGGERS'                                         { "TRIGGERS" }
  | 'TRUNCATE'                                         { "TRUNCATE" }
  | 'TYPE'                                             { "TYPE" }
  | 'TYPES'                                            { "TYPES" }
  | 'UNBOUNDED'                                        { "UNBOUNDED" }
  | 'UNCOMMITTED'                                      { "UNCOMMITTED" }
  | 'UNDEFINED'                                        { "UNDEFINED" }
  | 'UNDOFILE'                                         { "UNDOFILE" }
  | 'UNDO_BUFFER_SIZE'                                 { "UNDO_BUFFER_SIZE" }
  | 'UNICODE'                                          { "UNICODE" }
  | 'UNINSTALL'                                        { "UNINSTALL" }
  | 'UNKNOWN'                                          { "UNKNOWN" }
  | 'UNREGISTER'                                       { "UNREGISTER" }
  | 'UNTIL'                                            { "UNTIL" }
  | 'UPGRADE'                                          { "UPGRADE" }
  | 'USER'                                             { "USER" }
  | 'USE_FRM'                                          { "USE_FRM" }
  | 'VALIDATION'                                       { "VALIDATION" }
  | 'VALUE'                                            { "VALUE" }
  | 'VARIABLES'                                        { "VARIABLES" }
  | 'VARIANCE'                                         { "VARIANCE" }
  | 'VCPU'                                             { "VCPU" }
  | 'VIEW'                                             { "VIEW" }
  | 'VISIBLE'                                          { "VISIBLE" }
  | 'WAIT'                                             { "WAIT" }
  | 'WARNINGS'                                         { "WARNINGS" }
  | 'WEEK'                                             { "WEEK" }
  | 'WEIGHT_STRING'                                    { "WEIGHT_STRING" }
  | 'WITHOUT'                                          { "WITHOUT" }
  | 'WORK'                                             { "WORK" }
  | 'WRAPPER'                                          { "WRAPPER" }
  | 'X509'                                             { "X509" }
  | 'XA'                                               { "XA" }
  | 'XID'                                              { "XID" }
  | 'XML'                                              { "XML" }
  | 'YEAR'                                             { "YEAR" }
  | 'ZONE'                                             { "ZONE" }
  ;

order_expr -> OrderExpr:
    expr opt_ordering_direction
    {
      OrderExpr {
        span: $span,
        expr: $1,
        direction: $2
      }
    }
  ;

grouping_expr -> Expr:
    expr
    {
      $1
    }
  ;

simple_ident -> Value:
   ident
   {
      Value::Ident {
        span: $span,
        value: $1.0,
        quoted: $1.1
      }
   }
 | simple_ident_q
   {
     $1
   }
 ;

simple_ident_nospvar -> Value: 
    ident
    {
      Value::Ident {
        span: $span,
        value: $1.0,
        quoted: $1.1,
      }
    }
  | simple_ident_q
    {
      $1
    }
  ;

simple_ident_q -> Value:
    ident '.' ident
    {
      Value::TableIdent {
        span: $span,
        table: $1.0,
        field: $3.0,
        schema: None
      }
    }
  | ident '.' ident '.' ident
    {
      Value::TableIdent {
        span: $span,
        schema: Some($1.0),
        table: $3.0,
        field: $5.0,
      }
    }
  ;

ident_or_text -> String:
    ident           { $1.0 }
  | 'TEXT_STRING'
    {   
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  ;

role_ident_or_text -> String:
    role_ident           { $1.0 }
  | 'TEXT_STRING'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  ;

literal -> Value:
    text_literal  %prec 'LOWER_THEN_TEXT_STRING' 
    { 
      $1
    }
  | NUM_literal  
    { 
      $1 
    }
  | 'FALSE'
    {
      Value::False
    }
  | 'TRUE'
    {
      Value::True
    }
  | 'HEX_NUM'
    {
      Value::HexNum {
        span: $span,
        value: String::from($lexer.span_str($1.as_ref().unwrap().span()))
      }
    }
  | 'BIN_NUM'
    {
      Value::BinNum {
        span: $span,
        value: String::from($lexer.span_str($1.as_ref().unwrap().span()))
      }
    }
  | 'UNDERSCORE_CHARSET' 'HEX_NUM'
    {
      Value::HexNum {
        span: $span,
        value: String::from($lexer.span_str($2.as_ref().unwrap().span()))
      }
    }
  | 'UNDERSCORE_CHARSET' 'BIN_NUM'
    {
      Value::HexNum {
        span: $span,
        value: String::from($lexer.span_str($2.as_ref().unwrap().span()))
      }
    }
  ;


literal_or_null -> Value:
    literal 
    { 
      $1
    }
  | 'NULL'
    { 
      Value::Null
    }
  ;

NUM_literal -> Value:
    int64_literal
    {
      Value::Num {
        span: $span,
        value: $1,
        signed: false,
      }
    }
  | 'DECIMAL_NUM'
    {
      Value::Num {
        span: $span,
        value: String::from($lexer.span_str($1.as_ref().unwrap().span())),
        signed: false,
      }
    }
  | 'FLOAT_NUM'
    {
      Value::FloatNum {
        span: $span,
        value: String::from($lexer.span_str($1.as_ref().unwrap().span())),
        signed: false,
      }
    }
  ;

int64_literal -> String:
    'NUM'           
    { 
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  | 'LONG_NUM'      
    { 
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  | 'ULONGLONG_NUM' 
    { 
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  ;


opt_interval -> Option<String>:
    /* empty */   { None }
  | 'INTERVAL'    { Some(String::from("INTERVAL")) }
  ;

type_datetime_precision -> Option<String>:
    /* empty */                { None }
  | '(' 'NUM' ')'              
    { 
      Some(String::from($lexer.span_str($2.as_ref().unwrap().span())).to_string())
    }
  ;

func_datetime_precision -> Option<String>:
          /* empty */                { None }
        | '(' ')'                    { Some(String::from("()")) }
        | '(' 'NUM' ')'
           {
              Some(String::from($lexer.span_str($2.as_ref().unwrap().span())).to_string())
           }
        ;

charset_name -> String:
    ident_or_text 
    {
      $1
    }
  | 'BINARY' 
    { 
      String::from("BINARY")
    }
  ;

text_literal -> Value:
    'TEXT_STRING'
    {
      Value::Text {
        span: $span,
        value: String::from($lexer.span_str($1.as_ref().unwrap().span())),
      }
    }
  | 'NCHAR_STRING'
    {
      Value::Text {
        span: $span,
        value: String::from($lexer.span_str($1.as_ref().unwrap().span())),
      }
    }
  | 'UNDERSCORE_CHARSET' 'TEXT_STRING'
    {
      Value::Text {
        span: $span,
        value: String::from($lexer.span_str($1.as_ref().unwrap().span())),
      }
    }
  | text_literal 'TEXT_STRING'
    {
      let t = match $1 {
        Value::Text { span: _, value } => value, 
        _ => unreachable!()
      };

      let value = $lexer.span_str($2.as_ref().unwrap().span());
      let mut s = String::with_capacity(t.len()+value.len());
      s.push_str(&t);
      s.push_str(value);
      Value::TextN {
        span: $span,
        value: s,
      }
    }
  ;

text_binary_string -> String:
    'TEXT_STRING'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  | 'HEX_NUM'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  | 'BIN_NUM'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
  ;

now -> String:
  'NOW' func_datetime_precision
    {
      let mut s = String::with_capacity(30);
      s.push_str("NOW ");
      if let Some(precision) = $2 {
        s.push_str(precision.as_str());
      }
      s
    }
  ;

param_marker -> String:
    'PARAM_MARKER'
    {
      String::from("?")
    }
  ;

ws_num_codepoints -> String:
  '(' real_ulong_num ')'
  {
	$2
  }
  ;

opt_var_ident_type -> Option<String>:
    /* empty */     
    { 
      None
    }
  | 'GLOBAL' '.'  
    { 
      Some(String::from("GLOBAL."))
    }
  | 'LOCAL' '.'   
    { 
      Some(String::from("LOCAL.")) 
    }
  | 'SESSION' '.' 
    { 
      Some(String::from("SESSION."))
    }
  ;

opt_component -> Option<String>:
    /* empty */    { None }
  | '.' ident      
    { 
      let mut s = String::with_capacity(1+$2.0.len());
      s.push('.');
      s.push_str(&$2.0);
      Some(s)
    }
  ;

char -> &'input str:
  	"CHARACTER" { "CHARACTER" }
  |	"CHAR"    { "CHAR" }
  ;

character_set -> String:
    'CHAR' 'SET' { String::from("CHAR SET") }
  | 'CHARACTER' 'SET' { String::from("CHARACTER SET") }
  | 'CHARSET' { String::from("CHARSET") }
  ;

opt_load_data_charset -> Option<CharsetName>:
    /* Empty */ { None }
  | character_set charset_name 
    { 
      Some(CharsetName{
        span: $span,
        set_type: $1,
        name: $2
      })
    }
  ;

field_options -> Vec<String>:
    /* empty */ { vec![] }
  | field_opt_list { $1 }
  ;

field_opt_list -> Vec<String>:
    field_opt_list field_option
    {
      $1.push(String::from($2));
      $1
    }
  | field_option
    {
      vec![String::from($1)]
    }
  ;

field_option -> &'input str:
    'SIGNED'   { "SIGNED"   } 
  | 'UNSIGNED' { "UNSIGNED" }
  | 'ZEROFILL' { "ZEROFILL" }
  ;

// add handle for sign
signed_literal -> Value:
    literal   
    { 
      $1 
    }
  | '+' NUM_literal 
    { 
      $2
    }
  | '-' NUM_literal
    {
      match $2 {
        Value::Num { span, value, signed:_ } => {
          Value::Num { span, value, signed: true}
        },

        _ => unreachable!()
      }
    }
  ;

key_or_index -> &'input str:
    'KEY'   { "KEY" }
  | 'INDEX' { "INDEX" }
  ;

field_length -> String:
      '(' 'LONG_NUM' ')'      
      { 
        String::from($lexer.span_str($2.as_ref().unwrap().span()))
      }
    | '(' 'ULONGLONG_NUM' ')' 
      { 
        String::from($lexer.span_str($2.as_ref().unwrap().span()))
      }
    | '(' 'DECIMAL_NUM' ')'   
      { 
        String::from($lexer.span_str($2.as_ref().unwrap().span()))
      }
    | '(' 'NUM' ')' 
      { 
        String::from($lexer.span_str($2.as_ref().unwrap().span()))
      }
    ;

opt_field_length -> Option<String>:
    /* empty */   { None }
  | field_length  { Some($1) }
  ;

opt_field_term -> Option<FieldTermColumn>:
    /* empty */             { None }
  | fields_columns field_term_list
    { 
      Some(FieldTermColumn {
        span: $span,
        column_type: String::from($1),
        terms: $2,
      })
    }
  ;

fields_columns -> &'input str:
    'COLUMNS' { "COLUMNS" }
  | 'FIELDS'  { "FIELDS"  }
  ;

field_term_list -> Vec<FieldTerm>:
    field_term_list field_term
    {
      $1.push($2);
      $1
    }
  | field_term 
    {
      vec![$1]
    }
  ;

field_term -> FieldTerm:
    'TERMINATED' 'BY' text_binary_string
    {
      FieldTerm {
        span: $span,
        term_type: String::from("TERMINATED"),
        value: $3
      }
    }
  | 'OPTIONALLY' 'ENCLOSED' 'BY' text_binary_string
    {
      FieldTerm {
        span: $span,
        term_type: String::from("OPTIONALLY ENCLOSED"),
        value: $4
      }
    }
  | 'ENCLOSED' 'BY' text_binary_string
    {
      FieldTerm {
        span: $span,
        term_type: String::from("ENCLOSED"),
        value: $3
      }
    }
  | 'ESCAPED' 'BY' text_binary_string
    {
      FieldTerm {
        span: $span,
        term_type: String::from("ESCAPED"),
        value: $3
      }
    }
  ;

opt_line_term -> Vec<LineTerm>:
    /* empty */             { vec![] }
  | 'LINES' line_term_list  { $2 }
  ;

line_term_list -> Vec<LineTerm>:
    line_term_list line_term
    {
      $1.push($2);
      $1
    }
  | line_term
    {
      vec![$1]
    }
  ;

line_term -> LineTerm:
    'TERMINATED' 'BY' text_binary_string
    {
      LineTerm {
        span: $span,
        term_type: String::from("TERMINATED"),
        value: $3
      }
    }
  | 'STARTING' 'BY' text_binary_string
    {
      LineTerm {
        span: $span,
        term_type: String::from("STARTING"),
        value: $3
      }
    }
  ;

collation_name -> String:
    ident_or_text
    {
      $1
    }
  | 'BINARY' 
    { 
      String::from("BINARY")
    }
  ;

opt_collate -> Option<String>:
      /* empty */                { None }
    | 'COLLATE' collation_name   { Some($2) }
    ;

opt_default -> Option<String>:
      /* empty */ { None }
    | 'DEFAULT' { Some(String::from("DEFAULT")) }
    ;

opt_charset_with_opt_binary -> CharsetOrBinary:
    /* empty */
    {
      CharsetOrBinary::None
    }
  | 'ASCII'
    {
      CharsetOrBinary::Ascii
    }
  | 'UNICODE'
    {
      CharsetOrBinary::Unicode
    }
  | 'BYTE'
    {
      CharsetOrBinary::Byte
    }
  | character_set charset_name opt_bin_mod
    {
      CharsetOrBinary::Charset {
        set_type: $1,
        name: $2,
        bin_mod: $3.is_some(), 
      }
    }
  | 'BINARY'
    {
      CharsetOrBinary::Binary {
        set_type: None,
        name: None,
      }
    }
  | 'BINARY' character_set charset_name
    {
      CharsetOrBinary::Binary {
        set_type: Some($2),
        name: Some($3),
      }
    }
  ;

opt_bin_mod -> Option<String>:
    /* empty */ { None }
  | 'BINARY' 
    {
      Some(String::from("BINARY"))
    }
  ;
  
nchar -> String:
    'NCHAR' { String::from("NCHAR") }
  | 'NATIONAL' char 
    { 
      let mut s = String::with_capacity($2.len()+1+8);
      s.push_str("NATIONAL");
      s.push(' ');
      s.push_str($2);
      s
    }
  ;

varchar -> String:
    char 'VARYING' 
    {
      let mut s = String::with_capacity($1.len()+1+7);
      s.push_str(&$1);
      s.push(' ');
      s.push_str("VARYING");
      s
    }
  | 'VARCHAR'
    {
      String::from("VARCHAR")
    }
  ;

nvarchar -> String:
    'NATIONAL' 'VARCHAR'
    {
      String::from("NATIONAL VARCHAR")
    }
  | 'NVARCHAR' 
    { 
      String::from("NVARCHAR")
    }
  | 'NCHAR' 'VARCHAR' 
    {
      String::from("NCHAR VARCHAR")
    }
  | 'NATIONAL' char 'VARYING' 
    { 
      let mut s = String::with_capacity(30);
      s.push_str("NATIONAL ");
      s.push_str($2);
      s.push_str(" VARYING");
      s
    }
  | 'NCHAR' 'VARYING' 
    {
      String::from("NCHAR VARYING")
    }
  ;

opt_precision_ident -> Option<String>:
    /* empty */ { None }
  | 'PRECISION' { Some(String::from("PRECISION")) }
  ;

opt_precision -> Option<String>:
    /* empty */
    {
      None
    }
  | precision   { Some($1) }
  ;

int_type -> &'input str:
    'INT'       {  "INT" }
  | 'TINYINT'   {  "TINYINT" }
  | 'SMALLINT'  {  "SMALLINT" }
  | 'MEDIUMINT' {  "MEDIUMINT" }
  | 'BIGINT'    {  "BIGINT" }
  ;

real_type -> String:
    'REAL'
    {
      String::from("REAL")
    }
  | 'DOUBLE' opt_precision_ident
    { 
          let mut s = String::new();
          s.push_str("DOUBLE ");
          if let Some(ident) = $2 {
            s.push_str(ident.as_str());
          }
          s
    }
  ;


numeric_type -> &'input str:
    'FLOAT'   { "FLOAT" }
  | 'DECIMAL' { "DECIMAL" }
  | 'NUMERIC' { "NUMERIC" }
  | 'FIXED'   { "FIXED" }
  ;

standard_float_options -> Option<String>:
    /* empty */
    {
      None
    }
  | field_length
    {
      Some($1)
    }
  ;

float_options -> Option<String>:
    /* empty */
    {
      None
    }
  | field_length
    {
      Some($1.to_string())
    }
  | precision
    {
      Some($1)
    }
  ;

precision -> String:
    '(' NUM ',' NUM ')'
    {
      let span_str1 = $lexer.span_str($2.as_ref().unwrap().span());
      let span_str2 = $lexer.span_str($4.as_ref().unwrap().span());
      let mut s = String::with_capacity(1+span_str1.len()+span_str2.len());
      s.push_str(span_str1);
      s.push('.');
      s.push_str(span_str2);
      s
    }
  ;

type -> FieldType:
    int_type opt_field_length field_options
    {
      FieldType::FieldIntType(FieldIntType {
        span: $span,
        name: String::from($1),
        length: $2,
        opts: $3
      })
    }
  | real_type opt_precision field_options
    {
      FieldType::FieldRealType(FieldRealType {
        span: $span,
        name: $1,
        precision: $2,
        opts: $3
      })
    }
  | numeric_type float_options field_options
    {
      FieldType::FieldNumericType(FieldNumericType {
        span: $span,
        name: String::from($1),
        float_opts: $2,
        opts: $3
      })
    }
  | 'BIT' 
    {
      FieldType::FieldBitType(FieldBitType {
        span: $span,
        length: None
      })
    }
  | 'BIT' field_length
    {
      FieldType::FieldBitType(FieldBitType {
        span: $span,
        length: Some($2)
      })
    }
  | 'BOOL'
    {
      FieldType::FieldBoolType
    }
  | 'BOOLEAN'
    {
      FieldType::FieldBooleanType
    }
  | char field_length opt_charset_with_opt_binary
    {
      FieldType::FieldCharType(FieldCharType {
        span: $span,
        name: String::from($1),
        length: Some($2),
        opt: $3
      })
    }
  | char opt_charset_with_opt_binary
    {
      FieldType::FieldCharType(FieldCharType {
        span: $span,
        name: String::from($1),
        length: None,
        opt: $2
      })
    }
  | nchar field_length opt_bin_mod
    {
      FieldType::FieldNCharType(FieldNCharType {
        span: $span,
        name: $1,
        length: Some($2),
        bin_mod: $3
      })
    }
  | nchar opt_bin_mod
    {
      FieldType::FieldNCharType(FieldNCharType {
        span: $span,
        name: $1,
        length: None,
        bin_mod: $2
      })
    }
  | 'BINARY' field_length
    {
      FieldType::FieldBinaryType(FieldBinaryType {
        span: $span,
        length: Some($2),
      })
    }
  | 'BINARY'
    {
      FieldType::FieldBinaryType(FieldBinaryType {
        span: $span,
        length: None,
      })
    }
  | varchar field_length opt_charset_with_opt_binary
    {
      FieldType::FieldVarCharType(FieldVarCharType {
        span: $span,
        name: $1,
        length: Some($2),
        opt: $3
      })
    }
  | nvarchar field_length opt_bin_mod
    {
      FieldType::FieldNVarCharType(FieldNVarCharType {
        span: $span,
        name: $1,
        length: Some($2),
        bin_mod: $3
      })
    }
  | 'VARBINARY' field_length
    {
      FieldType::FieldVarBinaryType(FieldVarBinaryType {
        span: $span,
        length: $2
      })
    }
  | 'YEAR' opt_field_length field_options
    {
      FieldType::FieldYearType(FieldYearType {
        span: $span,
        length: $2,
        opts: $3
      })
    }
  | 'DATE'
    {
      FieldType::FieldDateType
    }
  | 'TIME' type_datetime_precision
    {
      FieldType::FieldTimeType(FieldTimeType {
        span: $span,
        name: String::from("TIME"),
        precision: $2
      })
    }
  | 'TIMESTAMP' type_datetime_precision
    {
      FieldType::FieldTimeType(FieldTimeType {
        span: $span,
        name: String::from("TIMESTAMP"),
        precision: $2 
      })
    }
  | 'DATETIME' type_datetime_precision
    {
      FieldType::FieldTimeType(FieldTimeType {
        span: $span,
        name: String::from("TIMESTAMP"),
        precision: $2 
      })
    }
  | 'TINYBLOB'
    {
      FieldType::FieldBlobType(FieldBlobType {
        span: $span,
        name: String::from("TINYBLOB"),
        length: None,
        varchar: None,
        opt: CharsetOrBinary::None
      })
    }
  | 'BLOB' opt_field_length
    {
      FieldType::FieldBlobType(FieldBlobType {
        span: $span,
        name: String::from("BLOB"),
        length: $2,
        varchar: None,
        opt: CharsetOrBinary::None,
      })
    }
  | spatial_type 
    {
      FieldType::FieldSpatialType(String::from($1))
    }
  | 'MEDIUMBLOB'
    {
      FieldType::FieldBlobType(FieldBlobType {
        span: $span,
        name: String::from("MEDIUMBLOB"),
        length: None,
        varchar: None,
        opt: CharsetOrBinary::None,
      })
    }
  | 'LONGBLOB'
    {
      FieldType::FieldBlobType(FieldBlobType {
        span: $span,
        name: String::from("LONGBLOB"),
        length: None,
        varchar: None,
        opt: CharsetOrBinary::None,
      })
    }
  | 'LONG' 'VARBINARY'
    {
      FieldType::FieldBlobType(FieldBlobType {
        span: $span,
        name: String::from("MEDIUMBLOB"),
        length: None,
        opt: CharsetOrBinary::None,
        varchar: None
      })
    }
  | 'LONG' varchar opt_charset_with_opt_binary
    {
      FieldType::FieldBlobType(FieldBlobType {
        span: $span,
        name: String::from("MEDIUMBLOB"),
        varchar: Some($2),
        opt: $3,
        length: None
      })
    }
  | 'TINYTEXT_SYN' opt_charset_with_opt_binary
    {
      FieldType::FieldTextType(FieldTextType {
        span: $span,
        name: String::from("TINYBLOB"),
        opt: $2,
        length: None
      })
    }
  | 'TEXT' opt_field_length opt_charset_with_opt_binary
    {
      FieldType::FieldTextType(FieldTextType {
        span: $span,
        name: String::from("BLOB"),
        length: $2,
        opt: $3
      })
    }
  | 'MEDIUMTEXT' opt_charset_with_opt_binary
    {
      FieldType::FieldTextType(FieldTextType {
        span: $span,
        name: String::from("MEDIUMBLOB"),
        opt: $2,
        length: None
      })
    }
  | 'LONGTEXT' opt_charset_with_opt_binary
    {
      FieldType::FieldTextType(FieldTextType {
        span: $span,
        name: String::from("LONGBLOB"),
        opt: $2,
        length: None
      })
    }
  | 'ENUM' '(' string_list ')' opt_charset_with_opt_binary
    {
      FieldType::FieldEnumType(FieldEnumType {
        span: $span,
        values: $3,
        opt: $5
      })
    }
  | 'SET' '(' string_list ')' opt_charset_with_opt_binary
    {
      FieldType::FieldSetType(FieldSetType {
        span: $span,
        values: $3,
        opt: $5
      })
    }
  | 'LONG' opt_charset_with_opt_binary
    {
      FieldType::FieldTextType(FieldTextType {
        span: $span,
        name: String::from("LONGBLOB"),
        opt: $2,
        length: None
      })
    }
  | 'SERIAL'
    {
      FieldType::FieldSerialType
    }
  | 'JSON'
    {
      FieldType::FieldJsonType
    }
  ;

spatial_type -> &'input str:
    'GEOMETRY'
    {
      "GEOMETRY"
    }
  | 'GEOMETRYCOLLECTION'
    {
      "GEOMETRYCOLLECTION"
    }
  | 'POINT'
    {
      "POINT"
    }
  | 'MULTIPOINT'
    {
      "MULTIPOINT"
    }
  | 'LINESTRING'
    {
      "LINESTRING"
    }
  | 'MULTILINESTRING'
    {
      "MULTILINESTRING"
    }
  | 'POLYGON'
    {
      "POLYGON"
    }
  | 'MULTIPOLYGON'
    {
      "MULTIPOLYGON"
    }
  ;

string_list -> Vec<String>:
    text_binary_string
    {
      vec![$1]
    }
  | string_list ',' text_binary_string
    {
      $1.push($3);
      $1
    }
  ;

// ================
// insert statement
// ================
insert_stmt -> Box<InsertStmt>:
    'INSERT'                    
    insert_lock_option          
    opt_ignore                  
    opt_into                   
    table_ident                 
    opt_use_partition           
    insert_from_constructor     
    opt_values_reference        
    opt_insert_update_list      
    {
      Box::new(InsertStmt {
        span: $span,
        lock: $2,
        ignore: $3,
        into: $4,
        table_name: $5,
        partition_names: $6,
        is_set: false,
        from_construct: Some($7),
        values_ref: $8,
        ins_updates: $9,
        query_expr: None,
        updates: vec![],
      })
    }
  | 'INSERT'                   
    insert_lock_option           
    opt_ignore                   
    opt_into                     
    table_ident                  
    opt_use_partition            
    'SET'                     
    update_list                  
    opt_values_reference         
    opt_insert_update_list       
    {
      Box::new(InsertStmt {
        span: $span,
        lock: $2,
        ignore: $3,
        into: $4,
        table_name: $5,
        partition_names: $6,
        is_set: true,
        updates: $8,
        values_ref: $9,
        ins_updates: $10,
        from_construct: None,
        query_expr: None,
      })
    }
  | 'INSERT'                  
    insert_lock_option        
    opt_ignore                
    opt_into                  
    table_ident               
    opt_use_partition         
    insert_query_expression   
    opt_insert_update_list    
    {
      Box::new(InsertStmt {
        span: $span,
        lock: $2,
        ignore: $3,
        into: $4,
        table_name: $5,
        partition_names: $6,
        ins_updates: $8,
        query_expr: Some($7),
        from_construct: None,
        values_ref: None,
        is_set: false,
        updates: vec![],
      })
    }
  ;

insert_lock_option -> InsertLockOpt:
    /* empty */   { InsertLockOpt::None }
  | LOW_PRIORITY  { InsertLockOpt::LowPriority }
  | DELAYED       { InsertLockOpt::Delayed }
  | HIGH_PRIORITY { InsertLockOpt::HighPriority }
  ;


opt_ignore -> bool:
    /* empty */ { false }
  | IGNORE      { true  }
  ;

opt_into -> bool:
    /* empty */ { false }
  | INTO        { true  }
  ;
  
update_list -> Vec<UpdateElem>:
    update_list ',' update_elem
    {
      $1.push($3);
      $1
    }
  | update_elem
    {
      vec![$1]
    }
  ;

update_elem -> UpdateElem:
    simple_ident_nospvar equal expr_or_default
    {
      UpdateElem {
        span: $span,
        var_name: $1,
        equal: String::from($2),
        expr: $3,
      }
    }
  ;

opt_values_reference -> Option<ValuesRef>:
    /* empty */
    {
      None
    }
  | AS ident opt_derived_column_list
    {
      Some(ValuesRef {
        span: $span,
        alias_name: $2.0,
        derived_columns: $3,
      })
    }
  ;

opt_insert_update_list -> Option<InsUpdates>:
    /* empty */
    {
      None
    }
  | ON DUPLICATE KEY UPDATE update_list
    {
      Some(InsUpdates {
        span: $span,
        modifers: String::from("ON DUPLICATE KEY UPDATE"),
        updates: $5,
      })
    }
  ;

insert_from_constructor -> InsertFromConstruct:
    insert_values
    {
      InsertFromConstruct {
        span: $span,
        values: $1,
        is_parens: false,
        fields: vec![],
      }
    }
  | '(' ')' insert_values
    {
      InsertFromConstruct {
        span: $span,
        is_parens: true,
        values: $3,
        fields: vec![],
      }
    }
  | '(' fields ')' insert_values
    {
      InsertFromConstruct {
        span: $span,
        is_parens: true,
        fields: $2,
        values: $4,
      }
    }
  ;

insert_values -> InsertVals:
    value_or_values values_list
    {
      InsertVals {
        span: $span,
        val_ident: $1,
        values: $2,
      }
    }
  ;

value_or_values -> ValOrVals:
    VALUE     { ValOrVals::Value }
  | VALUES    { ValOrVals::Values }
  ;

values_list -> Vec<RowValue>:
    values_list ','  row_value
    {
      $1.push($3);
      $1
    }
  | row_value
    {
      vec![$1]
    }
  ;

fields -> Vec<InsertIdent>:
    fields ',' insert_ident
    {
      $1.push($3);
      $1
    }
  | insert_ident
    {
      vec![$1]
    }
  ;

insert_ident -> InsertIdent:
    simple_ident_nospvar  { InsertIdent::Ident($1)     }
  | table_wild            { InsertIdent::TableWild($1) }
  ;

insert_query_expression -> InsertQueryExpr:
    query_expression_or_parens
    {
      InsertQueryExpr {
        span: $span,
        query: $1,
        is_parens: false,
        fields: vec![],
      }
    }
  | '(' ')' query_expression_or_parens
    {
      InsertQueryExpr {
        span: $span,
        query: $3,
        is_parens: true,
        fields: vec![],
      }
    }
  | '(' fields ')' query_expression_or_parens
    {
      InsertQueryExpr {
        span: $span,
        query: $4,
        is_parens: true,
        fields: $2,
      }
    }
  ;

query_expression_or_parens -> SelectStmt:
    query_expression
    {
      $1
    }
  | query_expression locking_clause_list
    {                    
      match $1 {
        SelectStmt::Query(mut q) => {
          q.lock_clauses = $2;
          SelectStmt::Query(q)
        }
        _ => $1
      }
    }
  | query_expression_parens
    {
      $1
    }
  ;

update_stmt -> Box<UpdateStmt>:
    opt_with_clause
    'UPDATE'
    opt_low_priority
    opt_ignore
    table_reference_list
    'SET'
    update_list
    opt_where_clause
    opt_order_clause
    opt_simple_limit
    {
      Box::new(UpdateStmt {
	      span: $span,
        with_clause: $1,
	      low_priority: $3,
	      ignore: $4,
	      table_refs: $5,
	      updates: $7,
	      where_clause: $8,
	      order_clause: $9,
	      simple_limit: $10,
      })
    }
  ;

opt_with_clause -> Option<WithClause>:
  /* empty */       { None }
  | with_clause     { Some($1) }
  ;

opt_low_priority -> bool:
  /* empty */         { false }
  | 'LOW_PRIORITY'    { true  }
  ;

delete_stmt -> Box<DeleteStmt>:
    opt_with_clause
    'DELETE'
    opt_quick
    opt_low_priority
    opt_ignore
    'FROM'
    table_ident
    opt_table_alias
    opt_use_partition
    opt_where_clause
    opt_order_clause
    opt_simple_limit
    {

      Box::new(DeleteStmt {
        span: $span,
        with_clause: $1,
        quick: $3,
        low_priority: $4,
        ignore: $5,
        table_name: Some($7),
        alias_name: $8,
        partition_names: $9,
        where_clause: $10,
        order_clause: $11,
        simple_limit: $12,
        table_alias_refs: vec![],
        table_refs: vec![],
        is_using: false,
      })
    }
  | opt_with_clause
    'DELETE'
    opt_quick
    opt_low_priority
    opt_ignore
    table_alias_ref_list   
    'FROM'
    table_reference_list
    opt_where_clause
    {
      Box::new(DeleteStmt {
        span: $span,
        with_clause: $1,
        quick: $3,
        low_priority: $4,
        ignore: $5,
        table_alias_refs: $6,
        table_refs: $8,
        where_clause: $9,
        alias_name: None,
        order_clause: None,
        simple_limit: None,
        partition_names: vec![],
        table_name: None,
        is_using: false,
      })
    }
  | opt_with_clause
    'DELETE'
    opt_quick
    opt_low_priority
    opt_ignore
    'FROM'
    table_alias_ref_list
    'USING'
    table_reference_list
    opt_where_clause
    {
      Box::new(DeleteStmt {
	      span: $span,
        with_clause: $1,
        quick: $3,
        low_priority: $4,
        ignore: $5,
	      table_alias_refs: $7,
	      table_refs: $9,
	      where_clause: $10,
        order_clause: None,
        alias_name: None,
        simple_limit: None,
        partition_names: vec![],
        table_name: None,
        is_using: true,
      })
    }
  ;

opt_quick -> bool:
  /* empty */   %prec EMPTY { false }
  | 'QUICK'                 { true  }

  ;

prepare -> Box<Prepare>:
  'PREPARE' ident FROM prepare_src
  {
    Box::new(Prepare {
      span: $span,
      stmt_name: $2.0,
      preparable_stmt: $4,
    })
  }
;

prepare_src -> Value:
  'TEXT_STRING'
  {
     Value::Text {
	    span: $span,
	    value: String::from($lexer.span_str($1.as_ref().unwrap().span())),
	  }
  }
  | '@' ident_or_text
    {
       Value::SelectVarIdent {
         span: $span,
         value: $2,
         is_at: true,
       }
    }
;

execute -> Box<ExecuteStmt>:
  'EXECUTE' ident execute_using
  {
    Box::new(ExecuteStmt {
      span: $span,
      stmt_name: $2.0,
      execute_using: $3,
    })
  }
;

execute_using -> Vec<Value>:
      /* nothing */           { vec![] }
    | 'USING' execute_var_list  { $2 }
    ;

execute_var_list -> Vec<Value>:
    execute_var_ident
      {
        vec![$1]
      }
    | execute_var_list ',' execute_var_ident
      {
        $1.push($3);
        $1
      }
  ;

execute_var_ident -> Value:
    '@' ident_or_text
    {
       Value::SelectVarIdent {
         span: $span,
         value: $2,
         is_at: true,
       }
    }
    ;

deallocate -> Box<Deallocate>:
    deallocate_or_drop 'PREPARE' ident
    {
       Box::new(Deallocate {
         span: $span,
         deallocate_or_drop: String::from($1),
         stmt_name: $3.0,
       })
    }
    ;

deallocate_or_drop -> &'input str:
    'DEALLOCATE'  { "DEALLOCATE" }
    | 'DROP'  { "DROP" }
    ;

begin_stmt -> Box<BeginStmt>:
    'BEGIN' opt_work
    {
       Box::new(BeginStmt {
             span: $span,
             work: $2
       })
    }
    ;

opt_work -> bool:
    /* empty */ { false }
    | 'WORK'    { true }
    ;

set -> Box<SetOptValues>:
  'SET' start_option_value_list
  {
    Box::new($2)
  }
  ;


// Start of option value list
start_option_value_list -> SetOptValues:
    option_value_no_option_type option_value_list_continued
    {
      SetOptValues::OptValues(
        OptValues {
          opt: $1,
          values: $2,
        }
      )
    }
  | TRANSACTION transaction_characteristics
    {
      SetOptValues::Transaction($2)
    }
  | option_type start_option_value_list_following_option_type
    {
      SetOptValues::OptTypeFollowing(
        OptTypeFollowing {
          opt_type: $1,
          following: $2,
        }
      )
    }
  ;

option_value_no_option_type -> SetOpts:
    lvalue_variable equal set_expr_or_default
    {
      SetOpts::SetVariable(SetVariable {
        var: $1,
        value: $3,
      })
    }
  | '@' ident_or_text equal expr
    {
      SetOpts::SetUserVar(SetUserVar {
        var: $2,
        expr: $4,
      })
    }
  | '@' '@' opt_set_var_ident_type lvalue_variable equal set_expr_or_default
    {
      SetOpts::SetSystemVar(SetSystemVar {
        opt_var: $3,
        var: $4,
        value: $6, 
      })
    }
  | character_set old_or_new_charset_name_or_default
    {
      SetOpts::SetCharset(SetCharset {
        charset: $1,
        new_value: $2,
      })
    }
  | NAMES charset_name opt_collate
    {
      SetOpts::SetNames(SetNames {
        charset_name: Some($2),
        collate: $3,
      })
    }
  | NAMES DEFAULT
    {
      SetOpts::SetNames(SetNames {
        charset_name: None,
        collate: None,
      })
    }
;

start_option_value_list_following_option_type -> FollowingOptType:
    option_value_following_option_type option_value_list_continued
    {
      FollowingOptType::TypeEq(
        FollowingOptTypeEq {
          opt_type: $1,
          values: $2,
        }
      )
    }
  | TRANSACTION transaction_characteristics
    {
      FollowingOptType::Transaction($2)
    }
  ;

lvalue_variable -> String:
    ident
    {
      $1.0
    }
  | ident '.' ident
    {
      let mut s = String::with_capacity($1.0.len()+1+$3.0.len());
      s.push_str(&$1.0);
      s.push('.');
      s.push_str(&$3.0);
      s
    }
  | DEFAULT '.' ident
    {
      let mut s = String::with_capacity(7+1+$3.0.len());
      s.push_str("DEFAULT");
      s.push('.');
      s.push_str(&$3.0);
      s
    }
  ;

transaction_characteristics -> Transaction:
    transaction_access_mode opt_isolation_level
    {
      Transaction {
        mode: Some($1),
        isolation_level: $2,
      }
    }
  | isolation_level opt_transaction_access_mode
    {
      Transaction {
        isolation_level: Some($1),
        mode: $2,
      }
    }
  ;

isolation_level -> IsolationType:
    ISOLATION LEVEL isolation_types
    {
      $3
    }
  ;

opt_isolation_level -> Option<IsolationType>:
    /* empty */         { None }
  | ',' isolation_level { Some($2) }
  ;

opt_transaction_access_mode -> Option<TransactionType>:
    /* empty */                 { None }
  | ',' transaction_access_mode { Some($2) }
  ;

transaction_access_mode -> TransactionType:
    transaction_access_mode_types
    {
      $1
    }
  ;

transaction_access_mode_types -> TransactionType:
    'READ' 'ONLY'   { TransactionType::ReadOnly }
  | 'READ' 'WRITE'  { TransactionType::ReadWrite }
  ;

isolation_types -> IsolationType:
    'READ' 'UNCOMMITTED' { IsolationType::ReadUncommitted }
  | 'READ' 'COMMITTED'   { IsolationType::ReadCommitted }
  | 'REPEATABLE' 'READ'  { IsolationType::RepeatableRead }
  | 'SERIALIZABLE'       { IsolationType::Serializable }
  ;

set_expr_or_default -> ExprOrDefault:
    expr
    {
      ExprOrDefault::Expr($1)
    }
  | DEFAULT
    {
      ExprOrDefault::Default  
    }
  | ON
    {
      ExprOrDefault::On
    }
  | ALL
    {
      ExprOrDefault::All
    }
  | BINARY
    {
      ExprOrDefault::Binary
    }
  | ROW
    {
      ExprOrDefault::Row
    }
  | SYSTEM
    {
      ExprOrDefault::System
    }
  ;

opt_set_var_ident_type -> SetVarIdentType:
    /* empty */         { SetVarIdentType::None }
  | 'PERSIST_ONLY' '.'  { SetVarIdentType::PersistOnly }
  | 'PERSIST' '.'       { SetVarIdentType::Persist }
  | GLOBAL '.'          { SetVarIdentType::Global }
  | LOCAL '.'           { SetVarIdentType::Local }
  | SESSION '.'         { SetVarIdentType::Session }
  ;

old_or_new_charset_name_or_default -> NewCharset:
    old_or_new_charset_name { $1 }
  | DEFAULT                 { NewCharset::Default }
  ;

old_or_new_charset_name -> NewCharset:
    ident_or_text
    {
      NewCharset::CharsetName($1)
    }
  | BINARY 
    { 
      NewCharset::Binary
    }
  ;

option_value_list_continued -> Vec<OptValue>:
    /* empty */           { vec![] }
  | ',' option_value_list { $2     }
  ;

option_value_list -> Vec<OptValue>:
    option_value
    {
      vec![$1]
    }
  | option_value_list ',' option_value
    {
      $1.push($3);
      $1
    }
  ;

option_value -> OptValue:
    option_type option_value_following_option_type
    {
      OptValue::OptValueType(OptValueType {
        opt_type: $1,
        set_var: $2,
      })
    }
  | option_value_no_option_type 
    { 
      OptValue::SetOpts($1)
    }
  ;

option_value_following_option_type -> SetVariable:
    lvalue_variable equal set_expr_or_default
    {
      SetVariable {
        var: $1,
        value: $3,
      }
    }
  ;


option_type -> OptType:
    GLOBAL          { OptType::Global }
  | PERSIST         { OptType::Persist }
  | PERSIST_ONLY    { OptType::PersistOnly }
  | LOCAL           { OptType::Local }
  | SESSION         { OptType::Session }
;

show_databases_stmt -> Box<ShowDatabasesStmt>:
    'SHOW' 'DATABASES' opt_wild_or_where
    {
    	Box::new(ShowDatabasesStmt {
    	   span: $span,
    	   opt_wild_or_where: $3,
    	})
    }
;

opt_wild_or_where -> Option<WildOrWhere>:
   /* empty */                { None }
  | 'LIKE' 'TEXT_STRING'      { Some(WildOrWhere::LikeTextString(String::from($lexer.span_str($2.as_ref().unwrap().span())))) }
  | where_clause              { Some(WildOrWhere::WhereClause($1)) }
;

show_tables_stmt -> Box<ShowTablesStmt>:
    'SHOW' opt_show_cmd_type 'TABLES' opt_db opt_wild_or_where
    {
    	Box::new(ShowTablesStmt {
    	   span: $span,
    	   opt_show_cmd_type: $2,
    	   opt_db: $4,
    	   opt_wild_or_where: $5,
    	})
    }
;

show_columns_stmt -> Box<ShowColumnsStmt>:
    'SHOW' opt_show_cmd_type columns_cmd_type from_table opt_db opt_wild_or_where
    {
    	Box::new(ShowColumnsStmt {
    	   span: $span,
    	   opt_show_cmd_type: $2,
    	   columns_cmd_type: $3,
    	   from_table: $4,
    	   opt_db: $5,
    	   opt_wild_or_where: $6,
    	})
    }
;

opt_show_cmd_type -> Option<ShowCmdType>:
       /* empty */          { None }
     | FULL                 { Some(ShowCmdType::Full) }
     | EXTENDED             { Some(ShowCmdType::Extended) }
     | EXTENDED FULL        { Some(ShowCmdType::ExtendedFull) }
;

columns_cmd_type -> ShowColumnsCmdType:
       COLUMNS              { ShowColumnsCmdType::Columns }
     | FIELDS               { ShowColumnsCmdType::Fields }
;

opt_db -> Option<ShowTableDb>:
       /* empty */  { None }
      | from_or_in ident {
           Some(ShowTableDb {
		span: $span,
		from_or_in: $1,
		db: $2.0,
           })
        }
;

from_table -> ShowFromTable:
       from_or_in ident {
           ShowFromTable {
		span: $span,
		from_or_in: $1,
		table: $2.0,
           }
        }
;

from_or_in -> FromOrIn:
      'FROM' { FromOrIn::From }
    | 'IN'   { FromOrIn::In }
;

show_create_table_stmt -> Box<ShowCreateTableStmt>:
    'SHOW' 'CREATE' 'TABLE' table_ident
    {
        Box::new(ShowCreateTableStmt {
           span: $span,
           table: $4,
        })
    }
;

show_keys_stmt -> Box<ShowKeysStmt>:
    'SHOW' opt_extended keys_or_index from_table opt_db opt_where_clause
    {
    	Box::new(ShowKeysStmt {
    	   span: $span,
    	   is_extended: $2,
    	   keys_or_index: $3,
    	   from_table: $4,
    	   opt_db: $5,
    	   opt_where_clause: $6,
    	})
    }
;

keys_or_index -> KeysOrIndex:
       'INDEX'              { KeysOrIndex::Index }
     | 'INDEXES'            { KeysOrIndex::Indexes }
     | 'KEYS'               { KeysOrIndex::Keys }
;

opt_extended -> bool:
       /* empty */          { false }
     | 'EXTENDED'           { true }
;

show_variables_stmt -> Box<ShowVariablesStmt>:
    'SHOW' opt_var_type 'VARIABLES' opt_wild_or_where
    {
        Box::new(ShowVariablesStmt {
           span: $span,
           opt_var_type: $2,
           opt_wild_or_where: $4,
        })
    }
;

opt_var_type -> Option<ShowVariableType>:
       /* empty */          { None }
     | GLOBAL               { Some(ShowVariableType::Global) }
     | LOCAL                { Some(ShowVariableType::Session) }
     | SESSION              { Some(ShowVariableType::Session) }
;

show_create_view_stmt -> Box<ShowCreateViewStmt>:
    'SHOW' 'CREATE' 'VIEW' table_ident
    {
        Box::new(ShowCreateViewStmt {
           span: $span,
           view_name: $4,
        })
    }
;

show_master_status_stmt -> Box<ShowDetailsStmt>:
    'SHOW' 'MASTER' 'STATUS'
    {
        Box::new(ShowDetailsStmt {
           span: $span,
        })
    }
;

show_engines_stmt -> Box<ShowEnginesStmt>:
    'SHOW' opt_storage 'ENGINES'
    {
        Box::new(ShowEnginesStmt {
           span: $span,
           is_storage: $2,
        })
    }
;

opt_storage -> bool:
       /* empty */          { false }
     | 'STORAGE'           { true }
;

show_plugins_stmt -> Box<ShowDetailsStmt>:
    'SHOW' 'PLUGINS'
    {
        Box::new(ShowDetailsStmt {
           span: $span,
        })
    }
;

show_privileges_stmt -> Box<ShowDetailsStmt>:
    'SHOW' 'PRIVILEGES'
    {
        Box::new(ShowDetailsStmt {
           span: $span,
        })
    }
;

show_create_procedure_stmt -> Box<ShowCreateSpStmt>:
    'SHOW' 'CREATE' 'PROCEDURE' sp_name
    {
        Box::new(ShowCreateSpStmt {
           span: $span,
           sp_name: $4,
        })
    }
;

show_create_function_stmt -> Box<ShowCreateSpStmt>:
    'SHOW' 'CREATE' 'FUNCTION' sp_name
    {
        Box::new(ShowCreateSpStmt {
           span: $span,
           sp_name: $4,
        })
    }
;

show_create_trigger_stmt -> Box<ShowCreateSpStmt>:
    'SHOW' 'CREATE' 'TRIGGER' sp_name
    {
        Box::new(ShowCreateSpStmt {
           span: $span,
           sp_name: $4,
        })
    }
;

show_create_event_stmt -> Box<ShowCreateSpStmt>:
    'SHOW' 'CREATE' 'EVENT' sp_name
    {
        Box::new(ShowCreateSpStmt {
           span: $span,
           sp_name: $4,
        })
    }
;

show_create_user_stmt -> Box<ShowCreateUserStmt>:
    'SHOW' 'CREATE' 'USER' user
    {
        Box::new(ShowCreateUserStmt {
           span: $span,
           user: $4,
        })
    }
;

show_status_stmt -> Box<ShowStatusStmt>:
    'SHOW' opt_var_type 'STATUS' opt_wild_or_where
    {
        Box::new(ShowStatusStmt {
           span: $span,
           opt_var_type: $2,
           opt_wild_or_where: $4,
        })
    }
;

show_processlist_stmt -> Box<ShowProcessListStmt>:
    'SHOW' opt_full 'PROCESSLIST'
    {
        Box::new(ShowProcessListStmt {
           span: $span,
           is_full: $2,
        })
    }
;

opt_full -> bool:
       /* empty */          { false }
     | 'FULL'               { true }
;

show_replicas_stmt -> Box<ShowDetailsStmt>:
    'SHOW' 'SLAVE' 'HOSTS'
    {
        Box::new(ShowDetailsStmt {
           span: $span,
        })
    }
   |
    'SHOW' 'REPLICAS'
    {
        Box::new(ShowDetailsStmt {
           span: $span,
        })
    }
;

show_replica_status_stmt -> Box<ShowReplicaStatusStmt>:
    'SHOW' replica 'STATUS' opt_channel
    {
        Box::new(ShowReplicaStatusStmt {
           span: $span,
           replica: $2,
           opt_channel: $4,
        })
    }
;

replica -> Replica:
       SLAVE       { Replica::Slave }
     | REPLICA     { Replica::Replica }
;

opt_channel -> Option<Channel>:
     /* empty */ { None }
    | 'FOR' 'CHANNEL' TEXT_STRING_sys_nonewline
    {
        Some(Channel {
            span: $span,
            channel: $3,
        })
    }
;

TEXT_STRING_sys_nonewline -> String:
    'TEXT_STRING'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
;

TEXT_STRING_sys -> String:
    'TEXT_STRING'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
;

TEXT_STRING_password -> String:
    'TEXT_STRING'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
;

TEXT_STRING_hash -> String:
    'TEXT_STRING'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
;

TEXT_STRING_literal -> String:
    'TEXT_STRING'
    {
      String::from($lexer.span_str($1.as_ref().unwrap().span()))
    }
;

show_grants_stmt -> Box<ShowGrantsStmt>:
    'SHOW' 'GRANTS'
    {
        Box::new(ShowGrantsStmt {
           span: $span,
           user: None,
           user_list: None,
        })
    }
   |
    'SHOW' 'GRANTS' 'FOR' user
    {
        Box::new(ShowGrantsStmt {
           span: $span,
           user: Some($4),
           user_list: None,
        })
    }
   |
    'SHOW' 'GRANTS' 'FOR' user 'USING' user_list
    {
        Box::new(ShowGrantsStmt {
           span: $span,
           user: Some($4),
           user_list: Some($6),
        })
    }
;

start -> Start:
  START TRANSACTION opt_start_transaction_option_list
  {
    Start {
      span: $span,
      transaction_opts: $3, 
    }
  }
  ;

opt_start_transaction_option_list -> u8:
    /* empty */
    {
      0
    }
  | start_transaction_option_list
    {
      $1
    }
;

start_transaction_option_list -> u8:
  start_transaction_option
    {
      $1
    }
  | start_transaction_option_list ',' start_transaction_option
    {
      $1 | $3
    }
;

start_transaction_option -> u8:
    WITH CONSISTENT SNAPSHOT
    {
      // $$= MYSQL_START_TRANS_OPT_WITH_CONS_SNAPSHOT;
      // see https://github.com/mysql/mysql-server/blob/8.0/sql/handler.h#L631
      1
    }
  | READ ONLY
    {
      // $$= MSQL_START_TRANS_OPT_READ_ONLY;
      2
    }
  | READ WRITE
    {
      //$$= MYSQL_START_TRANS_OPT_READ_WRITE;
      4
    }
;

commit -> Commit:
  COMMIT opt_work opt_chain opt_release
    {
      Commit {
        span: $span,
        opt_work: $2,
        opt_chain: $3,
        opt_release: $4,
      }
    }
  ;

rollback_stmt -> Rollback:
    ROLLBACK opt_work opt_chain opt_release
    {
      Rollback {
        span: $span,
        opt_work: $2,
        opt_chain: $3,
        opt_release: $4,
        opt_savepoint: false,
        ident: None,
      }
    }
  | ROLLBACK opt_work 'TO' opt_savepoint ident  
    {
      Rollback {
        span: $span,
        opt_work: $2,
        opt_chain: YesNoUnknown::TvlUnknown,
        opt_release: YesNoUnknown::TvlUnknown,
        opt_savepoint: $4,
        ident: Some($5.0),
      }
    }
;

opt_chain -> YesNoUnknown:
    /* empty */
    { 
      YesNoUnknown::TvlUnknown 
    }
  | AND NO CHAIN  
    {  
      YesNoUnknown::TvlNo 
    }
  | AND CHAIN
    { 
      YesNoUnknown::TvlYes 
    }
;

opt_release -> YesNoUnknown:
    /* empty */
    { 
      YesNoUnknown::TvlUnknown 
    }
  | 'RELEASE'     
    { 
      YesNoUnknown::TvlYes 
    }
  | NO 'RELEASE' 
    { 
      YesNoUnknown::TvlNo
    }
;

opt_savepoint ->  bool:
    /* empty */  %prec LOWER_THEN_SAVEPOINT { false }
  | 'SAVEPOINT'     { true  }
;

create -> Create:
       'CREATE' 'DATABASE' opt_if_not_exists ident opt_create_database_options
       {
		    Create::CreateDatabase(Box::new(
			    CreateDatabase{
				    is_not_exists: $3,
				    database_name: $4.0,
				    opt_create_database_options: $5,
			    }
		    ))
       }
    | 'CREATE' view_or_trigger_or_sp_or_event
       {
		    Create::CreateViewOrTriggerOrSpOrEvent(Box::new($2))
       }

    | 'CREATE' 'USER' opt_if_not_exists create_user_list default_role_clause
                      require_clause connect_options
                      opt_account_lock_password_expire_options
                      opt_user_attribute
       {
            Create::CreateUser(Box::new(
			    CreateUser {
			        span: $span,
				    is_not_exists: $3,
				    create_user_list: $4,
				    default_role_clause: $5,
				    require_clause: $6,
				    connect_options: $7,
				    opt_account_lock_password_expire_options: $8,
				    opt_user_attribute: $9,
			    }
		    ))
       }
    | 'CREATE' 'LOGFILE' 'GROUP' ident 'ADD' lg_undofile opt_logfile_group_options
       {
            Create::CreateLogFileGroup(Box::new(
                CreateLogFileGroup {
                    span: $span,
                    logfile_group: $4.0,
                    undo_file: $6,
                    opt_logfile_group_options: $7,
                }
            ))
       }
    | 'CREATE' 'TABLESPACE' ident opt_ts_datafile_name opt_logfile_group_name opt_tablespace_options
       {
           Create::CreateTablespace(Box::new(
               CreateTablespace {
                   span: $span,
                   tablespace_name: $3.0,
                   opt_ts_datafile: $4,
                   opt_logfile_group: $5,
                   opt_tablespace_options: $6,
               }
           ))
       }
    | 'CREATE' 'UNDO' 'TABLESPACE' ident 'ADD' ts_datafile opt_undo_tablespace_options
      {
           Create::CreateUndoTablespace(Box::new(
               CreateUndoTablespace {
                   span: $span,
                   tablespace_name: $4.0,
                   ts_datafile: $6,
                   opt_undo_tablespace_options: $7,
               }
           ))
      }
    | 'CREATE' 'SERVER' ident_or_text 'FOREIGN' 'DATA' 'WRAPPER'
                ident_or_text 'OPTIONS' '(' server_options_list ')'
      {
           Create::CreateServer(Box::new(
               CreateServer {
                   span: $span,
                   server_name: $3,
                   wrapper_name: $7,
                   server_options_list: $10,
               }
           ))
      }
;

opt_if_not_exists -> bool:
          /* empty */   { false }
        | 'IF' not 'EXISTS' { true }
        ;

opt_create_database_options -> Vec<CreateDatabaseOption>:
          /* empty */ { vec![] }
        | create_database_options { $1 }
        ;

create_database_options -> Vec<CreateDatabaseOption>:
          create_database_option
          {
          	vec![$1]
          }
        | create_database_options create_database_option
        {
        	$1.push($2);
        	$1
        }
        ;

create_database_option -> CreateDatabaseOption:
          default_collation
          {
            $1
          }
        | default_charset
          {
            $1
          }
        | default_encryption
          {
            $1
          }
        ;

default_collation -> CreateDatabaseOption:
          opt_default 'COLLATE' opt_equal collation_name {
          	let is_default = match $1 {
          		Some(_) => true,
          		None => false,
          	};
          	let is_equal = match $3 {
          		Some(_) => true,
          		None => false,
          	};
		CreateDatabaseOption::DefaultCollation(DefaultCollation{
		    is_default: is_default,
		    is_equal: is_equal,
		    collation_name: $4,
		})
          }
        ;

default_charset -> CreateDatabaseOption:
          opt_default character_set opt_equal charset_name
          {
		let is_default = match $1 {
			Some(_) => true,
			None => false,
		};
		let is_equal = match $3 {
			Some(_) => true,
			None => false,
		};
		CreateDatabaseOption::DefaultCharset(DefaultCharset{
		    is_default: is_default,
		    is_equal: is_equal,
		    charset_name: $4,
		})
          }
        ;

default_encryption -> CreateDatabaseOption:
          opt_default 'ENCRYPTION' opt_equal 'TEXT_STRING'
          {
		let is_default = match $1 {
			Some(_) => true,
			None => false,
		};
		let is_equal = match $3 {
			Some(_) => true,
			None => false,
		};
		CreateDatabaseOption::DefaultEncryption(DefaultEncryption{
		    is_default: is_default,
		    is_equal: is_equal,
		    encryption: String::from($lexer.span_str($4.as_ref().unwrap().span())),
		})
          }
        ;

view_or_trigger_or_sp_or_event -> ViewOrTriggerOrSpOrEvent:
          definer init_lex_create_info definer_tail
          {
          	ViewOrTriggerOrSpOrEvent{
          		definer: $1,
          		definer_tail: $3,
          	}
          }
        /* TODO */
        ;

definer -> Definer:
          'DEFINER' 'EQ' user
          {
		Definer{
			user: $3,
		}
          }
        ;

user -> User:
          user_ident_or_text
          {
            	User::UserIdentOrText($1)
          }
        | 'CURRENT_USER' optional_braces
          {
            	User::CurrentUser
          }
        ;

user_ident_or_text -> String:
          ident_or_text
          {
		$1
          }
        | ident_or_text '@' ident_or_text
          {
	    	$1.push('@');
	    	$1.push_str(&$3);
	    	$1
          }
        ;

user_list -> Vec<User>:
    user
    {
       vec![$1]
    }
   | user_list ',' user
    {
       $1.push($3);
       $1
    }
;

init_lex_create_info -> Option<String>:
          /* empty */
          {
          	None
          }
        ;

definer_tail -> DefinerTail:
          view_tail { DefinerTail::ViewTail($1) }
        | trigger_tail { DefinerTail::TriggerTail($1) }
        | sp_tail { DefinerTail::SpTail($1) }
        /* TODO */
        ;

view_tail -> ViewTail:
          view_suid 'VIEW' table_ident opt_derived_column_list 'AS' view_query_block
          {
            	ViewTail{
            		span: $span,
            		view_suid: $1,
            		view_name: $3,
            		columns: $4,
            		view_query_block: $6,
            	}
          }
        ;

view_suid -> ViewSuid:
          /* empty */
          { ViewSuid::ViewSuidDefault }
        | 'SQL' 'SECURITY' 'DEFINER'
          { ViewSuid::ViewSuidDefiner }
        | 'SQL' 'SECURITY' 'INVOKER'
          { ViewSuid::ViewSuidInvoker }
        ;

view_query_block -> ViewQueryBlock:
          query_expression_or_parens view_check_option
          {
		ViewQueryBlock{
			select_stmt:$1,
			view_check_option:$2,
		}
          }
          ;

view_check_option -> ViewCheckOption:
          /* empty */                     { ViewCheckOption::ViewCheckNone }
        | 'WITH' 'CHECK' 'OPTION'           { ViewCheckOption::ViewCheckCascade }
        | 'WITH' 'CASCADED' 'CHECK' 'OPTION'  { ViewCheckOption::ViewCheckCascade }
        | 'WITH' 'LOCAL' 'CHECK' 'OPTION' { ViewCheckOption::ViewCheckLocal }
        ;

trigger_tail -> TriggerTail:
          'TRIGGER'
          sp_name
          trg_action_time
          trg_event
          'ON'
          table_ident
          'FOR'
          'EACH'
          'ROW'
          trigger_follows_precedes_clause
          sp_proc_stmt
          {
          	TriggerTail{
          		span: $span,
			sp_name: $2,
			trg_action_time: $3,
			trg_event: $4,
			table_name: $6,
			trigger_follows_precedes_clause: $10,
			sp_proc_stmt: $11,
          	}
          }
        ;

sp_name -> String:
          ident '.' ident
          {
	      	let mut s = String::with_capacity($1.0.len()+1+$3.0.len());
	      	s.push_str(&$1.0);
	      	s.push('.');
	      	s.push_str(&$3.0);
	      	s
          }
        | ident
          {
          	$1.0
          }
        ;

trg_action_time -> TrgActionTime:
            'BEFORE'
            { TrgActionTime::Before }
          | 'AFTER'
            { TrgActionTime::After }
          ;

trg_event -> TrgEvent:
            'INSERT'
            { TrgEvent::Insert }
          | 'UPDATE'
            { TrgEvent::Update }
          | 'DELETE'
            { TrgEvent::Delete }
          ;

trigger_follows_precedes_clause -> TriggerFollowsPrecedesClause:
            /* empty */
            {
            	TriggerFollowsPrecedesClause{
            		ordering_clause: TrgActionOrder::None,
            		anchor_trigger_name: None,
            	}
            }
          |
            trigger_action_order ident_or_text
            {
              	TriggerFollowsPrecedesClause{
                          		ordering_clause: $1,
                          		anchor_trigger_name: Some($2),
                          	}
            }
          ;

trigger_action_order -> TrgActionOrder:
            'FOLLOWS'
            { TrgActionOrder::Follows }
          | 'PRECEDES'
            { TrgActionOrder::Precedes }
          ;

sp_proc_stmt -> Option<String>:
         /* TODO */
         { None }
        ;

sp_tail -> SpTail:
          'PROCEDURE'
          sp_name
          '('
          sp_pdparam_list
          ')'
          sp_c_chistics
          sp_proc_stmt
          {
          	SpTail{
          		span: $span,
          		sp_name: $2,
          		sp_pdparam_list: $4,
          		sp_c_chistics: $6,
          		sp_proc_stmt: $7,
          	}
          }
        ;

sp_pdparam_list -> Vec<SpPdparam>:
          /* Empty */ { vec![] }
        | sp_pdparams { $1 }
        ;

sp_pdparams -> Vec<SpPdparam>:
          sp_pdparams ',' sp_pdparam
          {
          	$1.push($3);
          	$1
          }
        | sp_pdparam
        {
        	vec![$1]
        }
        ;

sp_pdparam -> SpPdparam:
          sp_opt_inout ident type opt_collate
          {
            	SpPdparam{
            		sp_opt_inout: $1,
            		ident: $2.0,
            		sp_type: $3,
            		opt_collate: $4,
            	}
          }
        ;

sp_opt_inout -> SpOptInout:
          /* Empty */ { SpOptInout::In }
        | 'IN'      { SpOptInout::In }
        | 'OUT'     { SpOptInout::Out }
        | 'INOUT'   { SpOptInout::Inout }
        ;

sp_c_chistics -> Vec<SpCChistic>:
          /* Empty */ { vec![] }
        | sp_c_chistics sp_c_chistic
        {
        	$1.push($2);
        	$1
        }
        ;

sp_c_chistic -> SpCChistic:
          sp_chistic            { SpCChistic::SpChistic($1) }
        | 'DETERMINISTIC'     { SpCChistic::Deterministic }
        | not 'DETERMINISTIC' { SpCChistic::NotDeterministic }
        ;

sp_chistic -> SpChistic:
          'COMMENT' 'TEXT_STRING'
          {
	      SpChistic::Comment(String::from($lexer.span_str($1.as_ref().unwrap().span())))
          }
        | 'LANGUAGE' 'SQL'
          { SpChistic::LanguageSql }
        | 'NO' 'SQL'
          { SpChistic::NoSql }
        | 'CONTAINS' 'SQL'
          { SpChistic::ContainsSql }
        | 'READS' 'SQL' 'DATA'
          { SpChistic::ReadsSqlData }
        | 'MODIFIES' 'SQL' 'DATA'
          { SpChistic::ModifiesSqlData }
        | sp_suid
          { SpChistic::SpSuid($1) }
        ;

sp_suid -> SpSuid:
          'SQL' 'SECURITY' 'DEFINER'
          {
            SpSuid::SpIsSuid
          }
        | 'SQL' 'SECURITY' 'INVOKER'
          {
            SpSuid::SpIsNotSuid
          }
        ;

opt_comma -> bool:
      /* empty */  { false }
    | ','          { true }
;

create_index_stmt -> CreateIndexStmt:
      'CREATE' opt_unique 'INDEX' ident opt_index_type_clause 'ON' table_ident '(' key_list_with_expression ')'
      opt_index_options opt_index_lock_and_algorithm
      {
           CreateIndexStmt::CommonIndex(CreateCommonIndexStmt{
               span: $span,
               opt_unique: $2,
               index_name: $4.0,
               opt_index_type_clause: $5,
               table_name: $7,
               key_list_with_expression: $9,
               opt_index_options: Some($11),
               opt_index_lock_and_algorithm: $12,
           })
      }
    |
      'CREATE' 'FULLTEXT' 'INDEX' ident 'ON' table_ident '(' key_list_with_expression ')'
      opt_fulltext_index_options opt_index_lock_and_algorithm
      {
           CreateIndexStmt::FullTextIndex(CreateFullTextIndexStmt{
               span: $span,
               index_name: $4.0,
               table_name: $6,
               key_list_with_expression: $8,
               opt_fulltext_index_options: Some($10),
               opt_index_lock_and_algorithm: $11,
           })
      }
    |
      'CREATE' 'SPATIAL' 'INDEX' ident 'ON' table_ident '(' key_list_with_expression ')'
      opt_spatial_index_options opt_index_lock_and_algorithm
      {
          CreateIndexStmt::SpatialIndex(CreateSpatialIndexStmt{
               span: $span,
               index_name: $4.0,
               table_name: $6,
               key_list_with_expression: $8,
               opt_spatial_index_options: Some($10),
               opt_index_lock_and_algorithm: $11,
          })
      }
;

opt_unique -> bool:
       /* empty */          { false }
     | 'UNIQUE'             { true }
;

opt_index_options -> Vec<IndexOption>:
      /* empty */         { vec![] }
    | index_options       { $1 }
;

index_options -> Vec<IndexOption>:
      index_option
      {
            vec![$1]
      }
    | index_options index_option
      {
            $1.push($2);
            $1
      }
;

index_option -> IndexOption:
      common_index_option
      {
           IndexOption::CommonIndexOption($1)
      }
    | index_type_clause
      {
           IndexOption::IndexTypeClause($1)
      }
;

opt_fulltext_index_options -> Vec<FullTextIndexOption>:
      /* empty */                  { vec![] }
    | fulltext_index_options       { $1 }
;

fulltext_index_options -> Vec<FullTextIndexOption>:
      fulltext_index_option
      {
           vec![$1]
      }
    | fulltext_index_options fulltext_index_option
      {
           $1.push($2);
           $1
      }
;

fulltext_index_option -> FullTextIndexOption:
      common_index_option
      {
          FullTextIndexOption::CommonIndexOption($1)
      }
    | 'WITH' 'PARSER' IDENT_sys
      {
          FullTextIndexOption::WithParserIdent($3.0)
      }
;

opt_index_type_clause -> Option<IndexTypeClause>:
      /* empty */          { None }
    | index_type_clause    { Some($1) }
;

index_type_clause -> IndexTypeClause:
      'USING' index_type
      {
          IndexTypeClause {
              span: $span,
              index_type: $2,
          }
      }
    | 'TYPE' index_type
      {
          IndexTypeClause {
              span: $span,
              index_type: $2,
          }
      }
;

index_type -> IndexType:
      'BTREE'       { IndexType::Btree }
    | 'RTREE'       { IndexType::Rtree }
    | 'HASH'        { IndexType::Hash }
;

key_list_with_expression -> Vec<KeyPartWithExpression>:
      key_list_with_expression ',' key_part_with_expression
      {
          $1.push($3);
          $1
      }
    | key_part_with_expression
      {
          vec![$1]
      }
;

key_part_with_expression -> KeyPartWithExpression:
      key_part
      {
           KeyPartWithExpression::KeyPart($1)
      }
    | '(' expr ')' opt_ordering_direction
      {
           KeyPartWithExpression::OrderingDirection(OrderExpr {
               span: $span,
               expr: $2,
               direction: $4
           })
      }
;

key_part -> KeyPart:
      ident opt_ordering_direction
      {
           KeyPart {
               span: $span,
               ident: $1.0,
               length: None,
               direction: $2
           }
      }
    | ident '(' NUM ')' opt_ordering_direction
      {
           let length = String::from($lexer.span_str($3.as_ref().unwrap().span()));
           let length = length.parse::<u32>().unwrap();
           KeyPart {
               span: $span,
               ident: $1.0,
               length: Some(length),
               direction: $5
           }
      }
;

opt_spatial_index_options -> Vec<SpatialIndexOption>:
      /* empty */                  { vec![] }
    | spatial_index_options        { $1 }
;

spatial_index_options -> Vec<SpatialIndexOption>:
      spatial_index_option
      {
           vec![$1]
      }
    | spatial_index_options spatial_index_option
      {
           $1.push($2);
           $1
      }
;

spatial_index_option -> SpatialIndexOption:
    common_index_option
    {
        SpatialIndexOption::CommonIndexOption($1)
    }
;

common_index_option -> CommonIndexOption:
      'KEY_BLOCK_SIZE' opt_equal ulong_num
      {
          let is_equal = match $2 {
              Some(_) => true,
              None => false,
          };
          CommonIndexOption::KeyBlockSizeOption(KeyBlockSizeOption{
              span: $span,
              is_equal: is_equal,
              ulong_num: $3,
          })
      }
    | 'COMMENT' TEXT_STRING_sys
      {
          CommonIndexOption::CommentOption(IndexCommentOption{
              span: $span,
              comment: $2,
          })
      }
    | visibility
      {
          CommonIndexOption::Visibility($1)
      }
    | 'ENGINE_ATTRIBUTE' opt_equal json_attribute
      {
          let is_equal = match $2 {
              Some(_) => true,
              None => false,
          };
          CommonIndexOption::AttributeOption(AttributeOption{
              span: $span,
              is_equal: is_equal,
              attribute: $3,
          })
      }
    | 'SECONDARY_ENGINE_ATTRIBUTE' opt_equal json_attribute
      {
          let is_equal = match $2 {
              Some(_) => true,
              None => false,
          };
          CommonIndexOption::AttributeOption(AttributeOption{
              span: $span,
              is_equal: is_equal,
              attribute: $3,
          })
      }
;

visibility -> Visibility:
      'VISIBLE'       { Visibility::Visible }
    | 'INVISIBLE'     { Visibility::Invisible }
;

json_attribute -> String:
    TEXT_STRING_sys
    {
        $1
    }
;

opt_index_lock_and_algorithm -> Option<IndexLockAndAlgorithm>:
      /* empty */     { None }
    | alter_lock_option
      {
          Some(IndexLockAndAlgorithm{
              alter_lock_option: Some($1),
              alter_algorithm_option: None,
          })
      }
    | alter_algorithm_option
      {
          Some(IndexLockAndAlgorithm{
               alter_lock_option: None,
               alter_algorithm_option: Some($1),
          })
      }
    | alter_lock_option alter_algorithm_option
      {
          Some(IndexLockAndAlgorithm{
               alter_lock_option: Some($1),
               alter_algorithm_option: Some($2),
          })
      }
    | alter_algorithm_option alter_lock_option
      {
          Some(IndexLockAndAlgorithm{
               alter_lock_option: Some($2),
               alter_algorithm_option: Some($1),
          })
      }
;

alter_algorithm_option -> AlterAlgorithmOption:
    'ALGORITHM' opt_equal alter_algorithm_option_value
    {
        let is_equal = match $2 {
            Some(_) => true,
            None => false,
        };
        AlterAlgorithmOption {
            span: $span,
            is_equal: is_equal,
            alter_algorithm_option_value: $3,
        }
    }
;

alter_algorithm_option_value -> String:
      'DEFAULT'
      {
          String::from("DEFAULT")
      }
    | ident
      {
          $1.0
      }
;

alter_lock_option -> AlterLockOption:
    'LOCK' opt_equal alter_lock_option_value
    {
        let is_equal = match $2 {
            Some(_) => true,
            None => false,
        };
        AlterLockOption {
            span: $span,
            is_equal: is_equal,
            alter_lock_option_value: $3,
        }
    }
;

alter_lock_option_value -> String:
      'DEFAULT'
      {
          String::from("DEFAULT")
      }
    | ident
      {
          $1.0
      }
;

create_user_list -> Vec<UserWithAuthOption>:
      create_user
      {
           vec![$1]
      }
    | create_user_list ',' create_user
      {
           $1.push($3);
           $1
      }
;

create_user -> UserWithAuthOption:
      user identification
        {
          UserWithAuthOption::UserIdentification(UserIdentification {
              span: $span,
              user: $1,
              identification: $2,
              opt_create_user_with_mfa: None,
          })

        }
    | user identification create_user_with_mfa
      {
          UserWithAuthOption::UserIdentification(UserIdentification {
              span: $span,
              user: $1,
              identification: $2,
              opt_create_user_with_mfa: $3,
          })
      }
    | user identified_with_plugin create_user_with_mfa
      {
          UserWithAuthOption::UserIdentification(UserIdentification {
              span: $span,
              user: $1,
              identification: Identification::IdentifiedWithPlugin($2),
              opt_create_user_with_mfa: $3,
          })

      }

    | user identified_with_plugin
      {
          UserWithAuthOption::UserIdentification(UserIdentification {
              span: $span,
              user: $1,
              identification: Identification::IdentifiedWithPlugin($2),
              opt_create_user_with_mfa: None,
          })

      }

    | user identified_with_plugin initial_auth
      {
           UserWithAuthOption::UserIdentifiedWithPlugin(UserIdentifiedWithPlugin {
               span: $span,
               user: $1,
               identified_with_plugin: $2,
               opt_initial_auth: $3,
           })
      }

    | user create_user_with_mfa
      {
          UserWithAuthOption::UserWithMFA(UserWithMFA {
              span: $span,
              user: $1,
              opt_create_user_with_mfa: $2,
          })
      }

    | user
      {
          UserWithAuthOption::UserWithMFA(UserWithMFA {
              span: $span,
              user: $1,
              opt_create_user_with_mfa: None,
          })

      }
;

create_user_with_mfa -> Option<AuthMFA>:
    'AND' identification
      {
          Some(AuthMFA::Auth2FA(Auth2FA {
              span: $span,
              auth_2fa_option: $2,
          }))
      }
    | 'AND' identification 'AND' identification
      {
          Some(AuthMFA::Auth3FA(Auth3FA {
              span: $span,
              auth_2fa_option: $2,
              auth_3fa_option: $4,
          }))
      }
;

identification -> Identification:
      identified_by_password { Identification::IdentifiedByPassword($1) }
    | identified_by_random_password { Identification::IdentifiedByRandomPassword($1) }
    //| identified_with_plugin { Identification::IdentifiedWithPlugin($1) }
    | identified_with_plugin_as_auth { Identification::IdentifiedWithPluginAsAuth($1) }
    | identified_with_plugin_by_password { Identification::IdentifiedWithPluginByPassword($1) }
    | identified_with_plugin_by_random_password { Identification::IdentifiedWithPluginByRandomPassword($1) }
;

identified_by_password -> IdentifiedByPassword:
    'IDENTIFIED' 'BY' TEXT_STRING_password
    {
        IdentifiedByPassword {
            span: $span,
            auth_string: $3,
        }
    }
;

identified_by_random_password -> IdentifiedByRandomPassword:
    'IDENTIFIED' 'BY' 'RANDOM' 'PASSWORD'
    {
        IdentifiedByRandomPassword {
            span: $span,
        }
    }
;

identified_with_plugin -> IdentifiedWithPlugin:
    'IDENTIFIED' 'WITH' ident_or_text
    {
        IdentifiedWithPlugin {
            span: $span,
            auth_plugin: $3,
        }
    }
;

identified_with_plugin_as_auth -> IdentifiedWithPluginAsAuth:
    'IDENTIFIED' 'WITH' ident_or_text 'AS' TEXT_STRING_hash
    {
        IdentifiedWithPluginAsAuth {
            span: $span,
            auth_plugin: $3,
            auth_string: $5,
        }
    }
;

identified_with_plugin_by_password -> IdentifiedWithPluginByPassword:
    'IDENTIFIED' 'WITH' ident_or_text 'BY' TEXT_STRING_password
    {
        IdentifiedWithPluginByPassword {
            span: $span,
            auth_plugin: $3,
            auth_string: $5,
        }
    }
;

identified_with_plugin_by_random_password -> IdentifiedWithPluginByRandomPassword:
    'IDENTIFIED' 'WITH' ident_or_text 'BY' 'RANDOM' 'PASSWORD'
    {
        IdentifiedWithPluginByRandomPassword {
            span: $span,
            auth_plugin: $3,
        }
    }
;

default_role_clause -> Option<DefaultRoleClause>:
      /* empty */
      {
          None
      }
    |
      'DEFAULT' 'ROLE' role_list
      {
          Some(DefaultRoleClause {
              span: $span,
              roles: $3,
          })
      }
;

role_list -> Vec<Role>:
      role
      {
           vec![$1]
      }
    | role_list ',' role
      {
           $1.push($3);
           $1
      }
;

role -> Role:
      role_ident_or_text
      {
           Role {
               span: $span,
               role: $1,
           }
      }
    | role_ident_or_text '@' ident_or_text
      {
            $1.push('@');
            $1.push_str(&$3);
            Role {
                span: $span,
                role: $1,
            }
      }
;

require_clause -> Option<RequireClause>:
      /* empty */     { None }
    | 'REQUIRE' require_list
      {
           Some(RequireClause{
               span: $span,
               requires: Some($2),
           })
      }
    | 'REQUIRE' 'SSL'
      {
           Some(RequireClause{
               span: $span,
               requires: None,
           })
      }
    | 'REQUIRE' 'X509'
      {
           Some(RequireClause{
               span: $span,
               requires: None,
           })
      }
    | 'REQUIRE' 'NONE'
      {
           Some(RequireClause{
               span: $span,
               requires: None,
           })
      }
;

require_list -> Vec<RequireElement>:
      require_list_element opt_and require_list
      {
           $3.push($1);
           $3
      }
    | require_list_element
      {
           vec![$1]
      }
;

opt_and -> bool:
      /* empty */ { false }
    | 'AND'       { true }
;

require_list_element -> RequireElement:
      'SUBJECT' 'TEXT_STRING'
      {
          RequireElement {
              span: $span,
              require: String::from($lexer.span_str($2.as_ref().unwrap().span())),
          }
      }
    | 'ISSUER' 'TEXT_STRING'
      {
          RequireElement {
              span: $span,
              require: String::from($lexer.span_str($2.as_ref().unwrap().span())),
          }
      }
    | 'CIPHER' 'TEXT_STRING'
      {
          RequireElement {
              span: $span,
              require: String::from($lexer.span_str($2.as_ref().unwrap().span())),
          }
      }
;

connect_options -> Option<ConnectOptions>:
      /* empty */ { None }
    | 'WITH' connect_option_list
      {
           Some(ConnectOptions{
               span: $span,
               options: $2,
           })
      }
;

connect_option_list -> Vec<ConnectOption>:
      connect_option_list connect_option
      {
           $1.push($2);
           $1
      }
    | connect_option
      {
           vec![$1]
      }
;

connect_option -> ConnectOption:
      'MAX_QUERIES_PER_HOUR' ulong_num
      {
           ConnectOption {
               span: $span,
               num: $2,
           }
      }
    | 'MAX_UPDATES_PER_HOUR' ulong_num
      {
           ConnectOption {
               span: $span,
               num: $2,
           }
      }
    | 'MAX_CONNECTIONS_PER_HOUR' ulong_num
      {
           ConnectOption {
               span: $span,
               num: $2,
           }
      }
    | 'MAX_USER_CONNECTIONS' ulong_num
      {
           ConnectOption {
               span: $span,
               num: $2,
           }
      }
;

opt_account_lock_password_expire_options -> Option<AccountLockPasswordExpireOptions>:
      /* empty */    { None }
    | opt_account_lock_password_expire_option_list
      {
           Some(AccountLockPasswordExpireOptions{
               span: $span,
               options: $1,
           })
      }
;

opt_account_lock_password_expire_option_list -> Vec<AccountLockPasswordExpireOption>:
      opt_account_lock_password_expire_option
      {
           vec![$1]
      }
    | opt_account_lock_password_expire_option_list opt_account_lock_password_expire_option
      {
           $1.push($2);
           $1
      }
;

opt_account_lock_password_expire_option -> AccountLockPasswordExpireOption:
      'ACCOUNT' 'UNLOCK'
      {
           AccountLockPasswordExpireOption::AccountLock(AccountLock{
               span: $span,
           })
      }
    | 'ACCOUNT' 'LOCK'
      {
           AccountLockPasswordExpireOption::AccountLock(AccountLock{
               span: $span,
           })
      }
    | 'PASSWORD' 'EXPIRE'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
    | 'PASSWORD' 'EXPIRE' 'INTERVAL' real_ulong_num 'DAY'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: Some($4),
           })
      }
    | 'PASSWORD' 'EXPIRE' 'NEVER'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
    | 'PASSWORD' 'EXPIRE' 'DEFAULT'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
    | 'PASSWORD' 'HISTORY' real_ulong_num
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: Some($3),
           })
      }
    | 'PASSWORD' 'HISTORY' 'DEFAULT'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
    | 'PASSWORD' 'REUSE' 'INTERVAL' real_ulong_num 'DAY'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: Some($4),
           })
      }
    | 'PASSWORD' 'REUSE' 'INTERVAL' 'DEFAULT'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
    | 'PASSWORD' 'REQUIRE' 'CURRENT'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
    | 'PASSWORD' 'REQUIRE' 'CURRENT' 'DEFAULT'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
    | 'PASSWORD' 'REQUIRE' 'CURRENT' 'OPTIONAL'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
    | 'FAILED_LOGIN_ATTEMPTS' real_ulong_num
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: Some($2),
           })
      }
    | 'PASSWORD_LOCK_TIME' real_ulong_num
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: Some($2),
           })
      }
    | 'PASSWORD_LOCK_TIME' 'UNBOUNDED'
      {
           AccountLockPasswordExpireOption::PasswordExpire(PasswordExpire{
               span: $span,
               num: None,
           })
      }
;

opt_user_attribute -> Option<UserAttribute>:
      /* empty */    { None }
    | 'ATTRIBUTE' TEXT_STRING_literal
      {
           Some(UserAttribute{
               span: $span,
               content: $2,
           })
      }
    | 'COMMENT' TEXT_STRING_literal
      {
           Some(UserAttribute{
               span: $span,
               content: $2,
           })
      }
;

initial_auth -> Option<InitialAuth>:
      'INITIAL' 'AUTHENTICATION' identified_by_random_password
      {
          Some(InitialAuth::IdentifiedByRandomPassword($3))
      }
    | 'INITIAL' 'AUTHENTICATION' identified_with_plugin_as_auth
      {
          Some(InitialAuth::IdentifiedWithPluginAsAuth($3))
      }
    | 'INITIAL' 'AUTHENTICATION' identified_by_password
      {
          Some(InitialAuth::IdentifiedByPassword($3))
      }
;

/*
  This part of the parser contains common code for all TABLESPACE
  commands.
  CREATE TABLESPACE_SYM name ...
  ALTER TABLESPACE_SYM name ADD DATAFILE ...
  CREATE LOGFILE GROUP_SYM name ...
  ALTER LOGFILE GROUP_SYM name ADD UNDOFILE ..
  DROP TABLESPACE_SYM name
  DROP LOGFILE GROUP_SYM name
*/

opt_ts_datafile_name -> Option<AddTsDataFile>:
      /* empty */     { None }
    | 'ADD' ts_datafile
      {
          Some(AddTsDataFile {
              span: $span,
              ts_datafile: $2,
          })
      }
;

opt_logfile_group_name -> Option<LogFileGroup>:
      /* empty */   { None }
    | 'USE' 'LOGFILE' 'GROUP' ident
       {
            Some(LogFileGroup {
                span: $span,
                logfile_group: $4.0,
            })
       }
;

opt_tablespace_options -> Option<Vec<TablespaceOption>>:
      /* empty */     { None }
    | tablespace_option_list
      {
          Some($1)
      }
;

tablespace_option_list -> Vec<TablespaceOption>:
      tablespace_option
      {
            vec![$1]
      }
    | tablespace_option_list opt_comma tablespace_option
      {
            $1.push($3);
            $1
      }
;

tablespace_option -> TablespaceOption:
      ts_option_initial_size { TablespaceOption::InitialSize($1) }
    | ts_option_autoextend_size { TablespaceOption::AutoextendSize($1) }
    | ts_option_max_size { TablespaceOption::MaxSize($1) }
    | ts_option_extent_size { TablespaceOption::ExtentSize($1) }
    | ts_option_nodegroup { TablespaceOption::NodeGroup($1) }
    | ts_option_engine { TablespaceOption::Engine($1) }
    | ts_option_wait { TablespaceOption::Wait($1) }
    | ts_option_comment { TablespaceOption::Comment($1) }
    | ts_option_file_block_size { TablespaceOption::FileBlockSize($1) }
    | ts_option_encryption { TablespaceOption::Encryption($1) }
    | ts_option_engine_attribute { TablespaceOption::EngineAttribute($1) }
;

opt_alter_tablespace_options -> Option<Vec<AlterTablespaceOption>>:
      /* empty */    { None }
    | alter_tablespace_option_list { Some($1) }
;

alter_tablespace_option_list -> Vec<AlterTablespaceOption>:
      alter_tablespace_option
      {
           vec![$1]
      }
    | alter_tablespace_option_list opt_comma alter_tablespace_option
      {
           $1.push($3);
           $1
      }
;

alter_tablespace_option -> AlterTablespaceOption:
      ts_option_initial_size { AlterTablespaceOption::InitialSize($1) }
    | ts_option_autoextend_size { AlterTablespaceOption::AutoextendSize($1) }
    | ts_option_max_size { AlterTablespaceOption::MaxSize($1) }
    | ts_option_engine { AlterTablespaceOption::Engine($1) }
    | ts_option_wait { AlterTablespaceOption::Wait($1) }
    | ts_option_encryption { AlterTablespaceOption::Encryption($1) }
    | ts_option_engine_attribute { AlterTablespaceOption::EngineAttribute($1) }
;

opt_undo_tablespace_options -> Option<Vec<UndoTablespaceOption>>:
      /* empty */   { None }
    | undo_tablespace_option_list { Some($1) }
;

undo_tablespace_option_list -> Vec<UndoTablespaceOption>:
      undo_tablespace_option
      {
           vec![$1]
      }
    | undo_tablespace_option_list opt_comma undo_tablespace_option
      {
           $1.push($3);
           $1
      }
;

undo_tablespace_option -> UndoTablespaceOption:
    ts_option_engine { UndoTablespaceOption::Engine($1) }
;

opt_logfile_group_options -> Option<Vec<LogFileGroupOption>>:
      /* empty */ { None }
    | logfile_group_option_list
      {
          Some($1)
      }
;

logfile_group_option_list -> Vec<LogFileGroupOption>:
     logfile_group_option
     {
           vec![$1]
      }
    | logfile_group_option_list opt_comma logfile_group_option
      {
           $1.push($3);
           $1
      }
;

logfile_group_option -> LogFileGroupOption:
      ts_option_initial_size { LogFileGroupOption::InitialSize($1) }
    | ts_option_undo_buffer_size { LogFileGroupOption::UndoBufferSize($1) }
    | ts_option_redo_buffer_size { LogFileGroupOption::RedoBufferSize($1) }
    | ts_option_nodegroup { LogFileGroupOption::NodeGroup($1) }
    | ts_option_engine { LogFileGroupOption::Engine($1) }
    | ts_option_wait { LogFileGroupOption::Wait($1) }
    | ts_option_comment { LogFileGroupOption::Comment($1) }
;

opt_alter_logfile_group_options -> Option<Vec<AlterLogFileGroupOption>>:
      /* empty */   { None }
    | alter_logfile_group_option_list { Some($1) }
;

alter_logfile_group_option_list -> Vec<AlterLogFileGroupOption>:
       alter_logfile_group_option
       {
            vec![$1]
       }
     | alter_logfile_group_option_list opt_comma alter_logfile_group_option
       {
            $1.push($3);
            $1
       }
;

alter_logfile_group_option -> AlterLogFileGroupOption:
      ts_option_initial_size { AlterLogFileGroupOption::InitialSize($1) }
    | ts_option_engine { AlterLogFileGroupOption::Engine($1) }
    | ts_option_wait { AlterLogFileGroupOption::Wait($1) }
;

ts_datafile -> TsDataFile:
    'DATAFILE' TEXT_STRING_sys
    {
        TsDataFile {
            span: $span,
            file_name: $2,
        }
    }
;

undo_tablespace_state -> UndoTablespaceState:
      'ACTIVE'   { UndoTablespaceState::Active }
    | 'INACTIVE' { UndoTablespaceState::Inactive }
;

lg_undofile -> UndoFile:
    'UNDOFILE' TEXT_STRING_sys
    {
        UndoFile {
            span: $span,
            file_name: $2,
        }
    }
;

ts_option_initial_size -> SizeOption:
    'INITIAL_SIZE' opt_equal size_number
    {
         let is_equal = match $2 {
             Some(_) => true,
             None => false,
         };
         SizeOption {
             span: $span,
             is_equal: is_equal,
             size: $3,
         }
    }
;

ts_option_autoextend_size -> SizeOption:
    option_autoextend_size
    {
         $1
    }
;

option_autoextend_size -> SizeOption:
    'AUTOEXTEND_SIZE' opt_equal size_number
    {
         let is_equal = match $2 {
             Some(_) => true,
             None => false,
         };
         SizeOption {
             span: $span,
             is_equal: is_equal,
             size: $3,
         }
    }
;

ts_option_max_size -> SizeOption:
    'MAX_SIZE' opt_equal size_number
     {
         let is_equal = match $2 {
             Some(_) => true,
             None => false,
         };
         SizeOption {
             span: $span,
             is_equal: is_equal,
             size: $3,
         }
     }
;

ts_option_extent_size -> SizeOption:
    'EXTENT_SIZE' opt_equal size_number
     {
          let is_equal = match $2 {
              Some(_) => true,
              None => false,
          };
          SizeOption {
              span: $span,
              is_equal: is_equal,
              size: $3,
          }
     }
;

ts_option_undo_buffer_size -> SizeOption:
    'UNDO_BUFFER_SIZE' opt_equal size_number
    {
         let is_equal = match $2 {
             Some(_) => true,
             None => false,
         };
         SizeOption {
             span: $span,
             is_equal: is_equal,
             size: $3,
         }
    }
;

ts_option_redo_buffer_size -> SizeOption:
    'REDO_BUFFER_SIZE' opt_equal size_number
    {
         let is_equal = match $2 {
             Some(_) => true,
             None => false,
         };
         SizeOption {
             span: $span,
             is_equal: is_equal,
             size: $3,
         }
    }
;

ts_option_nodegroup -> NodeGroupOption:
    'NODEGROUP' opt_equal real_ulong_num
    {
         let is_equal = match $2 {
             Some(_) => true,
             None => false,
         };
         NodeGroupOption {
             span: $span,
             is_equal: is_equal,
             nodegroup_id: $3,
         }
    }
;

ts_option_comment -> CommentOption:
    'COMMENT' opt_equal 'TEXT_STRING'
    {
         let is_equal = match $2 {
             Some(_) => true,
             None => false,
         };
         CommentOption {
             span: $span,
             is_equal: is_equal,
             comment: String::from($lexer.span_str($3.as_ref().unwrap().span())),
         }
    }
;

ts_option_engine -> EngineOption:
    opt_storage 'ENGINE' opt_equal ident_or_text
    {
         let is_equal = match $3 {
             Some(_) => true,
             None => false,
         };
         EngineOption {
             span: $span,
             opt_storage: $1,
             is_equal: is_equal,
             engine_name: $4,
         }
    }
;

ts_option_file_block_size -> SizeOption:
    'FILE_BLOCK_SIZE' opt_equal size_number
     {
         let is_equal = match $2 {
             Some(_) => true,
             None => false,
         };
         SizeOption {
             span: $span,
             is_equal: is_equal,
             size: $3,
         }
     }
;

ts_option_wait -> WaitOption:
     'WAIT'
     {
          WaitOption::Wait
     }
    | 'NO_WAIT'
     {
          WaitOption::NoWait
     }
;

ts_option_encryption -> EncryptionOption:
    'ENCRYPTION' opt_equal TEXT_STRING_sys
     {
          let is_equal = match $2 {
              Some(_) => true,
              None => false,
          };
          EncryptionOption {
              span: $span,
              is_equal: is_equal,
              encryption: $3,
          }
     }
;

ts_option_engine_attribute -> EngineAttributeOption:
    'ENGINE_ATTRIBUTE' opt_equal json_attribute
     {
          let is_equal = match $2 {
              Some(_) => true,
              None => false,
          };
          EngineAttributeOption {
              span: $span,
              is_equal: is_equal,
              attribute: $3,
          }
     }
;

size_number -> String:
      real_ulonglong_num { $1 }
    | IDENT_sys { $1.0 }
;

/*
  End tablespace part
*/

server_options_list -> Vec<ServerOption>:
      server_option
      {
           vec![$1]
      }
    | server_options_list ',' server_option
      {
           $1.push($3);
           $1
      }
;

server_option -> ServerOption:
      'USER' TEXT_STRING_sys
      {
          ServerOption::User(StringOption {
              span: $span,
              content: $2,
          })
      }
    | 'HOST' TEXT_STRING_sys
      {
          ServerOption::Host(StringOption {
              span: $span,
              content: $2,
          })
      }
    | 'DATABASE' TEXT_STRING_sys
      {
          ServerOption::Database(StringOption {
              span: $span,
              content: $2,
          })
      }
    | 'OWNER' TEXT_STRING_sys
      {
          ServerOption::Owner(StringOption {
              span: $span,
              content: $2,
          })
      }
    | 'PASSWORD' TEXT_STRING_sys
      {
          ServerOption::Password(StringOption {
              span: $span,
              content: $2,
          })
      }
    | 'SOCKET' TEXT_STRING_sys
      {
          ServerOption::Socket(StringOption {
              span: $span,
              content: $2,
          })
      }
    | 'PORT' ulong_num
      {
          ServerOption::Port(StringOption {
              span: $span,
              content: $2,
          })
      }
;

%%

use lrpar::Span;
use crate::ast::*;
