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

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	logf "sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/webhook"
)

// log is for logging in this package.
var virtualdatabaselog = logf.Log.WithName("virtualdatabase-resource")

func (r *VirtualDatabase) SetupWebhookWithManager(mgr ctrl.Manager) error {
	return ctrl.NewWebhookManagedBy(mgr).
		For(r).
		Complete()
}

//+kubebuilder:webhook:path=/mutate-database-mesh-io-database-mesh-io-v1alpha1-virtualdatabase,mutating=true,failurePolicy=fail,sideEffects=None,groups=database-mesh.io.database-mesh.io,resources=virtualdatabases,verbs=create;update,versions=v1alpha1,name=mvirtualdatabase.kb.io,admissionReviewVersions=v1

var _ webhook.Defaulter = &VirtualDatabase{}

// Default implements webhook.Defaulter so a webhook will be registered for the type
func (r *VirtualDatabase) Default() {
	virtualdatabaselog.Info("default", "name", r.Name)
}

//+kubebuilder:webhook:path=/validate-database-mesh-io-database-mesh-io-v1alpha1-virtualdatabase,mutating=false,failurePolicy=fail,sideEffects=None,groups=database-mesh.io.database-mesh.io,resources=virtualdatabases,verbs=create;update,versions=v1alpha1,name=vvirtualdatabase.kb.io,admissionReviewVersions=v1

var _ webhook.Validator = &VirtualDatabase{}

// ValidateCreate implements webhook.Validator so a webhook will be registered for the type
func (r *VirtualDatabase) ValidateCreate() error {
	virtualdatabaselog.Info("validate create", "name", r.Name)

	return nil
}

// ValidateUpdate implements webhook.Validator so a webhook will be registered for the type
func (r *VirtualDatabase) ValidateUpdate(old runtime.Object) error {
	virtualdatabaselog.Info("validate update", "name", r.Name)

	return nil
}

// ValidateDelete implements webhook.Validator so a webhook will be registered for the type
func (r *VirtualDatabase) ValidateDelete() error {
	virtualdatabaselog.Info("validate delete", "name", r.Name)

	return nil
}
