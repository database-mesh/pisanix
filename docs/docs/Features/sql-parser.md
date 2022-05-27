---
sidebar_position: 3
---

# SQL解析

Pisanix 除了理解 SQL 协议外, 能读懂 SQL 语句也是一个很重要的功能，读写分离，分片等功能也都依赖 SQL 解析，在 Pisanix 中占着举足轻重的作用。
由于 Pisanix 支持多种数据源，因此在 Pisanix 中，每种不同的数据源都要实现各自对应的 SQL 解析，未来我们会支持不同数据源使用SQL 之间的转换，以实现快速支持新的数据源。


## 实现
目前 Pisanix 只实现了对 MySQL 的语法的解析。

## MySQL
为了最大的程度的兼容原生的 MySQL 语法，PISANIX 采用原生的[ MySQL 语法文件](https://github.com/mysql/mysql-server/blob/8.0/sql/sql_yacc.yy)，基于 `Grmtools` 实现了 SQL 语法解析。
 `Grmtools` 是一个用 RUST 写的兼容 `Yacc` 的语法解析工具，详细信息请参考[ GITHUB ](https://github.com/softdevteam/grmtools.git)。

### 目前状态
当前还有些 SQL 语句不支持，我们还在不断快速完善中。

- [x] SELECT
- [x] INSERT
- [x] UPDATE
- [x] DELETE
- [x] PREPARE
- [x] EXECUTE
- [x] BEGIN
- [x] SET
- [ ] SHOW
- [ ] CREATE

### AST 说明
Pisanix 中的 SQL 解析不会为所有表达式都生成 AST，只会为 Pisanix 感兴趣的部分生成 AST。
详细信息请参考[ GITHUB ](https://github.com/database-mesh/pisanix)。

### 测试
由于 SQL 语句的复杂多样性，很难有有一个完整的测试集能覆盖所有可能的 SQL 语句。
我们使用了 MySQL TEST 框架中能正常运行的sql语句作为测试集来测试。
目前测试正在进行中，只测试了 `SELECT` 语句，`98%`的语句能成功解析, 还在不断完善中。


