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
	"errors"
	"strings"
	"time"

	"github.com/database-mesh/golang-sdk/aws/client/rds"
	v1alpha1 "github.com/database-mesh/golang-sdk/kubernetes/api/v1alpha1"
	"github.com/database-mesh/pisanix/pisa-controller/pkg/utils"

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
	AWSRds rds.RDS
}

// SetupWithManager sets up the controller with the Manager.
func (r *VirtualDatabaseReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&v1alpha1.VirtualDatabase{}).
		Owns(&v1alpha1.DatabaseEndpoint{}).
		Complete(r)
}

const ReconcileTime = 30 * time.Second

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
func (r *VirtualDatabaseReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	log := logger.FromContext(ctx)

	vdb, err := r.getVirtualDatabase(ctx, req.NamespacedName)
	if apierrors.IsNotFound(err) {
		log.Info("Resource in work queue no longer exists!")
		//NOTE: how to know this is a DatabaseClass provisioned VirtualDatabase ?
		return ctrl.Result{}, nil
	} else if err != nil {
		log.Error(err, "Error getting CRD resource")
		return ctrl.Result{}, err
	}

	if _, err := r.reconcileFinalizers(ctx, req, vdb); err != nil {
		return ctrl.Result{RequeueAfter: ReconcileTime}, err
	}

	return r.updateVirtualDatabase(ctx, req, vdb)
}

func (r *VirtualDatabaseReconciler) getVirtualDatabase(ctx context.Context, namespacedName types.NamespacedName) (*v1alpha1.VirtualDatabase, error) {
	vdb := &v1alpha1.VirtualDatabase{}
	err := r.Get(ctx, namespacedName, vdb)
	return vdb, err
}

func (r *VirtualDatabaseReconciler) reconcileFinalizers(ctx context.Context, req ctrl.Request, rt *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	if rt.Spec.DatabaseClassName != "" {
		return r.finalizeWithDatabaseClass(ctx, req, rt)
	}
	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) finalizeWithDatabaseClass(ctx context.Context, req ctrl.Request, vdb *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	if vdb.DeletionTimestamp.IsZero() {
		return r.appendFinalizers(ctx, vdb)
	} else {
		if utils.ContainsString(vdb.ObjectMeta.Finalizers, AWSRdsFinalizer) {
			class := &v1alpha1.DatabaseClass{}
			err := r.Get(ctx, types.NamespacedName{
				Name: vdb.Spec.DatabaseClassName,
			}, class)
			if err != nil {
				return ctrl.Result{}, err
			}

			res, err := r.finalizeDbep(ctx, req, vdb, class)
			if err != nil {
				return res, err
			}

			res, err = r.removeFinalizers(ctx, vdb)
			if err != nil {
				return res, err
			}
		}
	}
	return ctrl.Result{}, nil
}

const (
	AWSRdsFinalizer = "cleanupAWSRds"
)

func (r *VirtualDatabaseReconciler) appendFinalizers(ctx context.Context, vdb *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	if !utils.ContainsString(vdb.ObjectMeta.Finalizers, AWSRdsFinalizer) {
		vdb.ObjectMeta.Finalizers = append(vdb.ObjectMeta.Finalizers, AWSRdsFinalizer)
		if err := r.Update(ctx, vdb); err != nil {
			return ctrl.Result{Requeue: true}, nil
		}
	}
	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) removeFinalizers(ctx context.Context, vdb *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	if utils.ContainsString(vdb.ObjectMeta.Finalizers, AWSRdsFinalizer) {
		vdb.ObjectMeta.Finalizers = utils.RemoveString(vdb.ObjectMeta.Finalizers, AWSRdsFinalizer)
		if err := r.Update(ctx, vdb); err != nil {
			return ctrl.Result{Requeue: true}, nil
		}
	}
	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) finalizeDbep(ctx context.Context, req ctrl.Request, vdb *v1alpha1.VirtualDatabase, class *v1alpha1.DatabaseClass) (ctrl.Result, error) {
	switch class.Spec.Provisioner {
	case v1alpha1.DatabaseProvisionerAWSRdsInstance:
		return r.finalizeAWSRdsInstance(ctx, req, vdb, class)
	case v1alpha1.DatabaseProvisionerAWSRdsCluster:
		return r.finalizeAWSRdsCluster(ctx, req, vdb, class)
	case v1alpha1.DatabaseProvisionerAWSRdsAurora:
		return ctrl.Result{}, nil
	}
	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) finalizeAWSRdsInstance(ctx context.Context, req ctrl.Request, vdb *v1alpha1.VirtualDatabase, class *v1alpha1.DatabaseClass) (ctrl.Result, error) {
	dbep := &v1alpha1.DatabaseEndpoint{}
	if err := r.Get(ctx, types.NamespacedName{
		Namespace: vdb.Namespace,
		Name:      vdb.Name,
	}, dbep); err != nil {
		if apierrors.IsNotFound(err) {
			return ctrl.Result{}, err
		}
	} else {
		if _, err := r.deleteAWSRds(ctx, req, dbep, class.Spec.Provisioner); err != nil {
			return ctrl.Result{RequeueAfter: ReconcileTime}, err
		}
		// FIXME:
		if err := r.Delete(ctx, dbep); err != nil {
			return ctrl.Result{RequeueAfter: ReconcileTime}, err
		}
	}

	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) finalizeAWSRdsCluster(ctx context.Context, req ctrl.Request, vdb *v1alpha1.VirtualDatabase, class *v1alpha1.DatabaseClass) (ctrl.Result, error) {
	dbep := &v1alpha1.DatabaseEndpoint{}
	if err := r.Get(ctx, types.NamespacedName{
		Namespace: vdb.Namespace,
		Name:      vdb.Name,
	}, dbep); err != nil {
		if apierrors.IsNotFound(err) {
			return ctrl.Result{}, err
		}
	} else {
		if _, err := r.deleteAWSRds(ctx, req, dbep, class.Spec.Provisioner); err != nil {
			return ctrl.Result{RequeueAfter: ReconcileTime}, err
		}
		// FIXME:
		if err := r.Delete(ctx, dbep); err != nil {
			return ctrl.Result{RequeueAfter: ReconcileTime}, err
		}
	}
	return ctrl.Result{}, nil
}

// TODO: turn provisioner into some object
func (r *VirtualDatabaseReconciler) deleteAWSRds(ctx context.Context, req ctrl.Request, dbep *v1alpha1.DatabaseEndpoint, provisioner v1alpha1.DatabaseProvisioner) (ctrl.Result, error) {
	switch provisioner {
	case v1alpha1.DatabaseProvisionerAWSRdsCluster:
		return r.deleteAWSRdsCluster(ctx, dbep.Name)
	case v1alpha1.DatabaseProvisionerAWSRdsInstance:
		return r.deleteAWSRdsInstance(ctx, dbep.Name)
	}

	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) deleteAWSRdsCluster(ctx context.Context, name string) (ctrl.Result, error) {
	_, err := r.AWSRds.Cluster().SetDBClusterIdentifier(name).Describe(ctx)
	if err != nil && strings.Contains(err.Error(), "DBClusterNotFoundFault") {
		return ctrl.Result{}, nil
	}

	err = r.AWSRds.Cluster().SetDBClusterIdentifier(name).SetSkipFinalSnapshot(true).Delete(ctx)
	if err != nil && !strings.Contains(err.Error(), "is already being deleted") {
		return ctrl.Result{Requeue: true}, err
	}
	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) deleteAWSRdsInstance(ctx context.Context, name string) (ctrl.Result, error) {
	_, err := r.AWSRds.Instance().SetDBInstanceIdentifier(name).Describe(ctx)
	if err != nil && strings.Contains(err.Error(), "DBInstanceNotFound") {
		return ctrl.Result{}, nil
	}

	err = r.AWSRds.Instance().SetDBInstanceIdentifier(name).SetSkipFinalSnapshot(true).Delete(ctx)
	if err != nil && !strings.Contains(err.Error(), "is already being deleted") {
		return ctrl.Result{Requeue: true}, err
	}

	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) updateVirtualDatabase(ctx context.Context, req ctrl.Request, rt *v1alpha1.VirtualDatabase) (ctrl.Result, error) {
	if rt.Spec.DatabaseClassName != "" {
		class := &v1alpha1.DatabaseClass{}
		err := r.Get(ctx, types.NamespacedName{
			Namespace: rt.Namespace,
			Name:      rt.Spec.DatabaseClassName,
		}, class)
		if err != nil {
			return ctrl.Result{}, err
		}

		switch class.Spec.Provisioner {
		case v1alpha1.DatabaseProvisionerAWSRdsInstance:
			return r.reconcileAWSRds(ctx, req, rt, class)
		case v1alpha1.DatabaseProvisionerAWSRdsCluster:
			return r.reconcileAWSRds(ctx, req, rt, class)
		case v1alpha1.DatabaseProvisionerAWSRdsAurora:
			return r.reconcileAWSRds(ctx, req, rt, class)
		default:
			return ctrl.Result{RequeueAfter: ReconcileTime}, errors.New("unknown DatabaseClass provisioner")
		}
	}

	return ctrl.Result{RequeueAfter: ReconcileTime}, nil
}

func (r *VirtualDatabaseReconciler) reconcileAWSRds(ctx context.Context, req ctrl.Request, vdb *v1alpha1.VirtualDatabase, class *v1alpha1.DatabaseClass) (ctrl.Result, error) {
	for _, svc := range vdb.Spec.Services {
		if svc.DatabaseMySQL != nil {
			switch class.Spec.Provisioner {
			case v1alpha1.DatabaseProvisionerAWSRdsInstance:
				return r.reconcileAWSRdsInstance(ctx, req, vdb, class, svc)
			case v1alpha1.DatabaseProvisionerAWSRdsCluster:
				return r.reconcileAWSRdsCluster(ctx, req, vdb, class, svc)
			}
		}
	}
	return ctrl.Result{RequeueAfter: ReconcileTime}, nil
}

func (r *VirtualDatabaseReconciler) reconcileAWSRdsInstance(ctx context.Context, req ctrl.Request, vdb *v1alpha1.VirtualDatabase, class *v1alpha1.DatabaseClass, svc v1alpha1.VirtualDatabaseService) (ctrl.Result, error) {
	subnetGroupName := vdb.Annotations[v1alpha1.AnnotationsSubnetGroupName]
	vpcSecurityGroupIds := vdb.Annotations[v1alpha1.AnnotationsVPCSecurityGroupIds]
	randompass := utils.RandomString()
	rdsDesc, err := r.AWSRds.Instance().SetDBInstanceIdentifier(vdb.Name).Describe(ctx)
	if err != nil {
		if strings.Contains(err.Error(), "DBInstanceNotFound") {
			if err := r.AWSRds.Instance().
				SetEngine(class.Spec.Engine.Name).
				SetEngineVersion(class.Spec.Engine.Version).
				//FIXME: should add this DatabaseClass name to tags
				SetDBInstanceIdentifier(vdb.Name).
				SetMasterUsername(class.Spec.DefaultMasterUsername).
				SetMasterUserPassword(randompass).
				SetDBInstanceClass(class.Spec.Instance.Class).
				SetAllocatedStorage(class.Spec.Storage.AllocatedStorage).
				//NOTE: It will be invalid if this is a auto sharding
				SetDBName(svc.DatabaseMySQL.DB).
				SetVpcSecurityGroupIds(strings.Split(vpcSecurityGroupIds, ",")).
				SetDBSubnetGroup(subnetGroupName).
				Create(ctx); err != nil {
				return ctrl.Result{}, err
			}
		}
	}

	// Update or Delete
	dbep := &v1alpha1.DatabaseEndpoint{}
	err = r.Get(ctx, types.NamespacedName{
		Namespace: vdb.Namespace,
		Name:      vdb.Name,
	}, dbep)
	if err != nil {
		if vdb.DeletionTimestamp.IsZero() && apierrors.IsNotFound(err) {
			dbep := &v1alpha1.DatabaseEndpoint{
				ObjectMeta: metav1.ObjectMeta{
					Name:      vdb.Name,
					Namespace: vdb.Namespace,
				},
				Spec: v1alpha1.DatabaseEndpointSpec{
					Database: v1alpha1.Database{
						MySQL: &v1alpha1.MySQL{
							Host:     "",
							Port:     0,
							User:     class.Spec.DefaultMasterUsername,
							Password: randompass,
							DB:       svc.DatabaseMySQL.DB,
						},
					},
				},
			}
			if err := r.Create(ctx, dbep); err != nil {
				return ctrl.Result{}, err
			}
		}
		return ctrl.Result{}, err
	} else {
		if rdsDesc != nil {
			dbep.Spec.Database.MySQL.Host = rdsDesc.Endpoint.Address
			dbep.Spec.Database.MySQL.Port = uint32(rdsDesc.Endpoint.Port)
			if err := r.Update(ctx, dbep); err != nil {
				return ctrl.Result{Requeue: true}, err
			}
			dbep.Status.Protocol = "MySQL"
			dbep.Status.Endpoint = rdsDesc.Endpoint.Address
			dbep.Status.Port = uint32(rdsDesc.Endpoint.Port)
			dbep.Status.Arn = rdsDesc.DBInstanceArn
			if err := r.Status().Update(ctx, dbep); err != nil {
				return ctrl.Result{Requeue: true}, err
			}
		}
	}
	return ctrl.Result{}, nil
}

func (r *VirtualDatabaseReconciler) reconcileAWSRdsCluster(ctx context.Context, req ctrl.Request, vdb *v1alpha1.VirtualDatabase, class *v1alpha1.DatabaseClass, svc v1alpha1.VirtualDatabaseService) (ctrl.Result, error) {
	subnetGroupName := vdb.Annotations[v1alpha1.AnnotationsSubnetGroupName]
	vpcSecurityGroupIds := vdb.Annotations[v1alpha1.AnnotationsVPCSecurityGroupIds]
	availabilityZones := vdb.Annotations[v1alpha1.AnnotationsAvailabilityZones]
	randompass := utils.RandomString()
	rdsDesc, err := r.AWSRds.Cluster().SetDBClusterIdentifier(vdb.Name).Describe(ctx)
	if err != nil {
		if strings.Contains(err.Error(), "DBClusterNotFound") {
			if err := r.AWSRds.Cluster().
				SetEngine(class.Spec.Engine.Name).
				SetEngineVersion(class.Spec.Engine.Version).
				SetDBClusterIdentifier(vdb.Name).
				SetMasterUsername(class.Spec.DefaultMasterUsername).
				SetMasterUserPassword(randompass).
				SetDBClusterInstanceClass(class.Spec.Instance.Class).
				SetDatabaseName(svc.DatabaseMySQL.DB).
				SetAllocatedStorage(100).
				SetVpcSecurityGroupIds(strings.Split(vpcSecurityGroupIds, ",")).
				SetStorageType("io1").
				SetIOPS(1000).
				SetDBSubnetGroupName(subnetGroupName).
				SetAvailabilityZones(strings.Split(availabilityZones, ",")).
				// SetAllocatedStorage(class.Spec.Storage.AllocatedStorage).
				Create(ctx); err != nil {
				return ctrl.Result{}, err
			}
		}
	}

	// Update or Delete
	dbep := &v1alpha1.DatabaseEndpoint{}
	err = r.Get(ctx, types.NamespacedName{
		Namespace: vdb.Namespace,
		Name:      vdb.Name,
	}, dbep)
	if err != nil {
		if vdb.DeletionTimestamp.IsZero() && apierrors.IsNotFound(err) {
			dbep := &v1alpha1.DatabaseEndpoint{
				ObjectMeta: metav1.ObjectMeta{
					Name:      vdb.Name,
					Namespace: vdb.Namespace,
				},
				Spec: v1alpha1.DatabaseEndpointSpec{
					Database: v1alpha1.Database{
						MySQL: &v1alpha1.MySQL{
							Host:     "",
							Port:     0,
							User:     class.Spec.DefaultMasterUsername,
							Password: randompass,
							DB:       svc.DatabaseMySQL.DB,
						},
					},
				},
			}
			if err := r.Create(ctx, dbep); err != nil {
				return ctrl.Result{}, err
			}
		}
		return ctrl.Result{}, err
	} else {
		if rdsDesc != nil {
			//TODO: how to handle this PrimaryEndpoint and reader endpoint ?
			dbep.Spec.Database.MySQL.Host = rdsDesc.PrimaryEndpoint
			dbep.Spec.Database.MySQL.Port = uint32(rdsDesc.Port)
			if err := r.Update(ctx, dbep); err != nil {
				return ctrl.Result{Requeue: true}, err
			}
			dbep.Status.Protocol = "MySQL"
			dbep.Status.Endpoint = rdsDesc.PrimaryEndpoint
			dbep.Status.Port = uint32(rdsDesc.Port)
			dbep.Status.Arn = rdsDesc.DBClusterArn
			if err := r.Status().Update(ctx, dbep); err != nil {
				return ctrl.Result{Requeue: true}, err
			}
		}
	}
	return ctrl.Result{}, nil
}
