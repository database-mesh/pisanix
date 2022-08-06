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

package webhook

import (
	"encoding/json"
	"errors"
	"strconv"
	"testing"

	"github.com/stretchr/testify/assert"
	v1 "k8s.io/api/admission/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

func Test_GetPisaProxyDeployedNameFromPod(t *testing.T) {
	cases := []struct {
		name          string
		generatedName string
		kind          string
		exp           string
		msg           string
	}{
		{
			name:          "a-b-c-6668ddbf55-abcde",
			generatedName: "a-b-c-6668ddbf55-",
			kind:          "ReplicaSet",
			exp:           "a-b-c",
			msg:           "Deployment or CronJob name should be equal",
		},
		{
			name:          "a-b-0",
			generatedName: "a-b-",
			kind:          "StatefulSet",
			exp:           "a-b",
			msg:           "StatefulSet, Job or DaemonSet name should be equal",
		},
	}

	for _, c := range cases {
		assert.Equal(t, c.exp, getPisaProxyDeployedNameFromPod(c.kind, c.generatedName), c.msg)
	}
}

func Test_injection(t *testing.T) {
	cases := []struct {
		rev       *v1.AdmissionReview
		msg       string
		err       error
		errString string
	}{
		{
			rev: &v1.AdmissionReview{
				Request: &v1.AdmissionRequest{
					Resource: metav1.GroupVersionResource{
						Group:    "",
						Version:  "v1",
						Resource: "svc",
					},
				},
			},
			msg:       "wrong gvr should be error",
			err:       errors.New("retrieve pod from admission request error"),
			errString: "retrieve pod from admission request error",
		},
		{
			rev: &v1.AdmissionReview{
				Request: &v1.AdmissionRequest{
					Resource: metav1.GroupVersionResource{
						Group:    "",
						Version:  "v1",
						Resource: "pods",
					},
				},
			},
			msg:       "duplicated sidecar cause error",
			err:       errors.New("build pod patch response error"),
			errString: "build pod patch response error",
		},
	}

	for _, c := range cases {
		assert.EqualError(t, c.err, c.errString, c.msg)
	}
}

const pod = `{
	"apiVersion": "v1",
	"kind": "Pod",
	"metadata": {
	  "labels": {
		"app": "nginx"
	  },
	  "name": "nginx",
	  "namespace": "temp"
	},
	"spec": {
	  "containers": [
		{
		  "image": "nginx:1.14.2",
		  "imagePullPolicy": "IfNotPresent",
		  "name": "nginx",
		  "ports": [
			{
			  "containerPort": 80,
			  "protocol": "TCP"
			}
		  ]
		},
		{
		  "image": "pisa-proxy: latest",
		  "name": "pisa-proxy",
		  "ports": [
			{
			  "containerPort": 5591,
			  "protocol": "TCP"
			}
		  ]
		}
	  ]
	}
  }`

func Test_retrievePodFromAdmissionRequest(t *testing.T) {
	cases := []struct {
		req *v1.AdmissionRequest
		msg string
	}{
		{
			req: &v1.AdmissionRequest{
				Resource: metav1.GroupVersionResource{
					Group:    "",
					Version:  "v1",
					Resource: "pods",
				},
			},
			msg: "gvr should be equal",
		},
		{
			req: &v1.AdmissionRequest{
				Resource: metav1.GroupVersionResource{
					Group:    "",
					Version:  "v1",
					Resource: "pods",
				},
				Object: runtime.RawExtension{
					Raw: []byte(
						pod,
					),
				},
			},
			msg: "decode should be succeed",
		},
	}

	for _, c := range cases {
		assert.NotNil(t, retrievePodFromAdmissionRequest(c.req), c.msg)
	}
}

type patch struct {
	OP    string            `json:"op"`
	Path  string            `json:"path"`
	Value *corev1.Container `json:"value"`
}

func Test_buildPatch(t *testing.T) {
	cases := []struct {
		pod   *corev1.Pod
		patch *patch
	}{
		{
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:         "pod",
					GenerateName: "a-b-c-12345678-",
				},
			},
			patch: &patch{
				Value: &corev1.Container{
					Env: []corev1.EnvVar{
						{Name: EnvPisaControllerService, Value: DefaultPisaControllerService},
						{Name: EnvPisaControllerNamespace, Value: DefaultPisaControllerNamespace},
						{Name: EnvPisaProxyDeployedNamespace, ValueFrom: &corev1.EnvVarSource{
							FieldRef: &corev1.ObjectFieldSelector{
								APIVersion: "v1",
								FieldPath:  "metadata.namespace",
							},
						}},
						{Name: EnvPisaProxyDeployedName, Value: "pod"},
						{Name: EnvPisaProxyAdminListenHost, Value: DefaultPisaProxyAdminListenHost},
						{Name: EnvPisaProxyAdminListenPort, Value: strconv.Itoa(DefaultPisaProxyAdminListenPort)},
						{Name: EnvPisaProxyAdminLoglevel, Value: DefaultPisaProxyAdminLoglevel},
					},
				},
			},
		},
		{
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name: "pod",
					OwnerReferences: []metav1.OwnerReference{
						{
							Kind: "ReplicaSet",
						},
					},
					GenerateName: "a-b-c-12345678-",
				},
			},
			patch: &patch{
				Value: &corev1.Container{
					Env: []corev1.EnvVar{
						{Name: EnvPisaControllerService, Value: DefaultPisaControllerService},
						{Name: EnvPisaControllerNamespace, Value: DefaultPisaControllerNamespace},
						{Name: EnvPisaProxyDeployedNamespace, ValueFrom: &corev1.EnvVarSource{
							FieldRef: &corev1.ObjectFieldSelector{
								APIVersion: "v1",
								FieldPath:  "metadata.namespace",
							},
						}},
						{Name: EnvPisaProxyDeployedName, Value: "a-b-c"},
						{Name: EnvPisaProxyAdminListenHost, Value: DefaultPisaProxyAdminListenHost},
						{Name: EnvPisaProxyAdminListenPort, Value: strconv.Itoa(DefaultPisaProxyAdminListenPort)},
						{Name: EnvPisaProxyAdminLoglevel, Value: DefaultPisaProxyAdminLoglevel},
					},
				},
			},
		},
		{
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name: "pod",
					OwnerReferences: []metav1.OwnerReference{
						{
							Kind: "StatefulSet",
						},
					},
					GenerateName: "a-b-",
				},
			},
			patch: &patch{
				Value: &corev1.Container{
					Env: []corev1.EnvVar{
						{Name: EnvPisaControllerService, Value: DefaultPisaControllerService},
						{Name: EnvPisaControllerNamespace, Value: DefaultPisaControllerNamespace},
						{Name: EnvPisaProxyDeployedNamespace, ValueFrom: &corev1.EnvVarSource{
							FieldRef: &corev1.ObjectFieldSelector{
								APIVersion: "v1",
								FieldPath:  "metadata.namespace",
							},
						}},
						{Name: EnvPisaProxyDeployedName, Value: "a-b"},
						{Name: EnvPisaProxyAdminListenHost, Value: DefaultPisaProxyAdminListenHost},
						{Name: EnvPisaProxyAdminListenPort, Value: strconv.Itoa(DefaultPisaProxyAdminListenPort)},
						{Name: EnvPisaProxyAdminLoglevel, Value: DefaultPisaProxyAdminLoglevel},
					},
				},
			},
		},
	}

	for _, c := range cases {
		pl := []patch{{Value: &corev1.Container{}}}
		err := json.Unmarshal([]byte(buildPatch(c.pod)), &pl)
		assert.NoError(t, err, "unmarshal should not be error")
		assert.ElementsMatch(t, c.patch.Value.Env, pl[0].Value.Env, "sidecar env should be equal[")
	}
}
