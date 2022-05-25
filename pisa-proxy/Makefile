.PHONY: ci clean build check-fmt

ci: check-fmt

fmt:
	cargo +nightly fmt

clippy:
	cargo clippy

check-fmt:
	cargo fmt --all -- --check
clean:
	cargo clean
build:
	cargo build
release:
	cargo build --release
backtrace:
	RUST_BACKTRACE=1 target/debug/pisa-proxy -c etc/config.toml

GIT_ERRMSG := "Unknown"
RUST_ERRMSG := "Unknown"

export RUST_VERSION := $(shell rustc --version 2> /dev/null || echo ${RUST_ERRMGS})
export GIT_COMMIT ?= $(shell git rev-parse HEAD 2> /dev/null || echo ${GIT_ERRMSG})
export GIT_BRANCH ?= $(shell git rev-parse --abbrev-ref HEAD 2> /dev/null || echo ${GIT_ERRMSG})
export CARGO_NET_GIT_FETCH_WITH_CLI=true
