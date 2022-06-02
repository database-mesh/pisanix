.PHONY:  build fmt docker

GIT_ERRMSG := "Unknown"

export GIT_COMMIT ?= $(shell git rev-parse --short HEAD 2> /dev/null || echo ${GIT_ERRMSG})
export GIT_BRANCH ?= $(shell git rev-parse --abbrev-ref HEAD 2> /dev/null || echo ${GIT_ERRMSG})
export GIT_TAG ?= $(shell git describe --abbrev=0 --tags 2>/dev/null  || echo '' )

REPO_NAME ?= "pisanixio"
IMAGE_NAME ?= "controller"
IMAGE_TAG ?= $(GIT_BRANCH)-$(GIT_COMMIT)

default: build

fmt:
	go fmt ./...

build:
	@CGO_ENABLED=0 GOOS=linux go build \
		-ldflags "-w -s -X main.branch=$(GIT_BRANCH) -X main.gitcommit=$(GIT_COMMIT) -X main.version=$(GIT_TAG)"  \
		-gcflags "-N -l" \
		-o ../bin/controller cmd/pisa-controller/main.go
	@echo -e "\033[33mbuild output to ./bin directory\033[0"

docker:
	ROOT_DIR=$(realpath ..)
	#@cd $(ROOT_DIR)
	sudo docker build --build-arg GIT_BRANCH=$(GIT_BRANCH) --build-arg GIT_COMMIT=$(GIT_COMMIT) --build-arg GIT_TAG=$(GIT_TAG) \
		-f hack/Dockerfile -t $(REPO_NAME)/$(IMAGE_NAME):$(IMAGE_TAG) $(realpath ..)
