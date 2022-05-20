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
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

type DatabaseServerProtocol string

const (
	DatabaseServerProtocolMySQL DatabaseServerProtocol = "MySQL"
)

type VirtualDatabaseServer struct {
	Port       int                    `json:"port"`
	Protocol   DatabaseServerProtocol `json:"protocol"`
	Credential string                 `json:"credentialName"`
	Backends   []DatabaseSource       `json:"backends"`
}

type DatabaseSource struct {
	Server     string `json:"server"`
	Port       int    `json:"port"`
	Credential string `json:"credentialName"`
}

// VirtualDatabaseSpec defines the desired state of VirtualDatabase
type VirtualDatabaseSpec struct {
	Selector  map[string]string     `json:"selector"` // TODO: Is it needed ? Or is it applied with Endpoints ?
	Server    VirtualDatabaseServer `json:"server"`
	QoS       string                `json:"qos"`
	Bandwidth string                `json:"bandwidth"`
}

// VirtualDatabaseStatus defines the observed state of VirtualDatabase
type VirtualDatabaseStatus struct {
	ClassInfo          string `json:"classInfo"`
	ObservedGeneration int64  `json:"observedGeneration,omitempty"`
}

type Pod string

//+kubebuilder:object:root=true
//+kubebuilder:subresource:status

// VirtualDatabase is the Schema for the virtualdatabases API
type VirtualDatabase struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   VirtualDatabaseSpec   `json:"spec,omitempty"`
	Status VirtualDatabaseStatus `json:"status,omitempty"`
}

//+kubebuilder:object:root=true

// VirtualDatabaseList contains a list of VirtualDatabase
type VirtualDatabaseList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []VirtualDatabase `json:"items"`
}

func init() {
	SchemeBuilder.Register(&VirtualDatabase{}, &VirtualDatabaseList{})
}
