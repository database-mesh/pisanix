# Contributing to Pisa-Controller

## Prerequisites
* Install Git
* Install [Go 1.16](https://golang.org/dl/) or later, and we use go module to manage the go package dependencies.
* Prepare an available Kubernetes cluster in your workstation, we recommend you to use [KIND](https://kind.sigs.k8s.io/).

## Fork and Clone
* Fork the repository from [database-mesh/pisanix](https://github.com/database-mesh/pisanix) to your own GitHub account.
* Clone the fork repository to your workstation.
* Run `go mod download` to download modules to local cache. By the way, if you are a developer in China, we suggest you setting `GOPROXY` to `https://goproxy.cn` to speed up the downloading.


## Build
```
# make build
```


## How to add a new feature or change an existing one

Before making any significant changes, please [open an issue](https://github.com/database-mesh/pisanix/issues). Discussing your proposed changes ahead of time will make the contribution process smooth for everyone.

Once we've discussed your changes and you've got your code ready, make sure that tests are passing and open your pull request. Your PR is most likely to be accepted if it:

* References the original issue in the description, e.g. "Resolves #123".
* Has a [good commit message](https://github.com/database-mesh/pisanix/blob/master/CONTRIBUTING.md).
