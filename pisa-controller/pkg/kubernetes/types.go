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

package kubernetes

import (
	"time"

	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// VirtualDatabaseSpec defines the desired state of VirtualDatabase
type VirtualDatabaseSpec struct {
	Services []Service `json:"services"`
}

// Service Defines the content of a VirtualDatabase
type Service struct {
	DatabaseService `json:",inline"`

	Name            string `json:"name"`
	TrafficStrategy string `json:"trafficStrategy"`
}

// DatabaseService The type of VirtualDatabase that needs to be applied for.
// Current support: databaseMySQL
type DatabaseService struct {
	DatabaseMySQL *DatabaseMySQL `json:"databaseMySQL"`
}

// DatabaseMySQL The type one of VirtualDatabase.Represents a virtual MySQL type
type DatabaseMySQL struct {
	Host     string `json:"host,omitempty"`
	Port     string `json:"port,omitempty"`
	Username string `json:"username,omitempty"`
	Password string `json:"password,omitempty"`
	DB       string `json:"db,omitempty"`
	PoolSize int    `json:"poolSize,omitempty"`
}

// VirtualDatabaseStatus defines the observed state of VirtualDatabase
// Endpoints display the name of the associated DatabaseEndpoint
// TODO: Implement dynamic updates
type VirtualDatabaseStatus struct {
	Endpoints []string `json:"endpoints"`
}

// TrafficStrategySpec defines the desired state of TrafficStrategy
type TrafficStrategySpec struct {
	Selector            *metav1.LabelSelector `json:"selector"`
	LoadBalance         *LoadBalance          `json:"loadBalance,omitempty"`
	CircuitBreaks       []CircuitBreak        `json:"circuitBreaks,omitempty"`
	ConcurrencyControls []ConcurrencyControl  `json:"ConcurrencyControls,omitempty"`
}

// LoadBalance The choice of load balancing strategy, currently supported: SimpleLoadBalancer
type LoadBalance struct {
	SimpleLoadBalancer *SimpleLoadBalancer `json:"simpleLoadBalancer"`
}

// SimpleLoadBalancer support load balancing type: 1. random 2. roundrobin
type SimpleLoadBalancer struct {
	Kind string `json:"kind"`
}

// CircuitBreak works with regular expressions.
// SQL statements that conform to regular expressions will be truncated.
type CircuitBreak struct {
	Regex string `json:"regex"`
}

// ConcurrencyControl works according to regular expressions.
// SQL statements that meet the regular conditions will be blown after the maximum concurrency limit is exceeded.
type ConcurrencyControl struct {
	Regex          string        `json:"regex"`
	Duration       time.Duration `json:"duration"`
	MaxConcurrency int           `json:"maxConcurrency"`
}

// DatabaseEndpointSpec defines the desired state of DatabaseEndpoint
type DatabaseEndpointSpec struct {
	Database Database `json:"database"`
}

// Database Backend data source type
type Database struct {
	MySQL *MySQL `json:"MySQL"`
}

// MySQL Configuration Definition
type MySQL struct {
	Name     string `json:"name,omitempty"`
	Host     string `json:"host"`
	Port     string `json:"port"`
	Username string `json:"username"`
	Password string `json:"password"`
	DB       string `json:"db"`
	Weight   int    `json:"weight"`
}
