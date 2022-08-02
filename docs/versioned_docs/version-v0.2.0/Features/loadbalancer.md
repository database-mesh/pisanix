---
sidebar_position: 1
---

# 负载均衡

负载均衡模块为 Pisa-Proxy 代理后端节点时，新建链接时对后端节点选取策略的实现。

## 已支持负载均衡策略
- [x] Random
- [x] RoundRobin

## 配置示例

若需要在代理中启用负载均衡，则需要配置 `simple_loadbalance` 块.
```
[proxy.config.simple_loadbalance]
balance_type = "roundrobin"
nodes = ["ds001","ds002"]
```
其中 balance_type: 可选值为 random，roundrobin，默认值为 random；nodes: 选取后端数据源中定义的 `name`.

后端数据源配置：
```
[[mysql.node]]
name = "ds001"
user = "root"
password = "12345678"
db = "test"
host = "127.0.0.1"
port = 3306

[[mysql.node]]
name = "ds002"
user = "root"
password = "12345678"
db = "test"
host = "127.0.0.1"
port = 3307
```

## 模块设计

该模块定义了一个负载均衡的 Trait，封装了在 Pisa-Proxy 中对于负载均衡的构建方式。以及定义了 Random 和 RoundRobin 两种负载均衡策略所要实现的具体方法。

### 代码结构
	FILES IN THIS DIRECTORY (loadbalance/src)
		balance.rs               - 负载均衡 Trait，定义了负载均衡方法和构建负载均衡模块
		random_weighted.rs       - random weighted 负载均衡策略
		roundrobin_weighted.rs   - roundrobin weighted 负载均衡策略

