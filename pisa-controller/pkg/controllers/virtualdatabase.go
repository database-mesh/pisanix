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
	"os"
	"strings"

	"github.com/database-mesh/golang-sdk/aws"
	"github.com/database-mesh/golang-sdk/aws/client/rds"
	v1alpha1 "github.com/database-mesh/golang-sdk/kubernetes/api/v1alpha1"

	apierrors "k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
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

	return r.reconcile(ctx, req, rt)
}

func (r *VirtualDatabaseReconciler) getRuntimeVirtualDatabase(ctx context.Context, namespacedName types.NamespacedName) (*v1alpha1.VirtualDatabase, error) {
	rt := &v1alpha1.VirtualDatabase{}
	err := r.Get(ctx, namespacedName, rt)
	return rt, err
}

func (r *VirtualDatabaseReconciler) reconcile(ctx context.Context, req ctrl.Request, rt *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	if rt.Spec.DatabaseClassName != "" {
		class := &v1alpha1.DatabaseClass{}
		err := r.Get(ctx, types.NamespacedName{
			Namespace: rt.Namespace,
			Name:      rt.Spec.DatabaseClassName,
		}, class)
		if err != nil {
			return ctrl.Result{}, err
		}

		subnetGroupName := class.Annotations[v1alpha1.AnnotationsSubnetGroupName]
		vpcSecurityGroupIds := class.Annotations[v1alpha1.AnnotationsVPCSecurityGroupIds]

		if class.Spec.Provisioner == v1alpha1.DatabaseProvisionerAWSRdsInstance {
			//TODO: first check AWS RDS Instance
			region := os.Getenv("AWS_REGION")
			accessKey := os.Getenv("AWS_ACCESS_KEY")
			secretAccessKey := os.Getenv("AWS_SECRET_ACCESS_KEY")
			sess := aws.NewSessions().SetCredential(region, accessKey, secretAccessKey).Build()
			if err := rds.NewService(sess[region]).Instance().
				SetEngine(class.Spec.Engine.Name).
				SetEngineVersion(class.Spec.Engine.Version).
				//FIXME: should be random name
				SetDBInstanceIdentifier(class.Name).
				//NOTE: if DefaultMasterUsername
				//FIXME: should be a default value or a specific one
				SetMasterUsername("admin").
				//FIXME: should be a randompass
				SetMasterUserPassword("randompass").
				SetDBInstanceClass(class.Spec.Instance.Class).
				SetAllocatedStorage(class.Spec.Storage.AllocatedStorage).
				//FIXME: checkout different service
				SetDBName(rt.Spec.Services[0].DatabaseMySQL.DB).
				SetVpcSecurityGroupIds(strings.Split(vpcSecurityGroupIds, ",")).
				SetDBSubnetGroup(subnetGroupName).
				Create(context.TODO()); err != nil {
				return ctrl.Result{}, err
			}

			dbep := &v1alpha1.DatabaseEndpoint{
				ObjectMeta: metav1.ObjectMeta{
					Name:      class.Name,
					Namespace: rt.Namespace,
				},
				Spec: v1alpha1.DatabaseEndpointSpec{
					Database: v1alpha1.Database{
						MySQL: &v1alpha1.MySQL{
							Host:     "thisshouldbe",
							Port:     3306,
							User:     "admin",
							Password: "randompass",
							DB:       rt.Spec.Services[0].DatabaseMySQL.DB,
						},
					},
				},
			}
			if err := r.Create(ctx, dbep); err != nil {
				return ctrl.Result{}, err
			}
		}
	}

	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) reconcileVirtualDatabase(ctx context.Context, namespacedName types.NamespacedName, vdb *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *VirtualDatabaseReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&v1alpha1.VirtualDatabase{}).
		Complete(r)
}
