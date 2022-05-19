---
sidebar_position: 1
---

# 简介 

`Pisanix` [Pi-sanics] 是一款面向 Kubernetes 的数据库治理框架。Pisanix 通过 SQL 感知的流量治理、审计、安全和扩展性等能力实现 [Database Mesh](https://www.database-mesh.io) 风格的数据库治理体验。

## 一、概述

Pisanix 关注如下几个问题:

* SQL 感知的流量治理：支持 SQL 流量负载均衡、访问控制和可观测性。 
* 运行时资源管理: 支持多种可扩展的资源控制能力 
* 数据库可靠性工程：简化 Kubernetes 环境下数据库的治理 

Pisanix 的架构图如下：

![Pisanix Arch](/img/pisanix-arch.png)

三个组件的功能分别为：

* ***Pisa-Controller***: 用 Go 实现的控制面，提供对数据面组件的管控，如 Sidecar 注入、配置转换和下发，是 Pisanix 所有配置的入口。

* ***Pisa-Proxy***: 用 Rust 实现的高性能量代理，通过 MySQL 协议获取应用的数据库访问流量，并基于此实现 SQL 流量治理、访问控制、防火墙、可观测性等各种治理能力。

* ***Pisa-Daemon***: 用 Go 实现的数据面，部署在集群中每个节点上，通过宿主机内核的各种能力提供可编程资源管理，如 TrafficQoS 等。


## 二、 特性

### 2.1  数据库流量治理 

应用通过 SQL 访问数据库，Pisanix 可以劫持所有的 SQL 流量。借助这个能力，Pisanix 可以实现多种流量治理能力，如负载均衡、SQL 防火墙等。

### 2.2 可观测性 

数据库的监控指标通常从相关实例处获取，借助 Pisanix 可以透视多种数据库访问指标。

### 2.3 可编程 

Pisanix 支持多种插件机制，如 Lua 和 Wasm，工程师们有机会重新定义数据库各种行为。


## 三、快速开始 

- [简介](https://www.pisanix.io/docs)
- [快速开始](https://www.pisanix.io/docs/quickstart)

## 四、文档 

所有文档可以在 [Pisanix 站点查看](https://www.pisanix.io/).

## 五、社区和支持 

 :link: [GitHub Issues](https://github.com/database-mesh/pisanix/issues). 适用于 Bug 上报、特性讨论等开发相关内容。

 :link: [Slack channel](https://join.slack.com/t/databasemesh/shared_invite/zt-12hlythpe-C4rrS1WZ2ZkEd3zn84SqeQ). 适用于：即时交流、线上会议、使用场景交流等。

- 微信交流群: 添加小助手微信邀请进群 

![Wechat user group broker](/img/wechat-user-group-broker.jpeg)
