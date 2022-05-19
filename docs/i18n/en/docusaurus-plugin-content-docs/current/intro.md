---
sidebar_position: 1
---

# Introduction

`Pisanix` [Pi-sanics] is a modern database governance framework for Kubernetes. Pisanix adds SQL-aware traffic control, audit, security and extension abilities to help manage various databases in the [Database Mesh](https://www.database-mesh.io) way.


![Pisanix Arch](/img/pisanix-arch.png)


# Highlights

Pisanix has the following goals:

* SQL-Aware Traffic Control: supports SQL traffic load balancing, access control, observability.
* Runtime Resource-oriented Programming: supports extensible resource control abilities.
* Database Reliability Engineering: make DBA's life easier with Kubernetes

# Features
## Database traffic governance

Applications access databases with SQL, so Pisanix will hijack all SQL traffic. This is a great opportunity to do a lot of things around traffic, like loadbalancing and SQL firewall.

## Observability

In the past, metrics could be retrieved from database instances and display in kinds of charts. Now with Pisanix, DBAs could have more chances to achieve better observability.

## Progammable 

For DBAs who could and would like to solve problems with programming. Pisanix supports many kinds of plugin mechanism, like Lua and Wasm. People will have the chance to 'reshape' the expected behavior of databases.

# Getting Started
- [Introduction](https://www.pisanix.io/docs)
- [Quick Start](https://www.pisanix.io/docs/quickstart)

# Documentation
Full documentation will be available on the [Pisanix website](https://www.pisanix.io/).

# Community & Support
 :link: [GitHub Issues](https://github.com/database-mesh/pisanix/issues). Best for: larger systemic questions/bug reports or anything development related.

 :link: [Slack channel](https://join.slack.com/t/databasemesh/shared_invite/zt-12hlythpe-C4rrS1WZ2ZkEd3zn84SqeQ). Best for: instant communications and online meetings, sharing your applications.

- Wechat User Group: Broker wechat to add you into the user group.

![Wechat user group broker](/img/wechat-user-group-broker.jpeg)
