---
sidebar_position: 1
---

# 在 Kubernetes 中部署 

***Pisanix*** 的三个组件在项目规划之初即按照`控制面`配合多`数据面`的形式进行设计，它们都可部署在 kubernetes 之上，并通过 CustomResourceDefinition 完成各种配置，由 ***Pisa-Controller*** 转换为相应的配置后，下发给 ***Pisa-Proxy*** 或 ***Pisa-Daemon***。

 <img src="/img/kubernetes.png" width="600" length="600"/>

## 前期准备

- Helm v3.8.0+

- kubectl 1.19+

- kubernetes 1.19+

**⚠️⚠️⚠️⚠️：目前已在 kubernetes 1.19+ 和 1.20+ 版本集群上通过测试**

## 安装步骤

### 源码安装

通过 Helm 构建依赖并安装 ***Pisanix*** 到指定 namespace

```shell
cd charts/pisa-controller 
helm dependency build
cd ..
helm install pisa-controller pisa-controller -n <your namespace>
```

Helm 将安装如下资源

- apiservice: v1alpha1.admission.database-mesh.io
- deployment: pisa-controller
- service: pisa-controller
- clusterrole: pisa-controller
- clusterrolebinding: pisa-controller
- customresourcedefinitions: 
  - virtualdatabases.core.database-mesh.io
  - databaseendpoints.core.database-mesh.io
  - trafficstrategies.core.database-mesh.io

### 清理

测试完成后使用 Helm 命令卸载 ***Pisanix***

```
helm uninstall pisa-controller -n <your namespace>
```

## Helm values 配置项介绍


| Name                        | Description                                    | Value                  |
| --------------------------- | ---------------------------------------------- | ---------------------- |
| `replicaCount`              | Pisa-Controller 节点数                         | `1`                    |
| `image.repository`          | Pisa-Controller 镜像名                         | `pisanixio/controller` |
| `image.pullPolicy`          | Pisa-Controller 镜像拉取策略                   | `IfNotPresent`         |
| `image.tag`                 | Pisa-Controller 镜像标签                       | `v0.1.0`               |
| `imagePullSecrets`          | Docker 私有仓库的密钥，以数组形式注入          | `[]`                   |
| `proxyImage.repository`     | Pisa-Proxy 的镜像名                            | `pisanixio/proxy`      |
| `proxyImage.tag`            | Pisa-Proxy 镜像标签                            | `v0.1.0`               |
| `resources.limits`          | Pisa-Controller 资源限制数值                   | `{}`                   |
| `resources.requests.cpu`    | Pisa-Controller 资源申请 cpu 核数              | `100m`                 |
| `resources.requests.memory` | Pisa-Controller 资源申请内存数量               | `128Mi`                |
| `service.corePort`          | Pisa-Controller 核心暴露端口                   | `80`                   |
| `service.webhookPort`       | Pisa-Controller Webhook 暴露端口               | `6443`                 |
| `service.proxyConfigsPort`  | Pisa-Controller 和 Pisa-Proxy 配置文件通信端口 | `8080`                 |

## CustomResourceDefinition 字段说明

**⚠️⚠️⚠️⚠️： 注意: VirtualDatabase 中 db 字段需要和 DatabaseEndpoint 中 db 字段保持一致，客户端中连接指定的 schema 需要和 db 字段三者保持一致**

### VirtualDatabase

| Name                       | Description                                                |
| -------------------------- | ---------------------------------------------------------- |
| `services`                 | 一个 VirtualDatabase 的详细设置                            |
| `services.name`            | 虚拟数据库名字                                             |
| `services.trafficStrategy` | 虚拟数据库的相关策略 [TrafficStrategy](###TrafficStrategy) |
| `databaseMySQL`            | 一个虚拟的 MySQL 类型                                      |
| `host`                     | 虚拟 MySQL Host地址                                        |
| `port`                     | 虚拟 MySQL 端口                                            |
| `user`                     | 虚拟 MySQL 用户名                                          |
| `password`                 | 虚拟 MySQL 密码                                            |
| `db`                       | 虚拟 MySQL schema名字                                      |

### TrafficStrategy

| Name                                   | Description                                                  |
| -------------------------------------- | ------------------------------------------------------------ |
| `selector`                             | 标签选择器，用于选择 [DatabaseEndpoint](###DatabaseEndpoint) |
| `loadBalance`                          | 负载均衡器，现阶段支持：simpleLoadBalancer                   |
| `loadBalance.simpleLoadBalancer`       | 简单负载均衡声明                                             |
| `loadBalance.simpleLoadBalancer.kind`  | 负载均衡策略类型                                             |
| `circuitBreaks`                        | 断路器                                                       |
| `circuitBreaks.regex`                  | 断路正则规则                                                 |
| `concurrencyControls`                  | 并发控制器                                                   |
| `concurrencyControls.regex`            | 并发控制正则规则                                             |
| `concurrencyControls.duration`         | 并发控制时延                                                 |
| `concurrencyControls.maxConcurrency`   | 最大并发执行数量                                             |

### DatabaseEndpoint

| Name       | Description          |
| ---------- | -------------------- |
| `database` | 后端的数据源类型     |
| `MySQL`    | MySQL 类型的详细描述 |
| `host`     | MySQL Host 地址      |
| `port`     | MySQL 端口           |
| `user`     | MySQL 用户名         |
| `password` | MySQL 密码           |
| `db`       | MySQL schema 名字    |


## Demo 运行

**以下 Demo 均在 namespace:demotest 中运行，如果需要改变 namespace，请改变相应的 namespace**

### 示例业务部署

以 [Weaveworks Socks-shop(github.com)](https://github.com/microservices-demo/microservices-demo) 为运行应用 Demo，下列演示将展示怎么样在 microservices-demo 使用 ***Pisanix***

首先利用 Helm 部署 microservices-demo 

```shell
kubectl create ns demotest
cd deploy/kubernetes/helm-chart
helm dependency build
cd ..
helm install microservices-demo helm-chart -n demotest
```
检查 Helm 命令部署情况

![socks-shop-deployed](/img/socks-shop-deployed.png)

等待程序启动并看到如下页面后确认已正确运行

![socks-shop-preview](/img/socks-shop-preview.png)

### 使用 Pisanix 实现 Database Mesh 理念 

#### Pisanix 配置

编写如下三个 CustomResourceDefinitions：

- VirtualDatabase

```yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: VirtualDatabase
metadata:
  name: catalogue
  namespace: demotest
spec:
  services:
    - name: "mysql"
      databaseMySQL:
        host: "127.0.0.1"
        port: 3306
        user: "catalogue_user"
        password: "default_password"
        db: "socksdb"
      trafficStrategy: "catalogue"
```

- TrafficStrategy

```yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: TrafficStrategy
metadata:
  name: catalogue
  namespace: demotest
spec:
  selector:
    matchLabels:
      source: catalogue
  loadBalance:
    simpleLoadBalance:
      kind: "random"
```

- DatabaseEndpoint

```yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: DatabaseEndpoint
metadata:
  name: catalogue-db
  namespace: demotest
  labels:
    source: catalogue
spec:
  database:
    MySQL:
      host: "catalogue-db.demotest"
      port: 3306
      user: "root"
      password: "fake_password"
      db: "socksdb"
```


#### 应用访问数据库切换至 Pisanix

为 namespace 添加标签以开启注入

```shell
kubectl label ns demotest pisanix.io/inject=enabled
```

修改 catalogue-dep.yaml 来为 catalogue deployment 添加标签以开启注入，

```yaml
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: catalogue
  labels:
    name: catalogue
spec:
  replicas: 1
  selector:
    matchLabels:
      name: catalogue
  template:
    metadata:
      labels:
        name: catalogue
        pisanix.io/inject: enabled  # 通过该 Label 标识注入启用
      {{- if .Values.istio.enabled }}
      annotations:
        sidecar.istio.io/rewriteAppHTTPProbers: "true"
      {{- end }}
    spec:
      containers:
      - name: catalogue
        image: {{if .Values.global.registry}}{{ .Values.global.registry }}/{{end}}{{ .Values.catalogue.image.repo }}:{{ .Values.catalogue.image.tag }}
        command: ["/app"]
        args:
        - -port={{ .Values.catalogue.containerPort }}
        - -DSN=catalogue_user:default_password@tcp(127.0.0.1:3306)/socksdb # DSN 中用户名、密码、主机名和端口都参照 VirtualDatabase 里的信息进行修改
        {{- if .Values.zipkin.enabled }}
        env:
         - name: ZIPKIN
           value: http://{{ .Values.zipkin.url }}:9411/api/v1/spans
        {{- end }}
        resources:
{{ toYaml .Values.catalogue.resources | indent 10 }}
        ports:
        - containerPort: {{ .Values.catalogue.containerPort }}
        securityContext:
          runAsNonRoot: true
          runAsUser: 10001
{{- if lt (int .Values.carts.containerPort) 1024 }}
          capabilities:
            drop:
              - all
            add:
              - NET_BIND_SERVICE
{{- end }}
          readOnlyRootFilesystem: true
        livenessProbe:
          httpGet:
            path: /health
            port: {{ .Values.catalogue.containerPort }}
          initialDelaySeconds: 300
          periodSeconds: 3
        readinessProbe:
          httpGet:
            path: /health
            port: {{ .Values.catalogue.containerPort }}
          initialDelaySeconds: 180
          periodSeconds: 3
```

通过 Helm 升级应用
```shell
helm upgrade  microservices-demo  helm-chart -n demotest
```

等待程序正常启动，并测试访问即可验证
