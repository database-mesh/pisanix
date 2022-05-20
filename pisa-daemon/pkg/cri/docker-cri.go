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

import (
	"context"
	"errors"
	"fmt"
	"net/http"

	dockertypes "github.com/docker/docker/api/types"
	dockerclient "github.com/docker/docker/client"
)

var _ ContainerRuntimeInterfaceClient = &DockerClient{}

type DockerOperations interface {
	ContainerInspect(ctx context.Context, containerId string) (dockertypes.ContainerJSON, error)
}

type DockerClient struct {
	Client DockerOperations
}

// GetPidFromContainer will get Pid from containerId
func (c DockerClient) GetPidFromContainer(ctx context.Context, containerId string) (uint32, error) {
	con, err := c.Client.ContainerInspect(ctx, containerId)
	if err != nil {
		return 0, err
	}

	if con.State.Pid == 0 || con.State.Dead {
		return 0, errors.New(fmt.Sprintf("unexpected container status %s", con.State.Status))
	}

	return uint32(con.State.Pid), nil
}

func NewDockerClient(host, version string, http *http.Client, headers map[string]string) (*DockerClient, error) {
	client, err := dockerclient.NewClientWithOpts(dockerclient.FromEnv, dockerclient.WithHost(host), dockerclient.WithVersion(version), dockerclient.WithHTTPClient(http), dockerclient.WithHTTPHeaders(headers))
	if err != nil {
		return nil, err
	}
	return &DockerClient{Client: client}, nil
}
