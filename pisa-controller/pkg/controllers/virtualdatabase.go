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

package controllers

import (
	"context"

	v1alpha1 "github.com/database-mesh/golang-sdk/kubernetes/api/v1alpha1"

	appsv1 "k8s.io/api/apps/v1"
	v1 "k8s.io/api/core/v1"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"

	logger "sigs.k8s.io/controller-runtime/pkg/log"
)

// VirtualDatabaseReconciler reconciles a VirtualDatabase object
type VirtualDatabaseReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
func (r *VirtualDatabaseReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	log := logger.FromContext(ctx)

	rt, err := r.getRuntimeVirtualDatabase(ctx, req.NamespacedName)
	if apierrors.IsNotFound(err) {
		log.Info("Resource in work queue no longer exists!")
		return ctrl.Result{}, nil
	} else if err != nil {
		log.Error(err, "Error getting CRD resource")
		return ctrl.Result{}, err
	}

	log.Info("namespace: %s, name: %s", rt.Namespace, rt.Name)

	return r.reconcile(ctx, req, rt)
}

func (r *VirtualDatabaseReconciler) getRuntimeVirtualDatabase(ctx context.Context, namespacedName types.NamespacedName) (*v1alpha1.VirtualDatabase, error) {
	rt := &v1alpha1.VirtualDatabase{}
	err := r.Get(ctx, namespacedName, rt)
	return rt, err
}

func (r *VirtualDatabaseReconciler) reconcile(ctx context.Context, req ctrl.Request, rt *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	// log := logger.FromContext(ctx)
	// if res, err := r.reconcileVirtualDatabase(ctx, req.NamespacedName, rt); err != nil {
	// 	log.Error(err, "Error reconcile Deployment")
	// 	return res, err
	// }

	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) reconcileVirtualDatabase(ctx context.Context, namespacedName types.NamespacedName, vdb *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *VirtualDatabaseReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&v1alpha1.VirtualDatabase{}).
		Owns(&appsv1.Deployment{}).
		Owns(&v1.Service{}).
		Owns(&v1.Pod{}).
		Complete(r)
}
