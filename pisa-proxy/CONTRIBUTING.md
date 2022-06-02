# Contributing to Pisa-Proxy

## Prerequisites

Pisa-proxy is written in Rust, to build Pisa-Proxy from scratch you will need to install the following tools:
- Install Rust 1.6.0 with [rustup](https://rustup.rs/)
- Install [clippy](https://github.com/rust-lang/rust-clippy) with `rustup`
- Install [fmt](https://github.com/rust-lang/rustfmt) 

## Fork and Clone

```
git clone https://github.com/database-mesh/pisanix.git
cd pisanix/pisa-proxy
```

## run clippy
```
make clippy
```

## run format
```
make fmt
```

## build with release

```
# make release
```

`proxy` will be placed in the `target/release` directory.

## Build with Docker Image

We provide a docker image with full development requirements.
```
# make docker
```

## How to add a new feature or change an existing one

Before making any significant changes, please [open an issue](https://github.com/database-mesh/pisanix/issues). Discussing your proposed changes ahead of time will make the contribution process smooth for everyone.

Once we've discussed your changes and you've got your code ready, make sure that tests are passing and open your pull request. Your PR is most likely to be accepted if it:

* Includes tests for new functionality.
* References the original issue in the description, e.g. "Resolves #123".
* Has a [good commit message](https://github.com/database-mesh/pisanix/blob/futures-0.1.0-docs/CONTRIBUTING.md).

## rust code style
* https://rust-coding-guidelines.github.io/rust-coding-guidelines-zh/overview.html