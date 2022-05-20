// Copyright 2022 SphereEx Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package cri

import "context"

const (
	ContainerRuntimeInterfaceDocker = "docker"
	dockerSocket                    = "unix://var/run/docker.sock"
)

// ContainerRuntimeInterfaceClient contains functions for container info
type ContainerRuntimeInterfaceClient interface {
	GetPidFromContainer(ctx context.Context, containerId string) (uint32, error)
}

// NewContainerRuntimeInterfaceClient return a new ContainerRuntimeInterfaceClient based different CRI
func NewContainerRuntimeInterfaceClient(cri string) (ContainerRuntimeInterfaceClient, error) {
	var (
		client ContainerRuntimeInterfaceClient
		err    error
	)
	switch cri {
	case ContainerRuntimeInterfaceDocker:
		if client, err = NewDockerClient(dockerSocket, "", nil, nil); err != nil {
			return nil, err
		}
	}

	return client, nil
}
