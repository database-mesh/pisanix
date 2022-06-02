# Introduction

## Pisa-Controller
A Golang control plane designed for sidecar injection and configuration transformation

## Feature
### auto-injection
By labeling some resources in Kubernetes, Pisa-Controller can achieve automatic injection and start the Pisa-Proxy as a sidecar with business programs

## configuration management
Use kubernetes CustomResourceDefinitions for configuration file matching and storage, and combine and deliver configuration files when the Pisa-Proxy starts
