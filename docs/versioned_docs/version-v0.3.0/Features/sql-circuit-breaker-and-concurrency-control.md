---
sidebar_position: 4
---

# 断路和并发控制 

目前实现了两个默认插件 `SQL 断路` 和 `SQL 并发控制`。

### SQL 断路

禁止运行匹配正则的语句。

#### 示例配置
如：不允许执行` SELECT *`

``` toml
[[proxy.config.plugin.circuit_break]]
regex = ["SELECT \\*"]
```
> 注：可以有多个规则

### SQL 并发控制

SQL 并发控制规则表示在 `duration` 秒内并发运行匹配正则的 SQL 语句只能有 `max_concurrency` 条，

#### 配置示例
如：不允许100秒内并发执行多于1条 `SELECT *`

``` toml
[[proxy.config.plugin.concurrency_control]]
regex = ["SELECT \\*"]    
max_concurrency = 1
duration = 100
```

> 注：可以有多个规则

#### 使用场景
SQL 并发控制通常使用在慢查询或者 OLAP 查询中，即一个 SQL 查询需要耗费大量时间的场景下。例如, 这里使用两个客户端同时执行以下 SQL 语句, 可以看出，当第一条 SQL 语句被阻塞后，在 `duration = 100` 周期内，另外一条 SQL 语句已经被插件限制执行 `max_concurrency = 1` 次。


```
# client 1

MySQL [(none)]> SELECT * FROM (SELECT SLEEP(5)) AS sock;
```

```
# client 2

MySQL [test]> SELECT * FROM sbtest1 LIMIT 1;
ERROR 1047 (08S01):  concurrency control plugin rejected
```

## 设计说明

目前运行中间件的方式参考了[ Tower-rs ](https://github.com/tower-rs/tower.git)，可以很好的满足未来扩展的需求。

其中有两个概念:
> Layer:  是对 `Service` 的包装，每个 `Service` 可以有多个不同的中间件。

> Service: 指 `Pisanix` 内部允许执行中间件的服务或者是某个功能函数，可以运行一些自定义的逻辑，如 `Metrics` 收集。

实现[伪代码](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=0db8ca6f72096c7a74682085a66e3270)。

注意 

- 并发控制规则表示在 `duration` 秒内并发运行匹配正则的 SQL 语句只能有 `max_concurrency` 条，从第一次匹配到开始计时，如果超过 `duration` 时间，则并发控制失效，重新开始下一次匹配。
- 目前由于不能动态生效并发控制规则，因此规则的生效时间是从第一次匹配到的时间开始，持续 `duration` 时间，超过后继续下次循环，没有失效情况，在未来支持动态生效后，规则失效有两种情况：
    -  `duration` 的时间将从获取到开启动作的时间开始，持续 `duration` 时间，规则失效。
    -  等到获取到关闭动作时，规则失效。
