#! /bin/sh

export PISANIX_NAMESPACE="pisanix-system"
kubectl create namespace ${PISANIX_NAMESPACE}
kubectl create secret docker-registry uhub --docker-server=${{ secrets.DOCKERHUB_CI_HOST }} --docker-username=${{ secrets.DOCKERHUB_CI_USERNAME }} --docker-password=${{ secrets.DOCKERHUB_CI_PASSWORD }}
kubectl create secret docker-registry uhub --docker-server=${{ secrets.DOCKERHUB_CI_HOST }} --docker-username=${{ secrets.DOCKERHUB_CI_USERNAME }} --docker-password=${{ secrets.DOCKERHUB_CI_PASSWORD }} -n ${PISANIX_NAMESPACE}
kubectl label namespace default pisanix.io/inject=enabled
kubectl get namespaces --show-labels
cd charts/pisa-controller
helm dependency build
cd ..
helm install --debug pisa-controller pisa-controller -n ${PISANIX_NAMESPACE} --set image.repository=${{ secrets.DOCKERHUB_CI_HOST }}/pisanixio/controller  --set image.tag=${{ steps.vars.outputs.imagetag }} --set proxyImage.repository=${{ secrets.DOCKERHUB_CI_HOST }}/pisanixio/proxy  --set proxyImage.tag=${{ steps.vars.outputs.imagetag }} --set image.imagePullSecrets[0].name=uhub
kubectl get pods -n ${PISANIX_NAMESPACE}
kubectl describe pods -n ${PISANIX_NAMESPACE}
kubectl wait --timeout=60s --for=condition=Ready --all pod -n ${PISANIX_NAMESPACE}
kubectl -l app=pisa-controller logs -n ${PISANIX_NAMESPACE}
cd ..
cd test/kubernetes
kubectl apply -f mysql.yml
kubectl apply -f virtualdatabase.yml
kubectl apply -f trafficstrategy.yml
kubectl apply -f databaseendpoint.yml
kubectl apply -f app.yml
sleep 15
kubectl patch deployment test  -p='{"spec": {"replicas": 0, "template": {"spec": {"imagePullSecrets": [{"name": "uhub"}]}}}}'
kubectl get pod -A --show-labels
sleep 15
kubectl get pod -A --show-labels
kubectl scale deployment test --replicas=1
kubectl get pod -A --show-labels
kubectl wait --timeout=60s --for=condition=ready --all pod
kubectl -l app=nginx logs  --all-containers=true
kubectl port-forward svc/test 3308:3306 &
kubectl get pod -A --show-labels
sleep 3
