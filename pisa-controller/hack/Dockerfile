# Copyright 2022 SphereEx Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

FROM golang:1.17 as builder

WORKDIR /workspace

# Copy the Go Modules manifests
COPY pisa-controller/go.mod go.mod
COPY pisa-controller/go.sum go.sum
# cache deps before building and copying source so that we don't need to re-download as much
# and so that source changes don't invalidate our downloaded layer
ENV GOPROXY=https://proxy.golang.com.cn
RUN go mod download

# Copy the go source
COPY pisa-controller/cmd cmd/

COPY pisa-controller/pkg pkg/

# Build
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build  -ldflags "-w -s" -gcflags "-N -l" -a -o controller cmd/pisa-controller/main.go


FROM --platform=linux/amd64 alpine:3.15.4
WORKDIR /
COPY --from=builder /workspace/controller .
USER root
COPY pisa-controller/build/certs /etc/webhook/certs

ENTRYPOINT ["/controller"]