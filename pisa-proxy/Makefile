.PHONY: ci clean build help check-fmt docker

BINARY_NAME := "proxy"
GIT_ERRMSG := "Unknown"
RUST_ERRMSG := "Unknown"

export RUST_VERSION := $(shell rustc --version 2> /dev/null || echo ${RUST_ERRMGS})
export GIT_COMMIT ?= $(shell git rev-parse --short HEAD 2> /dev/null | cut -c1-7 || echo ${GIT_ERRMSG})
export GIT_BRANCH ?= $(shell git rev-parse --abbrev-ref HEAD 2> /dev/null || echo ${GIT_ERRMSG})
export CARGO_NET_GIT_FETCH_WITH_CLI=true

REPO_NAME ?= pisanixio
IMAGE_NAME ?= proxy
IMAGE_TAG ?= $(GIT_BRANCH)-$(GIT_COMMIT)

ci: check-fmt

fmt:
	cargo +nightly fmt

clippy:
	cargo clippy

check-fmt:
	cargo +nightly fmt --all -- --check

clean:
	cargo clean

build:
	@mkdir -p ../bin && cargo build && /bin/cp -af target/debug/$(BINARY_NAME) ../bin/$(BINARY_NAME)
	@echo -e "\033[33mbuild output to ./bin directory\033[0"

release:
	cargo build --release

backtrace:
	RUST_BACKTRACE=1 target/debug/$(BINARY_NAME) -c etc/config.toml


# Cargo build release mode or debug mode
DOCKER_BUILD_TARGET ?= release

docker:
	@/bin/cp -af target/$(DOCKER_BUILD_TARGET)/$(BINARY_NAME) hack/$(BINARY_NAME)
	IMAGE_NAME=proxy-$(DOCKER_BUILD_TARGET)
	sudo docker build -t $(REPO_NAME)/$(IMAGE_NAME):$(IMAGE_TAG) -f hack/Local.Dockerfile hack

docker-push:
	IMAGE_NAME=proxy-$(DOCKER_BUILD_TARGET)
	sudo docker push $(REPO_NAME)/$(IMAGE_NAME):$(IMAGE_TAG)

