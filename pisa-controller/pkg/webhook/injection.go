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
	"errors"
	"fmt"
	"net/http"
	"os"
	"strconv"
	"strings"

	"github.com/gin-gonic/gin"
	log "github.com/sirupsen/logrus"
	v1 "k8s.io/api/admission/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/serializer"
	"k8s.io/apimachinery/pkg/util/json"
)

var (
	pisaProxyImage, pisaControllerService, pisaControllerNamespace, pisaProxyAdminListenHost, pisaProxyLoglevel string
	pisaProxyAdminListenPort                                                                                    uint32
)

const (
	pisaProxyContainerName = "pisa-proxy"

	EnvPisaProxyAdminListenHost = "PISA_PROXY_ADMIN_LISTEN_HOST"
	EnvPisaProxyAdminListenPort = "PISA_PROXY_ADMIN_LISTEN_PORT"
	EnvPisaProxyLoglevel        = "PISA_PROXY_ADMIN_LOG_LEVEL"
	EnvPisaProxyImage           = "PISA_PROXY_IMAGE"
	EnvPisaControllerService    = "PISA_CONTROLLER_SERVIE"
	EnvPisaControllerNamespace  = "PISA_CONTROLLER_NAMESPACE"

	DefaultPisaProxyAdminListenHost = "0.0.0.0"
	DefaultPisaProxyAdminListenPort = 5591
	DefaultPisaProxyLoglevel        = "INFO"
	DefaultPisaProxyImage           = "pisanixio/proxy:latest"
	DefaultPisaControllerService    = "default"
	DefaultPisaControllerNamespace  = "default"
)

func init() {
	if pisaProxyImage = os.Getenv(EnvPisaProxyImage); pisaProxyImage == "" {
		pisaProxyImage = DefaultPisaProxyImage
	}

	if pisaControllerService = os.Getenv(EnvPisaControllerService); pisaControllerService == "" {
		pisaControllerService = DefaultPisaControllerService
	}
	if pisaControllerNamespace = os.Getenv(EnvPisaControllerNamespace); pisaControllerNamespace == "" {
		pisaControllerNamespace = DefaultPisaControllerNamespace
	}

	if host := os.Getenv(EnvPisaProxyAdminListenHost); host == "" {
		pisaProxyAdminListenHost = DefaultPisaProxyAdminListenHost
	} else {
		pisaProxyAdminListenHost = host
	}

	if port, err := strconv.Atoi(os.Getenv(EnvPisaProxyAdminListenPort)); port <= 0 || err != nil {
		pisaProxyAdminListenPort = DefaultPisaProxyAdminListenPort
	} else {
		pisaProxyAdminListenPort = uint32(port)
	}

	if lv := os.Getenv(EnvPisaProxyLoglevel); lv == "" {
		pisaProxyLoglevel = DefaultPisaProxyLoglevel
	} else {
		pisaProxyLoglevel = lv
	}
}

const (
	podsSidecarPatch = `[
		{
			"op":"add", 
			"path":"/spec/containers/-",
			"value":{
				"image":"%v",
				"name":"%s",
				"args": ["sidecar"],
				"ports": [
					{
						"containerPort": %d,
						"name": "pisa-admin",
						"protocol": "TCP"
					}	
				],
				"resources":{},
				"env": [
					{
						"name": "PISA_CONTROLLER_SERVICE",
						"value": "%s"
					},{
						"name": "PISA_CONTROLLER_NAMESPACE",
						"value": "%s"
					},{
						"name": "PISA_DEPLOYED_NAMESPACE",
						"valueFrom": {
                            "fieldRef": {
                                "apiVersion": "v1",
                                "fieldPath": "metadata.namespace"
                            }
                        }
					},{
						"name": "PISA_DEPLOYED_NAME",
						"value": "%s"
					},{
						"name": "PISA_PROXY_ADMIN_LISTEN_HOST",
						"value": "%s"
					},{
						"name": "PISA_PROXY_ADMIN_LISTEN_PORT",
						"value": "%d"
					},{
						"name": "PISA_PROXY_ADMIN_LOG_LEVEL",
						"value": "%s"
					}
				]
			}
		}
	]`
)

var (
	runtimeScheme = runtime.NewScheme()
	codecs        = serializer.NewCodecFactory(runtimeScheme)
	deserializer  = codecs.UniversalDeserializer()
)

type PodInfo struct {
	Metadata struct {
		GenerateName string `json:"generateName"`
	} `json:"metadata"`
}

func InjectSidecar(ctx *gin.Context) {
	rawData, err := ctx.GetRawData()
	if err != nil || len(rawData) == 0 {
		log.Error("get body raw data error.")
		ctx.JSON(http.StatusBadRequest, NewV1AdmissionResponseFromError(err))
	}

	ar := &v1.AdmissionReview{}
	if _, _, err := deserializer.Decode(rawData, nil, ar); err != nil {
		log.Errorf("can not decode body to AdmissionReview: %v", err)
		ctx.JSON(http.StatusBadRequest, NewV1AdmissionResponseFromError(err))
		return
	}

	if err = injection(ar); err != nil {
		log.Errorf("injection error: %v", err)
		ctx.JSON(http.StatusInternalServerError, NewV1AdmissionResponseFromError(err))
		return
	}

	log.Infof("mutating Success %s/%s", ar.Request.Namespace, ar.Request.Name)
	ctx.JSON(http.StatusOK, ar)
}

func injection(ar *v1.AdmissionReview) error {
	podinfo := &PodInfo{}
	err := json.Unmarshal(ar.Request.Object.Raw, podinfo)
	if err != nil {
		return err
	}

	//FIXME: Considering only the Pod whose name contains two segments as suffix.
	// e.g. the Pod of Deployment
	podSlice := strings.Split(podinfo.Metadata.GenerateName, "-")
	podSlice = podSlice[:len(podSlice)-2]

	patch := fmt.Sprintf(podsSidecarPatch,
		pisaProxyImage,
		pisaProxyContainerName,
		pisaProxyAdminListenPort,
		pisaControllerService,
		pisaControllerNamespace,
		strings.Join(podSlice, "-"),
		pisaProxyAdminListenHost,
		pisaProxyAdminListenPort,
		pisaProxyLoglevel,
	)

	if err = applyPodPatch(ar, patch); err != nil {
		return err
	}

	return nil
}

func applyPodPatch(ar *v1.AdmissionReview, patch string) error {
	log.Info("mutating pods")

	pod := retrievePodFromAdmissionRequest(ar.Request)
	if pod == nil {
		return errors.New("retrieve pod from admission request error")
	}

	log.Infof("pod %v", pod)

	reviewResponse := &v1.AdmissionResponse{
		UID:     ar.Request.UID,
		Allowed: true,
	}

	if !hasContainer(pod.Spec.Containers, pisaProxyContainerName) {
		reviewResponse.Patch = []byte(patch)
		pt := v1.PatchTypeJSONPatch
		reviewResponse.PatchType = &pt
	}

	ar.Response = reviewResponse

	return nil
}

func retrievePodFromAdmissionRequest(req *v1.AdmissionRequest) *corev1.Pod {
	gvr := metav1.GroupVersionResource{Group: "", Version: "v1", Resource: "pods"}
	if req.Resource != gvr {
		log.Errorf("expect resource to be %s", gvr)
		return nil
	}

	pod := &corev1.Pod{}
	if _, _, err := deserializer.Decode(req.Object.Raw, nil, pod); err != nil {
		log.Error(err)
		return nil
	}

	return pod
}

func NewV1AdmissionResponseFromError(err error) *v1.AdmissionResponse {
	return &v1.AdmissionResponse{
		Result: &metav1.Status{
			Message: err.Error(),
		},
	}
}

func hasContainer(containers []corev1.Container, containerName string) bool {
	for _, container := range containers {
		if container.Name == containerName {
			return true
		}
	}
	return false
}
