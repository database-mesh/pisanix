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

// NOTE: json tags are required.  Any new fields you add must have json tags for the fields to be serialized.

// TrafficQoSMappingSpec defines the desired state of TrafficQoSMapping
type TrafficQoSMappingSpec struct {
	References map[string]string `json:"references`
}

// TrafficQoSMappingStatus defines the observed state of TrafficQoSMapping
type TrafficQoSMappingStatus struct {
	//TODO: add ObservedGeneration
}

//+kubebuilder:object:root=true
//+kubebuilder:subresource:status

// TrafficQoSMapping is the Schema for the trafficqos API
type TrafficQoSMapping struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec TrafficQoSMappingSpec `json:"spec,omitempty"`
}

//+kubebuilder:object:root=true

// TrafficQoSMappingList contains a list of TrafficQoSMapping
type TrafficQoSMappingList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []TrafficQoSMapping `json:"items"`
}

func init() {
	SchemeBuilder.Register(&TrafficQoSMapping{}, &TrafficQoSMappingList{})
}
