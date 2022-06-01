.PHONY:  build fmt docker

GIT_ERRMSG := "Unknown"

export GIT_COMMIT ?= $(shell git rev-parse --short HEAD 2> /dev/null | cut -c1-7 || echo ${GIT_ERRMSG})
export GIT_BRANCH ?= $(shell git rev-parse --abbrev-ref HEAD 2> /dev/null || echo ${GIT_ERRMSG})

IMAGE_NAME ?= "controller"
IMAGE_TAG ?= $(GIT_BRANCH)-$(GIT_COMMIT)

default: build

fmt:
	go fmt ./...

build:
	CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -ldflags "-w -s" -gcflags "-N -l"  -o ../bin/controller cmd/pisa-controller/main.go
	@echo -e "\033[33mbuild output to ./bin directory\033[0"

docker:
	sudo docker build -f hack/Dockerfile -t $(REPO_NAME)/$(IMAGE_NAME):$(IMAGE_TAG) .
