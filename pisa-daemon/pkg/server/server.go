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

package server

import (
	"github.com/database-mesh/pisanix/pisa-daemon/api/v1alpha1"
	"github.com/database-mesh/pisanix/pisa-daemon/pkg/cri"
	"github.com/database-mesh/pisanix/pisa-daemon/pkg/kubernetes/controllers"
	"github.com/database-mesh/pisanix/pisa-daemon/pkg/kubernetes/watcher"
	"github.com/database-mesh/pisanix/pisa-daemon/pkg/manager"
	"github.com/database-mesh/pisanix/pisa-daemon/pkg/server/config"

	"github.com/mlycore/log"
	"github.com/pkg/errors"
	"golang.org/x/sync/errgroup"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/client-go/kubernetes"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	ctrl "sigs.k8s.io/controller-runtime"
)

var (
	scheme = runtime.NewScheme()
)

func init() {
	_ = v1alpha1.AddToScheme(scheme)
	_ = clientgoscheme.AddToScheme(scheme)
}

type Server struct {
	KubernetesClient       kubernetes.Interface
	ContainerRuntimeClient cri.ContainerRuntimeInterfaceClient
}

func New(conf *config.Config) (*Server, error) {
	cr, err := cri.NewContainerRuntimeInterfaceClient(conf.CRI)
	if err != nil {
		return nil, err
	}

	kcfg := ctrl.GetConfigOrDie()
	kc, err := kubernetes.NewForConfig(kcfg)
	if err != nil {
		return nil, err
	}

	return &Server{
		ContainerRuntimeClient: cr,
		KubernetesClient:       kc,
	}, nil
}

func (s *Server) Run() error {
	// var metricsAddr string
	var enableLeaderElection bool
	// var probeAddr string

	mgr, err := controllers.NewManager(ctrl.GetConfigOrDie(), &ctrl.Options{
		Scheme:                 scheme,
		MetricsBindAddress:     ":8280",
		Port:                   9444,
		HealthProbeBindAddress: ":8281",
		LeaderElection:         enableLeaderElection,
		LeaderElectionID:       "cb843f1f.my.domain",
	})
	if err != nil {
		return err
	}
	w, err := watcher.NewPodWatcher(s.KubernetesClient)
	if err != nil {
		return err
	}
	manager := &manager.Manager{
		CRI: s.ContainerRuntimeClient,
		Pod: w,
		Mgr: mgr,
	}
	var eg errgroup.Group

	// apply to Pod
	eg.Go(func() error {
		log.Infof("starting controllers")
		if err := manager.Bootstrap(); err != nil {
			return errors.Wrap(err, "start controllers")
		}
		return nil
	})

	// remove from Pod
	eg.Go(func() error {
		log.Infof("starting watching kubernetes")
		if err := manager.WatchAndHandle(); err != nil {
			return errors.Wrap(err, "start pod watcher")
		}
		return nil
	})

	return eg.Wait()
}
