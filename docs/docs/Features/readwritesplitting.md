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

指无动态感知后端数据库角色变更。配置说明[请见](#读写分离配置)。

- 动态读写分离

指能够感知后端数据库角色变更，配置说明[请见](#动态读写分离)。

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
| rules | [array[rule]](#读写分离规则说明)| 是    |  无   | 读写分离规则|

***defaultTarget 值在真实场景中要配置成 `readwrite`***

## 读写分离规则说明
读写分离规则是指 Pisa-Proxy 需要把查询的 SQL 语句和配置的规则做匹配，如果匹配成功，将根据规则把 SQL 语句路由到对应的节点上。

目前读写分离规则支持通用规则和正则匹配两种方式。配置通用规则后，Pisa-Proxy 会根据 SQL 自动路由请求。若配置正则规则，则 Pisa-Proxy 会基于正则表达式行路由。此处用户可以同时配置两种路由规则，Pisa-Proxy 会优先根据正则进行路由，若未匹配到则通过通用规则进行路由，若两种规则全未命中，则 SQL 会被路由到默认节点上。

通用规则配置说明:

| 属性 | 值类型 | 是否依赖 | 默认值 | 含义 |
|-----|-------|---------|-------|-----|
|name | string|  是     |  无   | 规则名称|
|type | string|  是     |  无   | 路由类型为通用规则类型，此处值为 `generic`|
|algorithName| [enum](#algorithname-enum-值)| 是| 无   | 负载均衡算法的名称，将作用与路由到的 `role` 组中的机器列表|

正则规则配置说明：

| 属性 | 值类型 | 是否依赖 | 默认值 | 含义 |
|-----|-------|---------|-------|-----|
|name | string|  是     |  无   | 规则名称|
|type | string| 是      |  无   | 路由类型为正则类型，此处值为 `regex`|
|regex| array[string]| 是 | 无  | 具体的正则，将通过这些正则去匹配 SQL 语句|
|target| [enum](#targetrole-enum-值)| 是     | 无    | 路由到的 TargetRole 组，对应 DatabaseEndpoint CRD 的Annotation 属性中 `database-mesh.io/role` 的 值|
|algorithName| [enum](#algorithname-enum-值)| 是| 无   | 负载均衡算法的名称，将作用与路由到的 `role` 组中的机器列表|

### TargetRole Enum 值
- read
- readwrite

### algorithName Enum 值
- random
- roundrobin

## 配置示例

基于通用路由规则的 `TrafficStrategy` CRD 配置如下:
```yaml
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
        defaultTarget: read # or readwrite
        rules:
          - name: read-rule
            type: generic
            algorithmName: random # lb algorithm
```

基于正则匹配的 `TrafficStrategy` CRD 配置如下：

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
        defaultTarget: read # or readwrite
        rules:
          - name: read-rule
            type: regex
            regex:
              - "^select"
            target: read # readwrite
            algorithmName: random # lb algorithm
          - name: write-rule
            type: regex
            regex:
              - "^insert"
            target: readwrite
            algorithmName: roundrobin
```

一个完整的 `DatabaseEndpoint` CRD 配置如下：
``` yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: DatabaseEndpoint
metadata:
  annotations:
    database-mesh.io/role: read # or readwrite
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

## 动态读写分离

动态读写分离是在静态读写分离基础之上加入了对后端数据源的动态感知，Pisa-Proxy 主动探测后端集群节点状态和主从之间的状态，并根据探测到的状态来动态调整 Pisa-Proxy 的路由策略。在动态读写分离中主要包含了 Discovery，Monitor 两个模块用来感知后端数据源状态。

#### 名词解释
Discovery: discovery 指 Pisa-Proxy 对后端高可用的感知方式，例如: MHA，RDS，MGR等等。在当前版本中支持 MHA 的方式。

Monitor: monitor 为对后端数据源的探测模块，其中包含了以下4种探测方式。

- Connect Monitor: 探测后端数据源的网络层连通性。
- Ping Monitor: 探测后端数据源是否健康。
- Replication Lag Monitor: 探测后端数据源主从复制状态和延迟情况。
- Read Only Monitor: 探测后端数据源角色。

#### 配置项
|参数 | 类型| 是否依赖| 默认值 | 含义|
|-- | -- | -- | -- | --|
|user | string | 是 | None |探测模块执行检查 SQL 语句用户名|
|password | string | 是 | None |探测模块执行检查 SQL 语句密码|
|monitor_period | u64 | 是 | 1000 |探测模块更新感知后端数据源状态周期(毫秒)|
|connect_period | u64 | 是 | 1000 |connect 模块探测周期(毫秒)|
|connect_timeout | u64 | 是| 6000 |connect 模块探测超时时间(毫秒)|
|connect_failure_threshold | u64 | 是 | 1 |connect 模块探测失败重试次数|
|ping_period | u64 | 是 | 1000 |ping 模块探测周期(毫秒)|
|ping_timeout | u64 | 是 | 6000 |ping 模块探测超市时间(毫秒)|
|ping_failure_threshold | u64 | 是 | 1 |ping 模块探测失败重试次数|
|replication_lag_period | u64 | 是 | 1000 |replication lag 模块探测周期(毫秒)|
|replication_lag_timeout | u64 | 是 | 6000|replication lag 模块探测超时时间(毫秒)|
|replication_lag_failure_threshold | u64 | 是 | 1 |replication lag 探测失败重拾次数|
|max_replication_lag | u64 | 是 | 10000 |用户定义主从最大延迟时间阈值(毫秒)|
|read_only_period | u64 | 是 | 1000 |read only 探测周期(毫秒)|
|read_only_timeout | u64 | 是 | 6000|read only 探测超市时间(毫秒)|
|read_only_failure_threshold | u64 | 是 | 3 |read only 探测失败重拾次数|


#### 一个完整的 TrafficStrategy CRD 配置如下：
```yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: TrafficStrategy
metadata:
  name: catalogue
  namespace: demotest
spec:
  selector:
    matchLabels:
      source: test
  loadBalance:
    readWriteSplitting:
      dynamic:
        defaultTarget: readwrite
        discovery:
          masterHighAvailability:
            connectionProbe:
              failureThreshold: 3
              periodMilliseconds: 1000
              timeoutMilliseconds: 6000
            monitorPeriod: 1000
            pingProbe:
              failureThreshold: 3
              periodMilliseconds: 1000
              timeoutMilliseconds: 6000
            readOnlyProbe:
              failureThreshold: 3
              periodMilliseconds: 1000
              timeoutMilliseconds: 6000
            replicationLagProbe:
              failureThreshold: 3
              maxReplicationLag: 3
              periodMilliseconds: 1000
              timeoutMilliseconds: 6000
            user: monitor
            password: monitor
        rules:
        - algorithmName: roundrobin
          name: write-rule
          regex:
          - ^insert
          target: readwrite
          type: regex
        - algorithmName: roundrobin
          name: read-rule
          regex:
          - ^select
          target: read
          type: regex
```