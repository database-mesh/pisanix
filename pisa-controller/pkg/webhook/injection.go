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
	"k8s.io/apimachinery/pkg/types"
)

var (
	runtimeScheme = runtime.NewScheme()
	codecs        = serializer.NewCodecFactory(runtimeScheme)
	deserializer  = codecs.UniversalDeserializer()
)

func Injection(ctx *gin.Context) {
	rawData, err := ctx.GetRawData()
	if err != nil || len(rawData) == 0 {
		log.Error("get body raw data error.")
		ctx.JSON(http.StatusBadRequest, NewV1AdmissionResponseFromError(err))
		return
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
	pod := retrievePodFromAdmissionRequest(ar.Request)
	if pod == nil {
		return errors.New("retrieve pod from admission request error")
	}

	patch := buildPatch(pod)

	resp := buildPodPatchResponse(ar.Request.UID, pod, patch)
	if resp != nil {
		return errors.New("build pod patch response error")
	}

	ar.Response = resp
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
		log.Errorf("retrieve from raw object error: %s", err)
		return nil
	}

	return pod
}

//TODO: fix the fields
func buildPatch(pod *corev1.Pod) string {
	cm := &PisaControllerInjectionMeta{}
	cm.SetNamespaceFromEnv().SetServiceFromEnv()
	pm := &PisaProxyInjectionMeta{}
	pm.SetImageFromEnv().SetAdminListenHostFromEnv().SetAdminListenPortFromEnv().SetAdminLogLevelFromEnv()

	var pisaProxyDeployedName string
	if pod.OwnerReferences == nil || pod.OwnerReferences[0].Kind == "" {
		pisaProxyDeployedName = pod.ObjectMeta.Name
	} else {
		pisaProxyDeployedName = getPisaProxyDeployedNameFromPod(pod.OwnerReferences[0].Kind, pod.ObjectMeta.GenerateName)
	}

	return fmt.Sprintf(podsSidecarPatch,
		pm.Image,
		pisaProxyContainerName,
		pm.AdminListenPort,
		cm.Service,
		cm.Namespace,
		pisaProxyDeployedName,
		pm.AdminListenHost,
		pm.AdminListenPort,
		pm.AdminLogLevel,
	)
}

type PisaControllerInjectionMeta struct {
	Service   string
	Namespace string
}

func (m *PisaControllerInjectionMeta) BuildEnvs() []corev1.EnvVar {
	var envs []corev1.EnvVar
	return envs
}

func (m *PisaControllerInjectionMeta) SetServiceFromEnv() *PisaControllerInjectionMeta {
	if m.Service = os.Getenv(EnvPisaControllerService); m.Service == "" {
		m.Service = DefaultPisaControllerService
	}
	return m
}

func (m *PisaControllerInjectionMeta) SetNamespaceFromEnv() *PisaControllerInjectionMeta {
	if m.Namespace = os.Getenv(EnvPisaControllerNamespace); m.Namespace == "" {
		m.Namespace = DefaultPisaControllerNamespace
	}
	return m
}

type PisaProxyInjectionMeta struct {
	Image           string
	AdminListenHost string
	AdminListenPort uint32
	AdminLogLevel   string
}

func (m *PisaProxyInjectionMeta) BuildEnvs() []corev1.EnvVar {
	var envs []corev1.EnvVar
	return envs
}

func (m *PisaProxyInjectionMeta) SetImageFromEnv() *PisaProxyInjectionMeta {
	if m.Image = os.Getenv(EnvPisaProxyImage); m.Image == "" {
		m.Image = DefaultPisaProxyImage
	}
	return m
}

func (m *PisaProxyInjectionMeta) SetAdminListenHostFromEnv() *PisaProxyInjectionMeta {
	if host := os.Getenv(EnvPisaProxyAdminListenHost); host == "" {
		m.AdminListenHost = DefaultPisaProxyAdminListenHost
	} else {
		m.AdminListenHost = host
	}
	return m
}

func (m *PisaProxyInjectionMeta) SetAdminListenPortFromEnv() *PisaProxyInjectionMeta {
	if port, err := strconv.Atoi(os.Getenv(EnvPisaProxyAdminListenPort)); port <= 0 || err != nil {
		m.AdminListenPort = DefaultPisaProxyAdminListenPort
	} else {
		m.AdminListenPort = uint32(port)
	}
	return m
}

func (m *PisaProxyInjectionMeta) SetAdminLogLevelFromEnv() *PisaProxyInjectionMeta {
	if lv := os.Getenv(EnvPisaProxyAdminLoglevel); lv == "" {
		m.AdminLogLevel = DefaultPisaProxyAdminLoglevel
	} else {
		m.AdminLogLevel = lv
	}
	return m
}

type PisaProxyInjection struct {
	PisaProxyInjection PisaProxyInjectionMeta
	PisaController     PisaControllerInjectionMeta
}

// func getInjections() {

// }

func getPisaProxyDeployedNameFromPod(kind, generatedName string) string {
	var name string

	switch kind {
	case "ReplicaSet":
		fallthrough
	case "Job":
		name = getPodNameFromGeneratedName(generatedName, 2)
	case "StatefulSet":
		fallthrough
	case "DaemonSet":
		name = getPodNameFromGeneratedName(generatedName, 1)
	}
	return name
}

func getPodNameFromGeneratedName(generatedName string, offset uint) string {
	podSlice := strings.Split(generatedName, "-")
	l := len(podSlice)

	if l > int(offset) {
		podSlice = podSlice[:l-int(offset)]
		return strings.Join(podSlice, "-")
	}
	return generatedName
}

func buildPodPatchResponse(UID types.UID, pod *corev1.Pod, patch string) *v1.AdmissionResponse {
	log.Info("mutating pods")

	if !hasContainer(pod.Spec.Containers, pisaProxyContainerName) {
		resp := &v1.AdmissionResponse{
			UID:     UID,
			Allowed: true,
			Patch:   []byte(patch),
		}

		pt := v1.PatchTypeJSONPatch
		resp.PatchType = &pt
		return resp
	}

	return nil
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
