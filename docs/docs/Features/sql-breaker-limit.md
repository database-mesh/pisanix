---
sidebar_position: 4
---

## 中间件

目前运行中间件的方式参考了[ TOWER-RS ](https://github.com/tower-rs/tower.git)，可以很好的满足未来扩展的需求。

其中有两个概念:
> Layer:  是对 `Service` 的包装，每个 `Service` 可以有多个不同的中间件。

> Service: 指 `PISANIX` 内部允许执行中间件的服务或者是某个功能函数，可以运行一些自定义的逻辑，如 `METRICS` 收集。

实现[伪代码](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=0db8ca6f72096c7a74682085a66e3270)。

## 实现
目前实现了两个默认插件 `SQL断路器` 和 `SQL限流`。

### SQL断路器
禁止运行匹配正则的语句。

#### 示例配置
``` toml
[[proxy.configs.plugin.audit]]
regex = "\\w+"
```

> 可以有多个规则

### SQL限流
限流规则表示在duration秒内并发运行匹配正则的sql语句只能有 `limit` 条，

#### 示例配置
``` toml
[[proxy.configs.plugin.limit]]
regex = "\\w+"    
limit = 1
duration = "100"
```

> 可以有多个规则

#### 说明

1. 限流规则表示在 `duration` 秒内并发运行匹配正则的 sql 语句只能有 `limit` 条，从第一次匹配到开始计时，如果超过 `duration` 时间，则限流失效，重新开始下一次匹配。
2. 目前由于不能动态生效限流规则，因此规则的生效时间是从第一次匹配到的时间开始，持续 `duration` 时间，超过后继续下次循环，没有失效情况，在未来支持动态生效后，规则失效有两种情况：
    -  `duration` 的时间将从获取到开启动作的时间开始，持续 `duration` 时间，规则失效。

    -  等到获取到关闭动作时，规则失效。
