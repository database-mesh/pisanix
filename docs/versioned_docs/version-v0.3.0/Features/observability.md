---
sidebar_position: 5
---

# 可观测性 

Pisanix 目前在 Pisa-Proxy 处理 SQL 命令的时候采集相关指标，并以 `/metrics` 路径进行暴露。

## 已支持指标
- SQL_PROCESSED_TOTAL: 统计所有执行的 SQL 数量
- SQL_PROCESSED_DURATION: 统计所有 SQL 的执行时间
- SQL_UNDER_PROCESSING: 记录当前正在执行的 SQL 数量

测试效果如下图：

![grafana](/img/grafana.jpg)

下一步将支持更多标签和指标，如 SQL 语句类型、延迟、错误率、TopK、运行时资源等。
