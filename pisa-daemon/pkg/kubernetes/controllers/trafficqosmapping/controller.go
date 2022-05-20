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
)

// TrafficQoSMappingReconciler reconciles a TrafficQoSMapping object
type TrafficQoSMappingReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

//+kubebuilder:rbac:groups=database-mesh.io.my.domain,resources=sqltrafficqos,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=database-mesh.io.my.domain,resources=sqltrafficqos/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=database-mesh.io.my.domain,resources=sqltrafficqos/finalizers,verbs=update

func (r *TrafficQoSMappingReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	// _ = log.FromContext(ctx)

	// TODO(user): your logic here
	obj := &v1alpha1.TrafficQoSMapping{}
	if err := r.Client.Get(ctx, req.NamespacedName, obj); err != nil {
		log.Errorf("get TrafficQos error: %s", err)
		return ctrl.Result{}, nil
	}

	// TODO: sync TrafficQoSMappingStatus
	defer func() {

	}()

	// TODO: add logic, remove VirtualDatabase.
	// Read TrafficQoSMapping for basic QoS class up.
	// Read VirtualDatabase for application-level QoS after a Pod was scheduled on this Node

	//TODO: all rules should be removed once the resource was deleted
	//TODO: is there an exception when more than one resource was created

	log.Infof("TrafficQoSMapping: %#v", obj)

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *TrafficQoSMappingReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&v1alpha1.TrafficQoSMapping{}).
		Complete(r)
}
