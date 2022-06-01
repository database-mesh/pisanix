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

FROM rust:1.60.0-slim-buster as builder
ARG GIT_BRANCH
ARG GIT_COMMIT
ARG GIT_TAG
COPY pisa-proxy/hack/config ~/.cargo/config
RUN apt-get update && apt-get  -y  install gcc openssl libssl-dev pkg-config
WORKDIR /workspace
COPY pisa-proxy /workspace
RUN cargo build --release


FROM pisanixio/alpine-glibc
COPY --from=builder /workspace/target/release/proxy /pisanix/
WORKDIR /pisanix
ENTRYPOINT ["./proxy"]
