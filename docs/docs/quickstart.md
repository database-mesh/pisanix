---
sidebar_label: '快速开始'
sidebar_position: 2
---

# 快速开始

## 安装教程

### Pisa-Controller admission webhooks 证书配置

Pisa-Controller admission webhooks 和 kube-apiserver 通信需要使用 https 协议,我们需要对我们的 https 服务进行证书配置。

执行webhook-create-signed-cert.sh ，脚本中会生成自签名证书，并生成对应的csr ，从csr中获取token作为apiserver-server 的交互证书。

**namespace: default,如果要更改 namespace，请修改 webhook-create-signed-cert.sh 中 namespace=default ，将 default 更换为您的 namespace** 

脚本会安装下列资源对象

* csr  ${servicename}.${namespace}
* secret ${secretname}

```
./webhook-create-signed-cert.sh
```


### 配置 MutatingWebhookConfiguration

使用 kubectl 中的 ca 证书替换 mutatingwebhook.yaml 中的 caBundle 字段,如果您在1.1中修改过 namespace ，请将 mutatingwebhook.yaml.tpl 文件中的 namespace: default 字段改成您的namespace,MutatingWebhookConfiguration 自身没有 namespace 限制

此步骤将安装下列资源对象

* MutatingWebhookConfiguration

```
cat deploy/mutatingwebhook.yaml.tpl  | ./tool/cert/webhook-patch-ca-bundle.sh > ./deploy/mutatingwebhook.yaml

kubectl apply  -f ./deploy/mutatingwebhook.yaml
```


### 应用 Pisa-Proxy 配置 CRD

Pisa-Proxy 通过 http 和控制面进行交互以获取启动配置文件。配置文件以 CRD 形式保存在 Kubernetes 集群中。

此步骤将安装下列资源对象

* CustomResourceDefinition

```
kubectl apply -f networking.pisanix.io_proxyconfigs.yaml
```

### 安装 Pisa-Controller 

在上文中我们已经完成了在 kuebrnetes 集群中对于 Pisa-Controller admission webhooks 的相关定义配置，还有 Pisa-Proxy 配置 CRD 的应用，本章节将介绍如何部署 Pisa-Controller  服务 

**此阶段所有资源对象都有 namespace 限制，如需更改 namespace ，请在 kubectl 命令后跟上 -n ${your namespace}**

#### 部署 rbac

Pisa-Controller 需要对pod 进行注入，所以需要申请对于pod资源的相关权限。Pisa-Controller 同时需要对 networking.pisanix.io 这个CRD进行操作，用来下发 Pisa-Proxy 的配置文件。

Yaml 中将部署下列资源

* ServiceAccount
* ClusterRole
* ClusterRoleBinding(如果修改了namespace,请修改subjects.namespace至对应的namespace)

```
kubectl apply -f rbac.yaml
```

#### 部署 Service

Service 是 Pisa-Controller admission webhooks  对外暴露服务的方式，Pisa-Controller 将暴露三个端口:80,6443,8080

* 80  端口自身健康检查端口
* 6443 端口 Pisa-Controller admission webhooks  与 kube-apiserver 通信端口
* 8080 端口 Pisa-Controller 与 Pisa-Proxy 通信端口 

Yaml 中将部署下列资源

* Service

```
kubectl apply -f service.yaml
```

#### 部署 Pisa-Controller

Pisa-Controller 使用 Deployment 形式进行部署，并且以 Service 方式进行暴露。

Yaml 中将部署下列资源

* Deployment

```
kubectl apply -f deployment.yaml
```

### 使用范例

Pisa-Controller admission webhooks 通过条件限定进行 Sidecar 注入

注入条件为，label 对象中包含 

```
pisanix.io/inject: enabled
```

现阶段条件为 

| namespace | pod     | 注入 |
| --------- | ------- | ---- |
| labeled   | labeled | yes  |
| labeled   | no      | no   |
| no        | no      | no   |
| no        | labeled | no   |



### 部署例子

下列资源对象将创建如下对象

* namespace injecttest
* deployment nginx-deployment
* ProxyConfig proxy-testimage

```
kubectl apply -f sample.yaml 
```

期望结果为 Pod 中包含 Pisa-Proxy Sidecar 镜像

## 配置教程

Pisa-Proxy 支持从配置文件和 remote API 获取配置。Pisa-Proxy 默认从 remote API 获取配置，若需要从本地文件加载配置需要导出 ```LOCAL_CONFIG=true``` 环境变量，并通过 ```-c，--config``` 参数指定配置文件路径。若不指定，默认从 ```./etc/config.toml``` 文件中进行加载。pisa-proxy 支持通过命令行参数和环境变量进行服务启动配置。配置详解如下：


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
1. PORT: api 启动端口号
2. CONFIG: 加载配置文件路径，同命令行参数的 ```-c, --config```参数
3. LOGLEVEL: 日志级别
4. LOCAL_CONFIG: 指定 Pisa-Proxy 从本地加载配置

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
addr = "127.0.0.1:3307"
# 负载均衡节点权重
weight = 1
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
addr = "127.0.0.1:3307"
weight = 1
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
addr = "127.0.0.1:3307"
weight = 1

[[mysql.node]]
name = "ds002"
db = "test"
user = "root"
password = "root"
addr = "127.0.0.1:3308"
weight = 2
```
