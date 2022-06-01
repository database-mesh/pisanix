---
sidebar_position: 2
---

# 单独部署 Pisa-Proxy  

***Pisa-Proxy*** 作为高性能代理不仅可以在 kubernetes 中以 Sidecar 的方式部署，也可以作为统一接入层单独部署在 kubernetes 之外的服务器上：

![single.png](/img/single.png)

目前***Pisa-Proxy***支持 MySQL 协议，无论后端是云上的 RDS 实例，还是自建的 MySQL、ShardingSphere、TiDB 等，都可以由 Pisa-Proxy 统一流量分发，实现无感的高可用切换、面向 SQL 的可观测性等。

## 部署说明

Pisa-Proxy 支持从配置文件和 Remote API 获取配置。在当前版本,若需要从本地文件加载配置需要导出 ```LOCAL_CONFIG=true``` 环境变量，并通过 ```-c，--config``` 参数指定配置文件路径。若不指定，默认从 ```./etc/config.toml``` 文件中进行加载。Pisa-Proxy 支持通过命令行参数和环境变量进行服务启动配置。配置详解如下：

### 命令行参数
```
./pisa-proxy -h
Pisa-Proxy 

USAGE:
    pisa-proxy [OPTIONS]

OPTIONS:
    -c, --config <config>         Config path               # 指定配置文件路径
    -h, --help                    Print help information    # 查看使用帮助
        --log-level <loglevel>    Log level                 # 指定日志级别
    -p, --port <port>             Http port                 # 指定 api 端口号
```

### 环境变量

环境变量包括如下：
1. PORT: HTTP 服务启动端口号
2. LOG_LEVEL: 日志级别
3. LOCAL_CONFIG: 指定 Pisa-Proxy 从本地加载配置

### 配置文件

```
# api 配置块，对应命令行参数和环境变量
[admin]
# api 端口
port = "8081"
# 日志级别
log_level = "INFO"

# pisa-proxy 代理配置块
[proxy]
# config a proxy
[[proxy.config]]
# proxy 代理地址
listen_addr = "0.0.0.0:9088"
# proxy 认证用户名
user = "root"
# proxy 认证密码
password = "12345678"
# proxy schema
db = "test"
# 配置后端数据源类型
backend_type = "mysql"
# proxy 与后端数据库建连连接池大小，值范围：1 ~ 255, 默认值：64
pool_size = 3

# 后端负载均衡配置
[proxy.config.simple_loadbalance]
# 负载均衡算法：[random/roundrobin], 默认值: random 算法
balance_type = "random"
# 选择挂载后端节点
nodes = ["ds001"]

# 后端数据源配置
[mysql]
[[mysql.node]]
# 数据源 name
name = "ds001"
# database name
db = ""
# 数据库 user
user = "root"
# 数据库 password
password = "root"
# 数据库地址
host = "127.0.0.1"
# 数据库端口
port = 3307
```

### 配置示例
#### 配置多个代理
```
[admin]
port = "8081"
log_level = "INFO"

[proxy]
[[proxy.config]]
listen_addr = "0.0.0.0:9088"
user = "root"
password = "12345678"
db = "test"
backend_type = "mysql"
pool_size = 3

[proxy.config.simple_loadbalance]
balance_type = "random"
nodes = ["ds001"]

[proxy]
[[proxy.config]]
listen_addr = "0.0.0.0:9089"
user = "root"
password = "root"
db = "test"
backend_type = "mysql"
pool_size = 3

[proxy.config.simple_loadbalance]
balance_type = "random"
nodes = ["ds001"]

[mysql]
[[mysql.node]]
name = "ds001"
db = "test"
user = "root"
password = "root"
host = "127.0.0.1"
port = 3307
```

#### 配置后端数据库负载均衡
```
[proxy]
[[proxy.config]]
listen_addr = "0.0.0.0:9089"
user = "root"
password = "root"
db = "test"
backend_type = "mysql"
pool_size = 3

[proxy.config.simple_loadbalance]
balance_type = "random"
nodes = ["ds001", "ds002"]

[mysql]
[[mysql.node]]
name = "ds001"
db = "test"
user = "root"
password = "root"
host = "127.0.0.1"
port = 3307

[[mysql.node]]
name = "ds002"
db = "test"
user = "root"
password = "root"
host = "127.0.0.1"
port = 3308
```
在部署 ***Pisa-Proxy*** 后即可使用应用根据配置信息进行访问。

