---
sidebar_position: 1
---

# 负载均衡

负载均衡模块为 Pisa-Proxy 代理后端节点时，新建链接时对后端节点选取策略的实现。

### 说明
该模块定义了一个负载均衡的 Trait，封装了在 proxy 中对于负载均衡的构建方式。以及定义了 Random 和 RoundRobin 两种负载均衡策略所要实现的具体方法。

### 代码结构
	FILES IN THIS DIRECTORY (loadbalancer/src)
		balancer.rs              - 负载均衡 Trait，定义了负载均衡方法和构建负载均衡模块
		random_weighted.rs       - random weighted 负载均衡策略
		roundrobin_weighted.rs   - roundrobin weighted 负载均衡策略

## 已支持负载均衡策略
- [x] Random
- [x] Roundbin

## 配置示例
1. 若需要在代理中启用负载均衡，则需要配置 `simple_loadbalancer` 块.
```
[proxy.configs.simple_loadbalancer]
balancer_type = "roundrobin"
nodes = ["ds001","ds002"]
```
balancer_type: 可选值为 random，roundrobin，默认值为 random.

nodes: 选取后端数据源中定义的 `name`.

2. 在后端数据源配置中需配置 `weight` 字段，该字段指定该节点在负载均衡中的权重.
```
[[mysql.nodes]]
name = "ds001"
user = "root"
password = "12345678"
db = "test"
addr = "127.0.0.1:3306"
weight = 1

[[mysql.nodes]]
name = "ds002"
user = "root"
password = "12345678"
db = "test"
addr = "127.0.0.1:3307"
weight = 1
```

