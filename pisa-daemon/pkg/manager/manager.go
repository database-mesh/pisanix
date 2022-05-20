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

package manager

import (
	"context"
	"os"
	"strings"

	"github.com/database-mesh/waterline/api/v1alpha1"
	"github.com/database-mesh/waterline/pkg/bpf"
	"github.com/database-mesh/waterline/pkg/cri"
	trafficqos "github.com/database-mesh/waterline/pkg/kubernetes/controllers/trafficqos"
	trafficqosmapping "github.com/database-mesh/waterline/pkg/kubernetes/controllers/trafficqosmapping"
	virtualdatabase "github.com/database-mesh/waterline/pkg/kubernetes/controllers/virtualdatabase"
	"github.com/database-mesh/waterline/pkg/kubernetes/watcher"
	"github.com/database-mesh/waterline/pkg/tc"
	"github.com/mlycore/log"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/watch"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/healthz"
	ctrlmgr "sigs.k8s.io/controller-runtime/pkg/manager"
)

type Manager struct {
	Pod *watcher.PodWatcher
	Mgr ctrlmgr.Manager
	CRI cri.ContainerRuntimeInterfaceClient
}

func (m *Manager) WatchAndHandle() error {
	for {
		select {
		case event := <-m.Pod.Core.ResultChan():
			{
				pod := event.Object.(*corev1.Pod)
				log.Infof("[%s] pod event: %#v", event.Type, event.Object.(*corev1.Pod).Name)
				//TODO: Handle different types of events
				switch event.Type {
				case watch.Added:
					handleAdded(pod, m.Mgr.GetClient(), m.CRI)
				case watch.Modified:
					handleModified(pod, m.Mgr.GetClient(), m.CRI)
				case watch.Deleted:
					handleDeleted(pod, m.Mgr.GetClient(), m.CRI)
				}
			}
		}
	}
	return nil
}

func (m *Manager) Bootstrap() error {
	if err := (&trafficqos.TrafficQoSReconciler{
		Client: m.Mgr.GetClient(),
		Scheme: m.Mgr.GetScheme(),
	}).SetupWithManager(m.Mgr); err != nil {
		log.Errorf("trafficqos setupWithManager error: %s", err)
		return err
	}

	if err := (&trafficqosmapping.TrafficQoSMappingReconciler{
		Client: m.Mgr.GetClient(),
		Scheme: m.Mgr.GetScheme(),
	}).SetupWithManager(m.Mgr); err != nil {
		log.Errorf("trafficqosmapping setupWithManager error: %s", err)
		return err
	}

	if err := (&virtualdatabase.VirtualDatabaseReconciler{
		Client: m.Mgr.GetClient(),
		Scheme: m.Mgr.GetScheme(),
	}).SetupWithManager(m.Mgr); err != nil {
		log.Errorf("virtualdatabase setupWithManager error: %s", err)
		return err
	}

	if err := m.Mgr.AddHealthzCheck("healthz", healthz.Ping); err != nil {
		return err
	}
	if err := m.Mgr.AddReadyzCheck("readyz", healthz.Ping); err != nil {
		return err
	}

	if err := m.Mgr.Start(ctrl.SetupSignalHandler()); err != nil {
		return err
	}
	return nil
}

func handleAdded(pod *corev1.Pod, c client.Client, cr cri.ContainerRuntimeInterfaceClient) error {
	//TODO: add related rules
	hostname, err := os.Hostname()
	if err != nil {
		return err
	}

	if hostname == pod.Spec.Hostname {
		list := &v1alpha1.VirtualDatabaseList{Items: []v1alpha1.VirtualDatabase{}}

		if err := c.List(context.TODO(), list, &client.ListOptions{Namespace: pod.Namespace}); err != nil {
			log.Errorf("get SQLTrafficQos error: %s", err)
			return err
		}

		for _, db := range list.Items {
			var found bool
			for k, v := range db.Spec.Selector {
				if pod.Labels[k] == v {
					found = true
				} else {
					found = false
				}
			}

			if found {
				l := &bpf.Loader{}
				containerId := strings.Split(pod.Status.ContainerStatuses[0].ContainerID, "containerd://")[1]
				pid, err := cr.GetPidFromContainer(context.TODO(), containerId)
				if err != nil {
					return err
				}
				ifname, err := tc.GetNetworkDeviceFromPid(pid)
				if err != nil {
					return err
				}
				// Add veth egress
				err = l.Load(ifname, uint16(db.Spec.Server.Port))
				if err != nil {
					return err
				}

			}

			// TODO: add loader
			// db.Spec.Server.Port
			// db.Spec.QoS
		}

	}
	return nil
}

func handleModified(pod *corev1.Pod, c client.Client, cr cri.ContainerRuntimeInterfaceClient) {

}

func handleDeleted(pod *corev1.Pod, c client.Client, cr cri.ContainerRuntimeInterfaceClient) {
	//TODO: remove related rules
	// move it to a queue ?
}
