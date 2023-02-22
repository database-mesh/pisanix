// Copyright 2023 SphereEx Authors
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
	"errors"
	"strings"

	"github.com/database-mesh/golang-sdk/aws/client/rds"
	v1alpha1 "github.com/database-mesh/golang-sdk/kubernetes/api/v1alpha1"
	"github.com/database-mesh/pisanix/pisa-controller/pkg/utils"
	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	logger "sigs.k8s.io/controller-runtime/pkg/log"
)

// DatabaseEndpointReconciler reconciles a DatabaseEndpoint object
type DatabaseEndpointReconciler struct {
	client.Client
	Scheme *runtime.Scheme
	AWSRds rds.RDS
}

// SetupWithManager sets up the controller with the Manager.
func (r *DatabaseEndpointReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&v1alpha1.DatabaseEndpoint{}).
		Complete(r)
}

func (r *DatabaseEndpointReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	log := logger.FromContext(ctx)

	dbep, err := r.getDatabaseEndpoint(ctx, req.NamespacedName)
	if apierrors.IsNotFound(err) {
		log.Info("Resource in work queue no longer exists!")
		return ctrl.Result{}, nil
	} else if err != nil {
		log.Error(err, "Error getting CRD resource")
		return ctrl.Result{}, err
	}

	if err := r.reconcileFinalizers(ctx, req, dbep); err != nil {
		return ctrl.Result{Requeue: true}, err
	}

	if err := r.reconcile(ctx, req, dbep); err != nil {
		return ctrl.Result{Requeue: true}, err
	}

	return ctrl.Result{RequeueAfter: ReconcileTime}, nil
}

func (r *DatabaseEndpointReconciler) getDatabaseEndpoint(ctx context.Context, namespacedName types.NamespacedName) (*v1alpha1.DatabaseEndpoint, error) {
	dbep := &v1alpha1.DatabaseEndpoint{}
	err := r.Get(ctx, namespacedName, dbep)
	return dbep, err
}

func (r *DatabaseEndpointReconciler) reconcileFinalizers(ctx context.Context, req ctrl.Request, dbep *v1alpha1.DatabaseEndpoint) error {
	//TODO
	if dbep.Annotations[v1alpha1.AnnotationsDatabaseClassName] != "" {
		return r.finalizeWithDatabaseClass(ctx, req, dbep)
	}
	return nil
}

func (r *DatabaseEndpointReconciler) finalizeWithDatabaseClass(ctx context.Context, req ctrl.Request, dbep *v1alpha1.DatabaseEndpoint) error {
	if dbep.DeletionTimestamp.IsZero() {
		return r.appendFinalizers(ctx, dbep)
	} else {
		if utils.ContainsString(dbep.ObjectMeta.Finalizers, AWSRdsFinalizer) {
			class := &v1alpha1.DatabaseClass{}
			err := r.Get(ctx, types.NamespacedName{
				Name: dbep.Annotations[v1alpha1.AnnotationsDatabaseClassName],
			}, class)
			if err != nil {
				return err
			}

			err = r.finalizeAWS(ctx, dbep, class.Spec.Provisioner)
			if err != nil {
				return err
			}

			err = r.removeFinalizers(ctx, dbep)
			if err != nil {
				return err
			}
		}
	}
	return nil
}

func (r *DatabaseEndpointReconciler) finalizeAWS(ctx context.Context, dbep *v1alpha1.DatabaseEndpoint, provisioner v1alpha1.DatabaseProvisioner) error {
	switch provisioner {
	case v1alpha1.DatabaseProvisionerAWSRdsInstance:
		fallthrough
	case v1alpha1.DatabaseProvisionerAWSRdsAurora:
		return r.finalizeAWSRdsInstance(ctx, dbep)
	case v1alpha1.DatabaseProvisionerAWSRdsCluster:
		fallthrough
	default:
		return nil
	}
}

func (r *DatabaseEndpointReconciler) finalizeAWSRdsInstance(ctx context.Context, dbep *v1alpha1.DatabaseEndpoint) error {
	return r.deleteAWSRdsInstance(ctx, dbep.Name)
}

func (r *DatabaseEndpointReconciler) deleteAWSRdsInstance(ctx context.Context, name string) error {
	_, err := r.AWSRds.Instance().SetDBInstanceIdentifier(name).Describe(ctx)
	if err != nil && strings.Contains(err.Error(), "DBInstanceNotFound") {
		return nil
	}

	err = r.AWSRds.Instance().SetDBInstanceIdentifier(name).SetSkipFinalSnapshot(true).Delete(ctx)
	if err != nil && !strings.Contains(err.Error(), "is already being deleted") {
		return err
	}

	return nil
}

func (r *DatabaseEndpointReconciler) appendFinalizers(ctx context.Context, dbep *v1alpha1.DatabaseEndpoint) error {
	if !utils.ContainsString(dbep.ObjectMeta.Finalizers, AWSRdsFinalizer) {
		dbep.ObjectMeta.Finalizers = append(dbep.ObjectMeta.Finalizers, AWSRdsFinalizer)
		if err := r.Update(ctx, dbep); err != nil {
			return err
		}
	}
	return nil
}

func (r *DatabaseEndpointReconciler) removeFinalizers(ctx context.Context, dbep *v1alpha1.DatabaseEndpoint) error {
	if utils.ContainsString(dbep.ObjectMeta.Finalizers, AWSRdsFinalizer) {
		dbep.ObjectMeta.Finalizers = utils.RemoveString(dbep.ObjectMeta.Finalizers, AWSRdsFinalizer)
		if err := r.Update(ctx, dbep); err != nil {
			return err
		}
	}
	return nil
}

func (r *DatabaseEndpointReconciler) reconcile(ctx context.Context, req ctrl.Request, dbep *v1alpha1.DatabaseEndpoint) error {
	classname := dbep.Annotations[v1alpha1.AnnotationsDatabaseClassName]
	if classname != "" {
		class := &v1alpha1.DatabaseClass{}
		err := r.Get(ctx, types.NamespacedName{
			Namespace: dbep.Namespace,
			Name:      classname,
		}, class)
		if err != nil {
			return err
		}

		switch class.Spec.Provisioner {
		case v1alpha1.DatabaseProvisionerAWSRdsInstance:
			err = r.reconcileAWSRdsInstance(ctx, req, dbep, class)
		case v1alpha1.DatabaseProvisionerAWSRdsCluster:
			err = r.reconcileAWSRdsCluster(ctx, req, dbep, class)
		case v1alpha1.DatabaseProvisionerAWSRdsAurora:
			err = r.reconcileAWSRdsCluster(ctx, req, dbep, class)
		default:
			return errors.New("unknown DatabaseClass provisioner")
		}
		return err
	}
	return nil
}

func (r *DatabaseEndpointReconciler) reconcileAWSRdsInstance(ctx context.Context, req ctrl.Request, dbep *v1alpha1.DatabaseEndpoint, class *v1alpha1.DatabaseClass) error {
	subnetGroupName := dbep.Annotations[v1alpha1.AnnotationsSubnetGroupName]
	vpcSecurityGroupIds := dbep.Annotations[v1alpha1.AnnotationsVPCSecurityGroupIds]

	rdsDesc, err := r.AWSRds.Instance().SetDBInstanceIdentifier(dbep.Name).Describe(ctx)
	if err != nil {
		if strings.Contains(err.Error(), "DBInstanceNotFound") {
			if err := r.AWSRds.Instance().
				SetEngine(class.Spec.Engine.Name).
				SetEngineVersion(class.Spec.Engine.Version).
				SetDBInstanceClass(class.Spec.Instance.Class).
				SetAllocatedStorage(class.Spec.Storage.AllocatedStorage).
				//NOTE: It will be invalid if this is a auto sharding
				SetVpcSecurityGroupIds(strings.Split(vpcSecurityGroupIds, ",")).
				SetDBSubnetGroup(subnetGroupName).
				//FIXME: should add this DatabaseClass name to tags
				SetDBInstanceIdentifier(dbep.Name).
				SetMasterUsername(dbep.Spec.Database.MySQL.User).
				SetMasterUserPassword(dbep.Spec.Database.MySQL.Password).
				SetDBName(dbep.Spec.Database.MySQL.DB).
				Create(ctx); err != nil {
				return err
			}
			return nil
		}
		return err
	}

	if rdsDesc.DBInstanceStatus == "Deleting" {
		return nil
	}

	// Update or Delete
	if rdsDesc != nil {
		act, err := r.getDatabaseEndpoint(ctx, req.NamespacedName)
		if err != nil {
			return err
		}
		act.Spec.Database.MySQL.Host = rdsDesc.Endpoint.Address
		act.Spec.Database.MySQL.Port = uint32(rdsDesc.Endpoint.Port)
		if err := r.Update(ctx, act); err != nil {
			return err
		}
		act.Status.Protocol = "MySQL"
		act.Status.Endpoint = rdsDesc.Endpoint.Address
		act.Status.Port = uint32(rdsDesc.Endpoint.Port)
		act.Status.Arn = rdsDesc.DBInstanceArn
		if err := r.Status().Update(ctx, act); err != nil {
			return err
		}
	}
	return nil
}

func (r *DatabaseEndpointReconciler) reconcileAWSRdsCluster(ctx context.Context, req ctrl.Request, dbep *v1alpha1.DatabaseEndpoint, class *v1alpha1.DatabaseClass) error {
	rdsDesc, err := r.AWSRds.Instance().SetDBInstanceIdentifier(dbep.Name).Describe(ctx)
	if err != nil && strings.Contains(err.Error(), "DBInstanceNotFound") {
		//FIXME: if the Rds instance is deleted which can not be described, will return as normal
		return nil
	}
	if err != nil {
		return err
	}

	if rdsDesc.DBInstanceStatus == "Deleting" {
		return nil
	}

	// Update or Delete
	if rdsDesc != nil {
		act, err := r.getDatabaseEndpoint(ctx, req.NamespacedName)
		if err != nil {
			return err
		}
		act.Spec.Database.MySQL.Host = rdsDesc.Endpoint.Address
		act.Spec.Database.MySQL.Port = uint32(rdsDesc.Endpoint.Port)
		if err := r.Update(ctx, act); err != nil {
			return err
		}
		act.Status.Protocol = "MySQL"
		act.Status.Endpoint = rdsDesc.Endpoint.Address
		act.Status.Port = uint32(rdsDesc.Endpoint.Port)
		act.Status.Arn = rdsDesc.DBInstanceArn
		if err := r.Status().Update(ctx, act); err != nil {
			return err
		}
	}
	return nil
}
