---
sidebar_position: 1
---

# 在 Kubernetes 中部署 

Pisanix 的三个组件在项目规划之初即按照`控制面`配合多`数据面`的形式进行设计的，它们都可部署在 Kubernetes 之上，并通过 CustomResourceDefinition 完成各种配置，由 Pisa-Controller 转换为相应的配置后，下发给 Pisa-Proxy 或 Pisa-Daemon。

![kubernetes.png](/img/kubernetes.png)
