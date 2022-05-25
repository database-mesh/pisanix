---
sidebar_position: 2
---

# Pisa-Proxy 单独部署 

***Pisa-Proxy*** 作为高性能代理不仅可以在 Kubernetes 中以 Sidecar 的方式部署，也可以作为统一接入层单独部署在 Kubernetes 之外的服务器上：

![single.png](/img/single.png)

目前***Pisa-Proxy***支持 MySQL 协议，无论后端是云上的 RDS 实例，还是自建的 MySQL、ShardingSphere、TiDB 等，都可以由 Pisa-Proxy 统一流量分发，实现无感的高可用切换、面向 SQL 的可观测性等。
