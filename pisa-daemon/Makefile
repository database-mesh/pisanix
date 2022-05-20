LDFLAGS := "-s -w -X 'github.com/database-mesh/pisanix/pisa-daemon/pkg/vesrion.GitCommit=`git log | grep commit | head -1 | cut -d" " -f2 | cut -c1-8`' -X 'github.com/database-mesh/pisanix/pisa-daemon/pkg/version.BuildGoVersion=`go version | cut -d" " -f3`'"
.PHONY: build
build:
	mkdir -p bin
	go build -ldflags=${LDFLAGS} -o bin/pisa-daemon cmd/pisa-daemon/main.go
linux:
	mkdir -p bin
	GOOS=linux GOARCH=amd64 go build -ldflags=${LDFLAGS} -o bin/pisa-daemon cmd/pisa-daemon/main.go


