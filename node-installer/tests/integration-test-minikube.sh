#!/bin/bash
set -euo pipefail

: ${IMAGE_NAME:=ghcr.io/spinkube/containerd-shim-spin/node-installer:dev}

echo "=== Step 1: Create a MiniKube cluster ==="
minikube start -p minikube --driver=docker --container-runtime=containerd

echo "=== Step 2: Create namespace and deploy RuntimeClass ==="
kubectl create namespace kwasm || true
kubectl apply -f ./tests/workloads/runtime.yaml

echo "=== Step 3: Build and deploy the KWasm node installer ==="
if ! docker image inspect $IMAGE_NAME >/dev/null 2>&1; then
  echo "Building node installer image..."
  IMAGE_NAME=$IMAGE_NAME make build-dev-installer-image
fi

echo "Loading node installer image into MiniKube..."
minikube image load $IMAGE_NAME -p minikube

NODE_NAME=$(kubectl get nodes --context=minikube -o jsonpath='{.items[0].metadata.name}')
cp ./tests/workloads/kwasm-job.yml minikube-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm/minikube-provision-kwasm/g" minikube-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm-dev/minikube-provision-kwasm-dev/g" minikube-kwasm-job.yml
sed -i "s/spin-test-control-plane/${NODE_NAME}/g" minikube-kwasm-job.yml

echo "Applying KWasm node installer job..."
kubectl apply -f ./minikube-kwasm-job.yml

echo "Waiting for node installer job to complete..."
kubectl wait -n kwasm --for=condition=Ready pod --selector=job-name=minikube-provision-kwasm --timeout=90s || true
kubectl wait -n kwasm --for=jsonpath='{.status.phase}'=Succeeded pod --selector=job-name=minikube-provision-kwasm --timeout=60s

if ! kubectl get pods -n kwasm | grep -q "minikube-provision-kwasm.*Completed"; then
  echo "Node installer job failed!"
  kubectl logs -n kwasm $(kubectl get pods -n kwasm -o name | grep minikube-provision-kwasm)
  exit 1
fi

echo "=== Step 4: Apply and test the workload ==="
make deploy-workload
if make test-workload; then
  echo "=== Integration Test Passed! ==="
  minikube delete -p minikube
  exit 0
else
  echo "=== Integration Test Failed! ==="
  echo "Could not get a successful response from the workload."
  kubectl describe pods
  kubectl logs $(kubectl get pods -o name | grep wasm-spin)
  minikube delete -p minikube
  exit 1
fi 