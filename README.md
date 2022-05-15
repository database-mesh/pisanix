# Introduction

`Pisanix` [Pi-sanics] is a modern database governance framework for Kubernetes. Pisanix adds SQL-aware traffic control, audit, security and extension abilities to help manage various databases in the [Database Mesh](https://www.database-mesh.io) way.

# Highlights

`Pisanix` has the following goals:

1. SQL-Aware Traffic Control: supports SQL traffic load balancing, access control, observability.
2. Runtime Resource-oriented Programming: supports extensible resource control abilities.
3. Database Reliability Engineering: make DBA's life easier with Kubernetes

 <img src="static/pisanix-arch.png" width="600" length="600"/>

Pisanix has 3 componenets:

* ***Pisa-Controller***: A Golang control plane designed for sidecar injection and configuration transformation
* ***Pisa-Proxy***: A high performance Rust data plane used as SQL traffic proxy, support various of traffic governance capabilities.
* ***Pisa-Daemon***: A Golang optional data plane works on every node, provide programmable runtime management such as TrafficQoS.

# Features
## Database traffic governance

Applications access databases with SQL, so Pisanix will hijack all SQL traffic. This is a great opportunity to do a lot of things around traffic, like loadbalancing and SQL firewall.

## Observability

In the past, metrics could be retrieved from database instances and display in kinds of charts. Now with Pisanix, DBAs could have more chances to achieve better observability.

## Progammable 

For DBAs who could and would like to solve problems with programming. Pisanix supports many kinds of plugin mechanism, like Lua and Wasm. People will have the chance to 'reshape' the expected behavior of databases.

# Getting Started
- [Introduction](https://www.pisanix.io/docs/intro)
- [Quick Start](https://www.pisanix.io/docs/quickstart)

# Documentation
Full documentation will be available on the [Pisanix website](https://pisanix.io/).

# Community & Support
 :link: [GitHub Issues](https://github.com/database-mesh/pisanix/issues). Best for: larger systemic questions/bug reports or anything development related.

 :link: [Slack channel](https://join.slack.com/t/databasemesh/shared_invite/zt-177m5biwz-VKe3eXPSjarlrgYDPdQuhw). Best for: instant communications and online meetings, sharing your applications.

 :link: [Tecent Meeting / Voov: 553-8242-5155](https://meeting.tencent.com/dm/6UXDMNsHBVQO). Best for: Bi-weekly meeting, started from Feb 16th 2022.


- Wechat User Group: Broker wechat to add you into the user group.
 <img src="static/wechat-user-group-broker.jpeg" width="200" length="200"/>
