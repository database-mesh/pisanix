---
sidebar_position: 8
---

# Pisa-Daemon

Pisa-Daemon 是用 rust 实现的数据面，主要通过主机内核的各种能力来实现资源管理。
目前的主要功能是：
* 运行时流量管理
借助 eBPF 技术为数据库访问流量提供 QoS 保证，以减少流量之间的互相干扰。

Pisa-Daemon 即可以部署在 kubernetes 中以 daemonset 的方式部署，也可以单独部署。

# 现状
当前 Pisa-Damon 实现了 Service 级别的 QoS，Service 来自 VirtualDatabase 中的定义。

主要实现技术是 ebpf + tc。

# 配置

* 在 kubernetes 中部署，需要依赖 `QoSClaim` CRD， Spec 配置项如下：

| 属性 | 值类型 | 是否依赖 | 默认值 | 含义 |
|-----|-------|---------|-------|-----|
|trafficQoS | [object](#trafficQoS-配置) |  是   |  无   | traffic qos |


## trafficQoS 配置
| 属性 | 值类型 | 是否依赖 | 默认值 | 含义 |
|-----|-------|---------|-------|-----|
|name | string|  是   |  无   | qos 名称 |
|qos_group| [object](#qos_group-配置)| 否 | 无 | 具体带宽限制 |

### qos_group 配置
| 属性 | 值类型 | 是否依赖 | 默认值 | 含义 |
|-----|-------|---------|-------|-----|
|rate | string|  否   |  无   | 带宽最大值|
|ceil | string| 否 | 无 | 如果带宽有剩余，带宽可以达到的最大值 |

rate 和 ceil 的单位可以是

bit, kbit, mbit, gbit，tbit 表示 bit/s。

bps，kbps, mbps, gbps, tbps 表示 bps/s。


注意点：

  * 配置了 QoSClaim 后，需要在 VirtualDatabase 中配置 `qosClaim` 以生效。
  * trafficQos 的 name 名称要包含在 VirtualDatabase 的 Serivice 中。

示例：
``` yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: QoSCliam
metadata:
  name: test 
  namespace: default
spec:
  trafficQoS:
    name: svc1
    qos_group:
      rate: 1mps
      ceil: 2mps
```

* 单独部署配置如下：
  
``` toml
[global]
egress_device = "cali16adef18cfd"
bridge_device = "tunl0"

[[app]]
name = "test1"

[[app.service]]
name = "test"
[[app.service.endpoints]]
ip = "192.168.136.29"
port = 5201

[app.service.qos_group]
rate = "5mbps"
ceil = "5mbps"
```
