---
sidebar_position: 3
---

# 读写分离

读写分离是业界使用 MySQL 最常用的方案之一， 在实际场景中可以提高查询性能，降低服务器负载。一般架构如以下：

```
+--------+     +-------+          +-----------+
| Client | --> | Proxy | -+-----> | ReadNode1 |
+--------+     +-------+  |       +-----------+
                          |       +-----------+
                          +-----> | ReadNode2 |
                          |       +-----------+
                          |       +-----------+
                          +-----> | ReadNode3 |
                          |       +-----------+
                          |       +-----------+
                          +-----> | WriteNode |
                                  +-----------+

```

读写分离是 Pisa-Proxy 流量治理的一部分。

Pisa-Proxy 支持两种类型的读写分离方案：
- 静态读写分离

`指无动态感知后端数据库角色变更。配置说明[请见](#读写分离配置)`

- 动态读写分离(目前还不支持)

`指能够感知后端数据库角色变更。`

目前，两种方案都需要配置[读写分离规则](#读写分离规则说明)。

## 名词解释：

本章中出现的`关键字`代表的含义。

节点： 指后端数据库节点

内部逻辑如下：

```
+------------+     +------------+     +-------------+     +----------------+
| RulesMatch | --> | TargetRole | --> | LoadBalance | --> | TargetInstance |
+------------+     +------------+     +-------------+     +----------------+
```


RulesMatch: RulesMatch 引擎通过编写的规则集，与 Pisa-Proxy 接收到的 SQL 查询语句做匹配。

TargetRole: 指通过规则匹配引擎匹配到的 TargetRole 组，每个 TargetRole 组里可能会有一个或多个节点。

LoadBalance: 负载均衡模块会按照相应的算法从 TargetRole 组里选取一个合适的节点。

TargetInstnce: 指由 LoadBalance 模块选出的节点。 

## 读写分离配置
配置说明：

| 属性 | 值类型 | 是否依赖 | 默认值 | 含义 |
|-----|-------|---------|-------|-----|
| static | [object](#静态读写分离配置) |  否     |  无   | 静态读写分离类型 |
| dynamic| object| 否     |  无   | 动态读写分离类型， ***目前还不支持*** |

### 静态读写分离配置
配置说明：

| 属性 | 值类型 | 是否依赖 | 默认值 | 含义 |
|-----|-------|---------|-------|-----|
| defaultTarget | [enum](#targetrole-enum-值) |  否     |  无   | 默认路由的 TargetRole 组 |
| rules | [array\<rule\>](#读写分离规则说明)| 是    |  无   | 读写分离规则|

***<span style="color:#0F0">defaultTarget</span> 值在真实场景中要配置成 `readwrite`***

## 读写分离规则说明
读写分离规则是指 Pisa-Proxy 需要把查询的 SQL 语句和配置的规则做匹配，如果匹配成功，将根据规则把 SQL 语句路由到对应的节点上。如果匹配不成功，SQL 语句将被路由到默认的节点上。

目前读写分离规则只支持通过正则去匹配。

正则规则配置说明：

| 属性 | 值类型 | 是否依赖 | 默认值 | 含义 |
|-----|-------|---------|-------|-----|
|name | string|  是     |  无   | 规则名称|
|type | string| 是      |  无   | 路由类型为正则类型，此处值为 `regex`|
|regex| array\<string\>| 是 | 无  | 具体的正则，将通过这些正则去匹配 SQL 语句|
|target| [enum](#targetrole-enum-值)| 是     | 无    | 路由到的 TargetRole 组，对应 DatabaseEndpoint CRD 的Annotation 属性中 `database-mesh.io/role` 的 值|
|algorithName| [enum](#algorithname-enum-值)| 是| 无   | 负载均衡算法的名称，将作用与路由到的 `role` 组中的机器列表|

### TargetRole Enum 值
- read
- readwrite

### algorithName Enum 值
- random
- roundrobin

## 配置示例

一个完整的 `TrafficStrategy` CRD 配置如下：

``` yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: TrafficStrategy
metadata:
  name: test 
  namespace: default 
spec:
  selector:
    matchLabels:
      source: test
  loadBalance:
    readWriteSplitting:
      static:  
        defaultTarget: read // or read_write
        rules:
          - name: read-rule
            type: regex
            regex: ".*"
            target: read // read_write
            algorithmName: random // lb algorithm
          - name: write-rule
            type: regex
            regex: ".*"
            target: read_write
            algorithmName: roundrobin
```

一个完整的 `DatabaseEndpoint` CRD 配置如下：
``` yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: DatabaseEndpoint
metadata:
  annotations:
    database-mesh.io/role: read // or read_write
    database-mesh.io/weight: 1
  labels:
    source: test 
  name: mysql 
  namespace: default 
spec:
  database:
    MySQL:
      db: test 
      host: mysql.default 
      password: root 
      port: 3306
      user: root
```