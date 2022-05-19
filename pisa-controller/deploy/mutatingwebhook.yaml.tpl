apiVersion: admissionregistration.k8s.io/v1
kind: MutatingWebhookConfiguration
metadata:
  name: controller-webhook.pisanix.io
webhooks:
  - name: controller-webhook.pisanix.io
    clientConfig:
      service:
        name: pisa-controller
        namespace: default
        path: "/mutate"
      caBundle: __ca__
    rules:
      - apiGroups: [ "" ]
        apiVersions: [ "v1" ]
        operations: [ "CREATE" ]
        resources: [ "pods" ]
        scope: "Namespaced"
    objectSelector:
      matchLabels:
        pisanix.io/inject: enabled
    namespaceSelector:
      matchLabels:
        pisanix.io/inject: enabled
    admissionReviewVersions: ["v1"]
    sideEffects: None