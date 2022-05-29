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
	"fmt"
	"net/http"
	"os"
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

var pisanixProxyImage, pisaControllerService, pisaControllerNameSpace string

func init() {
	pisanixProxyImage = os.Getenv("PISA_PROXY_IMAGE")
	if pisanixProxyImage == "" {
		pisanixProxyImage = "pisanixio/proxy:latest"
	}
	pisaControllerService = os.Getenv("PISA_CONTROLLER_SERVICE")
	pisaControllerNameSpace = os.Getenv("PISA_CONTROLLER_NAMESPACE")
}

const (
	podsSidecarPatch = `[{"op":"add", "path":"/spec/containers/-","value":{"image":"%v","name":"pisanix-proxy","resources":{},"env": [{"name": "PISA_CONTROLLER_SERVICE","value": "%s"},{"name": "PISA_CONTROLLER_NAMESPACE","value": "%s"},{"name": "PISA_DEPLOYED_NAMESPACE","value": "%s"},{"name": "PISA_DEPLOYED_NAME","value": "%s"}]}}]`
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
	if err != nil {
		log.Error("get body raw data error.")
		ctx.JSON(http.StatusBadRequest, toV1AdmissionResponse(err))
	}
	if len(rawData) == 0 {
		log.Errorf("get a empty body!")
		return
	}

	ar := v1.AdmissionReview{}
	if _, _, err := deserializer.Decode(rawData, nil, &ar); err != nil {
		log.Errorf("can't decode body to AdmissionReview: %v", err)
		ctx.JSON(http.StatusBadRequest, toV1AdmissionResponse(err))
		return
	}
	shouldPatchPod := func(pod *corev1.Pod) bool {
		return !hasContainer(pod.Spec.Containers, "pisa-proxy")
	}
	podinfo := &PodInfo{}
	_ = json.Unmarshal(ar.Request.Object.Raw, podinfo)
	podSlice := strings.Split(podinfo.Metadata.GenerateName, "-")
	podSlice = podSlice[:len(podSlice)-2]
	ar.Response = applyPodPatch(ar, shouldPatchPod, fmt.Sprintf(podsSidecarPatch, pisanixProxyImage, pisaControllerService, pisaControllerNameSpace, ar.Request.Namespace, strings.Join(podSlice, "-")))
	log.Info("mutating Success")

	ctx.JSON(http.StatusOK, ar)
}

func applyPodPatch(ar v1.AdmissionReview, shouldPatchPod func(*corev1.Pod) bool, patch string) *v1.AdmissionResponse {
	log.Info("mutating pods")
	podResource := metav1.GroupVersionResource{Group: "", Version: "v1", Resource: "pods"}
	if ar.Request.Resource != podResource {
		log.Errorf("expect resource to be %s", podResource)
		return nil
	}

	raw := ar.Request.Object.Raw
	pod := corev1.Pod{}
	if _, _, err := deserializer.Decode(raw, nil, &pod); err != nil {
		log.Error(err)
		return toV1AdmissionResponse(err)
	}
	reviewResponse := v1.AdmissionResponse{}
	reviewResponse.UID = ar.Request.UID
	reviewResponse.Allowed = true
	if shouldPatchPod(&pod) {
		reviewResponse.Patch = []byte(patch)
		pt := v1.PatchTypeJSONPatch
		reviewResponse.PatchType = &pt
	}
	return &reviewResponse
}

func toV1AdmissionResponse(err error) *v1.AdmissionResponse {
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

func ApiCheck(ctx *gin.Context) {
	rawData, _ := ctx.GetRawData()
	ctx.JSON(http.StatusOK, rawData)
}
