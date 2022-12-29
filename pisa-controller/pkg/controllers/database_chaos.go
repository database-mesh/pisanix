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
	"time"

	"github.com/database-mesh/golang-sdk/aws"
	"github.com/database-mesh/golang-sdk/aws/client/rds"
	v1alpha1 "github.com/database-mesh/golang-sdk/kubernetes/api/v1alpha1"

	apierrors "k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"

	"github.com/gorhill/cronexpr"
	logger "sigs.k8s.io/controller-runtime/pkg/log"
)

// DatabaseChaosReconciler reconciles a VirtualDatabase object
type DatabaseChaosReconciler struct {
	client.Client
	Scheme *runtime.Scheme
}

func (r *DatabaseChaosReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	log := logger.FromContext(ctx)

	rt, err := r.getRuntimeDatabaseChaos(ctx, req.NamespacedName)
	if apierrors.IsNotFound(err) {
		log.Info("Resource in work queue no longer exists!")
		return ctrl.Result{}, nil
	} else if err != nil {
		log.Error(err, "Error getting CRD resource")
		return ctrl.Result{}, err
	}

	return r.reconcile(ctx, req, rt)
}

func (r *DatabaseChaosReconciler) reconcile(ctx context.Context, req ctrl.Request, rt *v1alpha1.DatabaseChaos) (ctrl.Result, error) {
	log := logger.FromContext(ctx)
	if !rt.Spec.Suspend {
		vdblist := &v1alpha1.VirtualDatabaseList{}
		lbs := labels.Set{}
		lbs = rt.Spec.Selector.MatchLabels
		opts := &client.ListOptions{
			LabelSelector: lbs.AsSelector(),
		}
		if err := r.List(ctx, vdblist, opts); err != nil {
			return ctrl.Result{Requeue: true}, err
		}

		nxt := cronexpr.MustParse(rt.Spec.Schedule).Next(time.Now())
		for _, vdb := range vdblist.Items {
			dbep := &v1alpha1.DatabaseEndpoint{}
			if err := r.Get(ctx, types.NamespacedName{
				Namespace: vdb.Namespace,
				Name:      vdb.Name,
			}, dbep); err != nil {
				log.Error(err, "Getting DatabaseEndpoint error")
				continue
			}

			if dbep.Annotations == nil {
				dbep.Annotations = map[string]string{}
			}

			if len(dbep.Annotations[v1alpha1.AnnotationsNextScheduledChaosTimestamp]) != 0 && len(dbep.Status.Arn) != 0 {
				expectedSchedule, err := time.Parse(time.RFC3339, dbep.Annotations[v1alpha1.AnnotationsNextScheduledChaosTimestamp])
				if err != nil {
					log.Error(err, "Schedule parse error")
					continue
				}

				if time.Duration(time.Now().Sub(expectedSchedule).Seconds()) < 30*time.Second {
					region := os.Getenv("AWS_REGION")
					accessKey := os.Getenv("AWS_ACCESS_KEY")
					secretAccessKey := os.Getenv("AWS_SECRET_ACCESS_KEY")
					sess := aws.NewSessions().SetCredential(region, accessKey, secretAccessKey).Build()
					names := strings.Split(dbep.Status.Arn, ":")
					name := names[len(names)-1]
					// TODO: try using ARN
					rdsDesc, err := rds.NewService(sess[region]).Instance().SetDBInstanceIdentifier(name).Describe(ctx)
					if err != nil {
						log.Error(err, "Desc RDS error")
						continue
					}

					if rdsDesc.DBInstanceStatus == "available" || rdsDesc.DBInstanceStatus == "Available" {
						err := rds.NewService(sess[region]).Instance().SetDBInstanceIdentifier(name).Reboot(context.TODO())
						if err != nil {
							log.Error(err, "Reboot RDS error")
							continue
						}
					}

				}
			}

			dbep.Annotations[v1alpha1.AnnotationsNextScheduledChaosTimestamp] = nxt.Format(time.RFC3339)
			if err := r.Update(ctx, dbep); err != nil {
				return ctrl.Result{Requeue: true}, err
			}
		}
		// TODO: add DatabaseChaos status handle
		return ctrl.Result{RequeueAfter: nxt.Sub(time.Now())}, nil

	} else {
		return ctrl.Result{RequeueAfter: ReconcileTime}, nil
	}
}

func (r *DatabaseChaosReconciler) getRuntimeDatabaseChaos(ctx context.Context, namespacedName types.NamespacedName) (*v1alpha1.DatabaseChaos, error) {
	rt := &v1alpha1.DatabaseChaos{}
	err := r.Get(ctx, namespacedName, rt)
	return rt, err
}

// SetupWithManager sets up the controller with the Manager.
func (r *DatabaseChaosReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&v1alpha1.DatabaseChaos{}).
		Complete(r)
}
