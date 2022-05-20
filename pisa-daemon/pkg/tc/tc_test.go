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
	"github.com/database-mesh/waterline/api/v1alpha1"
	"github.com/vishvananda/netlink"
	"github.com/vishvananda/netns"
	"os"
	"runtime"
	"testing"
)

// from https://github.com/vishvananda/netlink/blob/main/netlink_test.go
type tearDownNetlinkTest func()

func skipUnlessRoot(t *testing.T) {
	if os.Getuid() != 0 {
		t.Skip("Test requires root privileges.")
	}
}

func setUpNetlinkTest(t *testing.T) tearDownNetlinkTest {
	skipUnlessRoot(t)

	// new temporary namespace so we don't pollute the host
	// lock thread since the namespace is thread local
	runtime.LockOSThread()
	var err error
	ns, err := netns.NewNamed("test")
	if err != nil {
		t.Fatal("Failed to create newns", ns)
	}

	return func() {
		netns.DeleteNamed("test")
		runtime.UnlockOSThread()
	}
}
func TestShaper_AddClass(t *testing.T) {
	tearDown := setUpNetlinkTest(t)
	defer tearDown()

	if err := netlink.LinkAdd(&netlink.Ifb{LinkAttrs: netlink.LinkAttrs{Name: "foo"}}); err != nil {
		t.Fatal(err)
	}

	trafficQos := v1alpha1.SQLTrafficQoS{
		Spec: v1alpha1.SQLTrafficQoSSpec{
			NetworkDevice: "foo",
			QoSClass:      v1alpha1.QoSClassGuaranteed,
			Strategy:      v1alpha1.TrafficQoSStrategyPreDefined,
			Groups: []v1alpha1.TrafficQoSGroup{
				{
					Rate: "10Mi",
				},
				{
					Rate: "20M",
				},
			},
		},
	}

	shaper, err := NewTcShaper(trafficQos, "100M")
	if err != nil {
		t.Fatal(err)
	}

	if err := shaper.AddClasses(); err != nil {
		t.Fatal(err)
	}
}
