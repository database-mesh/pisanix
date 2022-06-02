# Contributing to Pisa-Proxy

Thanks for your interest in contributing to Pisa-Proxy! This document outlines some of the conventions on building, running, and testing Pisa-Proxy, the development workflow, commit message formatting, contact points and other resources.

If you need any help or mentoring getting started, understanding the codebase, or making a PR (or anything else really), please ask on [Slack](https://databasemesh.slack.com/). If you don't know where to start, please click on the contributor icon below to get you on the right contributing path.

# Building From Source

## Install prerequisites

Pisa-proxy is written in Rust, to build Pisa-Proxy from scratch you will need to install the following tools:
- Git
- Rust Install with [rustup](https://rustup.rs/)
- Install Clippy with `rustup`

## Get the Pisa-Proxy code

```
git clone https://github.com/database-mesh/pisanix.git
cd pisanix/pisa-proxy
```

## Run make

```
# make release
```

`proxy` will be placed in the `target/release` directory.

## Build with Docker Image

We provide a docker image with full development requirements.
```
# make docker
```

## rust code style
* https://rust-coding-guidelines.github.io/rust-coding-guidelines-zh/overview.html