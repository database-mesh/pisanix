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

package kubernetes

import (
	"context"

	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"

	"github.com/mlycore/log"

	"github.com/database-mesh/waterline/api/v1alpha1"
	"github.com/database-mesh/waterline/pkg/tc"
)

// TrafficQoSReconciler reconciles a TrafficQoS object
type TrafficQoSReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

//+kubebuilder:rbac:groups=database-mesh.io.my.domain,resources=sqltrafficqos,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=database-mesh.io.my.domain,resources=sqltrafficqos/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=database-mesh.io.my.domain,resources=sqltrafficqos/finalizers,verbs=update

func (r *TrafficQoSReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	// _ = log.FromContext(ctx)

	// TODO(user): your logic here
	obj := &v1alpha1.TrafficQoS{}
	if err := r.Client.Get(ctx, req.NamespacedName, obj); err != nil {
		log.Errorf("get TrafficQos error: %s", err)
		return ctrl.Result{}, nil
	}

	// TODO: sync TrafficQoSStatus
	defer func() {

	}()

	// TODO: add logic, remove VirtualDatabase.
	// Read TrafficQoS for basic QoS class up.
	// Read VirtualDatabase for application-level QoS after a Pod was scheduled on this Node

	//TODO: all rules should be removed once the resource was deleted
	//TODO: is there an exception when more than one resource was created

	//TODO: add check for existed class rules

	shaper, err := tc.NewTcShaper(*obj, "1000M")
	if err != nil {
		log.Errorf("get shaper error: %s", err)
		return ctrl.Result{Requeue: true}, nil
	}

	if err = shaper.AddClasses(); err != nil {
		log.Errorf("add classes error: %s", err)
		return ctrl.Result{Requeue: true}, nil
	}

	log.Infof("TrafficQoS: %#v", obj)

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *TrafficQoSReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&v1alpha1.TrafficQoS{}).
		Complete(r)
}
