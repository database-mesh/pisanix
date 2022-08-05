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

package webhook

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func Test_GetPisaProxyDeployedNameFromPod(t *testing.T) {
	cases := []struct {
		name          string
		generatedName string
		kind          string
		exp           string
		msg           string
	}{
		{
			name:          "a-b-c-6668ddbf55-abcde",
			generatedName: "a-b-c-6668ddbf55-",
			kind:          "ReplicaSet",
			exp:           "a-b-c",
			msg:           "Deployment or CronJob name should be equal",
		},
		{
			name:          "a-b-0",
			generatedName: "a-b-",
			kind:          "StatefulSet",
			exp:           "a-b",
			msg:           "StatefulSet, Job or DaemonSet name should be equal",
		},
	}

	for _, c := range cases {
		assert.Equal(t, c.exp, getPisaProxyDeployedNameFromPod(c.kind, c.generatedName), c.msg)
	}
}
