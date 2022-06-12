---
slug: greetings
title: Hello! Pisanix!
authors: [Maxwell, Jiandong, Bo, Zhuo]
tags: [pisanix, v0.1.0]
---

June 6th, 2022, Pisanix announced its first release v0.1.0.

As the first open source project of [Database Mesh 2.0](https://www.database-mesh.io), Pisanix follows the core concepts that `building an efficient and programmable database governance experience in cloud native era` from the very beginning. This means Pisanix really hope to help Database Mesh grow to an achievable framework. 

## Who is Pisanix ?

Pisanix is the open source solution for Database Mesh, which composes of three different components: `Pisa-Controller`, `Pisa-Proxy` and `Pisa-Daemon`(***Coming Soon***), and is sponsored by [SphereEx](https://www.sphere-ex.com).

Written with Golang and Rustlang, Pisanix is going to build an experience that support SQL aware traffic management, runtime oriented resource programming and Database Reliability Engineering.

Like classical Service Mesh deployment pattern, Pisanix also contains a `control-plane` which is `Pisa-Controller`, and two `data-plane` which is `Pisa-Proxy` and `Pisa-Daemon`. 

 <img src="/img/pisanix-arch-blog.png" width="600" length="600"/>

### Pisa-Controller

Act as the required part of Pisanix, Pisa-Controller is responsible for:

* Sidecar injection: Using MutatingAdmissionWebhook inject sidecar to selected Pods
* Pisa-Proxy configuration conversion: Retrieve service discovery, load balance, concurrency control, SQL circuit break configs in CRDs and convert them to Pisa-Proxy configurations
* Pisa-Daemon configuration conversion: Retrieve traffic QoS configs in CRDs and convert them to Pisa-Daemon configurations

### Pisa-Proxy

Pisa-Proxy is the core part of Pisanix, working as a high-performance proxy for different protocols. Currently supports:
* SQL traffic control: Traffic load balance, concurrency control and
* Access Control: Fine-grained access control rule according to user and data relationship
* Circuit break: Reject high risk SQL execution
* Observability: Metrics about SQL processing: latency、throughput

### Pisa-Daemon

Pisa-Daemon is optional and will be release soon. Features like runtime resource management, such as providing traffic QoS with the help of eBPF is what Pisa-Daemon care about. 

## What does Maintainers say ?

[Maxwell](https://github.com/mlycore): "Thanks to the cloud native ecosystem and all of the open source projects. We have learned a lot from the community, and now we get the chance to do something, that is Pisanix. The core motivation of Pisanix is to help, help developers, SREs and DBAs, and also help databases. This makes Pisanix a very meaningful infrastructure software. Thanks for your attention and support."

[Jiandong](https://github.com/xuanyuan300): "Pisanix is a concrete practice of the Database Mesh idea. It is a project with great potential. As a nascent community, I hope everyone can participate and witness our growth together."

[Bo](https://github.com/wbtlb): "Pisanix is a set of governance frameworks in the database DRE project. This framework is all-encompassing. It not only provides users in different scenarios such as developer-facing and DBA, but also provides rich thinking in the field of software design for colleagues in different area. Friends are very welcomed to join the community and build it together."

[Zhuo](https://github.com/windghoul): "As an emerging project that implements DRE ideas and realizes database-mesh design. Pisanix provides developers, DBAs, and SREs with unlimited possibilities. There will be more designs and inspirations presented to you in the future. I wish everyone will continue to pay attention, build together, and complete this idea."

## What's new in v0.1.0
This is the first release of `Pisanix`.

### Features

#### Pisa-Controller
##### Setup
* Label-based sidecar injection
##### Configuration conversion 
* Kubernetes CRD conversion to Pisa-Proxy configurations, including `VirtualDatabase`, `TrafficStrategy`, `DatabaseEndpoint` 

#### Pisa-Proxy
##### Setup
* Retrieve configuration from Pisa-Controller

##### Runtime
* Support multiple proxy runtimes speaking MySQL protocol
* Support connection pool and basic loadbalancing strategy
* Support `Yacc` based SQL parser: SELECT, INSERT, UPDATE, DELETE, PREPARE, EXECUTE, BEGIN and SET.
* Support `Regex` based SQL circuit break
* Support `Regex` based SQL concurrency control

##### Observability
* Supoort basic observability
  * `sql_processed_total`
  * `sql_processed_duration`


## What can I do with Pisanix ?

Database Mesh designs with these building blocks and terminology below:
* Virtual Database: A database endpoint could be consumed by application
* Traffic Strategy: Various strategy for database traffic, such as load balancing, sharding, rate limit and circuit breaker
* Access Control: Providing fine-grained admission mechanism
* Security Claim: Claim for security enhance mechanism, such as encryption
* Audit Request: Request for user operation audit
* Observability: Provide a config for observability of databases
* Event Bus: Sink database change events to external systems
* QoS Claim: Porviding several object guarantee for databases
* Backup Job: Database backup jobs
* Schema Pipeline: Using pipeline for versioned schema changing

Pisanix introduced CustomResourceDefinition including `VirtualDatabase`, `Traffic Strategy` and `DatabaseEndpoint` as workloads. And this is a example of Weaveworks [Socks-shop](https://github.com/microservices-demo/microservices-demo). 

### VirtualDatabase
VirtualDatabase is the root concept for every database governance actions in Pisanix. To developers, VirtualDatabase is represented as a database endpoint. For DBAs, VirtualDatabase is some kind of a logical database, so they need to provide some traffic strategy and bind it to a real backend database endpoint.

```yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: VirtualDatabase
metadata:
  name: catalogue
  namespace: default
spec:
  services:
  - databaseMySQL:               # Declare a MySQL database 
      db: socksdb                # Declare the schema 
      host: 127.0.0.1            
      port: 3306                 
      user: catalogue_user       
      password: default_password 
    name: mysql                  
    trafficStrategy: catalogue   # The target traffic strategy for the database 
```


### TrafficStrategy

TrafficStrategy defines how the SQL requests will be routed to the real database endpoint, strategies like loadbalance, concurrency control, circuit break are supported now.

```yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: TrafficStrategy
metadata:
  name: catalogue
  namespace: default
spec:
  loadBalance:                   
    simpleLoadBalance:           # A simple load balance strategy
      kind: random               # support random as selection algorithm 
  selector:
    matchLabels:                 # Declare the label selector to choose the backend database
      source: catalogue
```

### DatabaseEndpoint

Database Endpoint refers to a real database endpoint, no matter it's endpoint of AWS RDS, a MySQL instance, or ShardingSphere. Virtual Database consumes several DatabaseEndpoint with TrafficStrategy.

```yaml
apiVersion: core.database-mesh.io/v1alpha1
kind: DatabaseEndpoint
metadata:
  name: catalogue-db
  namespace: default
spec:
  database:
    MySQL:                        # Declare the database type MySQL 
      db: socksdb                 
      host: cataloguedb.codtynlacssn.rds.cn-northwest-1.amazonaws.com.cn
      port: 3306                  
      user: root                  
      password: fake_password    
```

Now the working flow is like :
1. Developers submit their need of database as VirtualDatabase
2. DBAs create and bind TrafficStrategy with DatabaseEndpoint
3. SREs add labels `pisanix.io/inject=enabled` to the application and update the configuration with credentials stored in VirtualDatabase.

After the application is running, we can checkout the Socks-shop website as below:

![](/img/socks-shop.png)

## How about the next steps ?
As we can see that Pisanix is very young, and have a definitely long way to run. In the next, we will first enhance the ability of traffic governance, like adding data sharding, data access behavior auditing, runtime resource Qos, etc. And we also continuously improving the performance and operability of Pisanix. More extensions mechanism like plugins will be in the future, help users to build their own solution very easily.

## Community Call

Building an open source community needs help from everywhere, no matter it's code or documentation, issues or pull requests, community thanks all of your efforts.

At present, there are some ways to join the community:

| | |
|:-|:-|
| Mailing List| https://groups.google.com/g/database-mesh |
| Dev Meetings (Starting Feb 27th, 2022), Bi-weekly Wednesday 9:00AM PST|https://meet.google.com/yhv-zrby-pyt |
| Dev Meetings APAC Friendly (Starting April 27th, 2022), Bi-weekly APAC Wednesday 9:00PM GMT+8|https://meeting.tencent.com/dm/6UXDMNsHBVQO |
| Wechat Broker|pisanix|
| Slack |https://join.slack.com/t/databasemesh/shared_invite/zt-19rhvnxkz-USjZ~am~ghd_Q0q~8bAJXA  |
| Meetings Notes |https://bit.ly/39Fqt3x |

Feel free to talk !