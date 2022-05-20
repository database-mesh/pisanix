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

package tc

import (
	"fmt"
	"os/exec"
	"strings"
)

func GetNetworkDeviceFromPid(pid uint32) (string, error) {
	igmpArgs := []string{
		fmt.Sprintf("/proc/%d/net/igmp", pid),
	}
	igmpCmd := exec.Command("cat", igmpArgs...)
	igmpinfo, err := igmpCmd.Output()
	if err != nil {
		return "", err
	}

	igmp := readlineFromCmdOutput(igmpinfo)

	var deviceId string
	for _, r := range igmp {
		if strings.Contains(r, "eth0") {
			deviceId = strings.Split(r, "\t")[0]
			break
		}
	}

	ipaCmd := exec.Command("ip", "a")
	ipainfo, err := ipaCmd.Output()
	if err != nil {
		return "", err
	}

	ipa := readlineFromCmdOutput(ipainfo)

	var (
		deviceName string
		deviceInfo string
	)

	for _, r := range ipa {
		if strings.Contains(r, fmt.Sprintf("if%s", deviceId)) {
			deviceInfo = strings.Split(r, " ")[1]
			deviceName = strings.Split(deviceInfo, "@")[0]
			break
		}
	}
	return deviceName, nil

}

func readlineFromCmdOutput(output []byte) []string {
	result := []string{}
	current := []byte{}

	for _, s := range output {
		current = append(current, s)
		if s == '\n' {
			result = append(result, string(current))
			current = []byte{}
		}
	}
	return result
}
