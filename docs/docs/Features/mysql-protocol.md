---
sidebar_position: 2
---
# MySQL 协议 

此库主要为 Pisa-Proxy MySQL 代理的核心组件, 本库为 [MySQL 协议](https://dev.mysql.com/doc/internals/en/client-server-protocol.html) 的实现。该库大量使用了由 Rust 实现的 [Tokio](https://github.com/tokio-rs/tokio) 异步运行时框架。其中对网络数据包的读写、协议的编码等操作都通过 Tokio 提供的工具集实现。


## 介绍

### 说明

本模块共由3部分组成,对应 Pisa-Proxy 中作为客户端和服务端两部分。在 server 目录中主要定义了 Pisa-Proxy 作为服务端对客户端请求的处理逻辑。也包含了在 TCP 层的网络数据包的读写操作。在 client 目录中定义了 Pisa-Proxy 作为客户端对 MySQL 数据库的建立链接、握手认证和发送客户端命令等操作。

### 代码结构

	FILES IN THIS DIRECTORY (protocol/mysql/src/client)
		auth.rs          - 对 MySQL 登陆认证
		codec.rs         - 对 MySQL 协议的解码
		conn.rs          - 对 MySQL 发起链接、握手及命令处理
		err.rs           - MySQL Error 类型定义
		resultset.rs     - 一些 MySQL ResultSet 结果处理方法
		stmt.rs          - 对 MySQL Prepare Statement 处理方法
		stream.rs        - 对 Tokio TcpSteam 封装

	FILES IN THIS DIRECTORY (protocol/mysql/src/server)
		conn.rs          - 和客户端建链、握手认证处理
		packet.rs        - 网络数据包的读写操作
		stream.rs        - 对底层 Tcp 流的封装
		tls.rs           - MySQL TLS 链接封装

	FILES IN THIS DIRECTORY (protocol/mysql/src)
		charset.rs       - MySQL 字符集声明
		err.rs           - 协议解析错误处理定义
		mysql_const.rs。 - 常量定义
		util.rs          - 一些通用函数的实现


## 已支持协议
- [x] COM_INIT_DB
- [x] COM_QUERY
- [x] COM_FIELD_LIST
- [x] COM_QUIT
- [x] COM_PING
- [x] COM_STMT_PREPARE
- [x] COM_STMT_EXECUTE
- [x] COM_STMT_CLOSE
- [x] COM_STMT_RESET 

## 支持认证方式
- [x] mysql_native_password
- [ ] sha256_password
- [ ] caching_sha2_password 
