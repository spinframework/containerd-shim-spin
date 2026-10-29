#!/bin/bash
set -euo pipefail
shopt -s expand_aliases

: ${IMAGE_NAME:=ghcr.io/spinframework/containerd-shim-spin/node-installer:dev}

echo "=== Step 1: Setup MicroK8s ==="
if ! command -v microk8s >/dev/null 2>&1; then
  echo "MicroK8s is not installed. Please install it first."
  exit 1
fi

# CI installs a fresh cluster; only reset pre-existing local test clusters.
if [[ "${CI:-false}" != "true" ]]; then
  sudo microk8s start
  sudo microk8s status --wait-ready --timeout=180
  sudo microk8s reset
fi

sudo microk8s status --wait-ready --timeout=180

sudo microk8s enable dns

alias kubectl='sudo microk8s kubectl'
kubectl version

echo "=== Step 2: Create namespace and deploy RuntimeClass ==="
kubectl create namespace kwasm || true
kubectl apply -f ./tests/workloads/runtime.yaml

echo "=== Step 3: Build and deploy the KWasm node installer ==="
if ! docker image inspect $IMAGE_NAME >/dev/null 2>&1; then
  echo "Building node installer image..."
  IMAGE_NAME=$IMAGE_NAME make build-dev-installer-image
fi

echo "Loading node installer image into MicroK8s..."
docker save $IMAGE_NAME > node-installer.tar
sudo microk8s ctr image import node-installer.tar
rm node-installer.tar

NODE_NAME=$(kubectl get nodes -o jsonpath='{.items[0].metadata.name}')
cp ./tests/workloads/kwasm-job.yml microk8s-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm/microk8s-provision-kwasm/g" microk8s-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm-dev/microk8s-provision-kwasm-dev/g" microk8s-kwasm-job.yml
sed -i "s/spin-test-control-plane/${NODE_NAME}/g" microk8s-kwasm-job.yml

echo "Applying KWasm node installer job..."
kubectl apply -f ./microk8s-kwasm-job.yml

echo "Waiting for node installer job to complete..."
kubectl wait -n kwasm --for=jsonpath='{.status.phase}'=Succeeded pod --selector=job-name=microk8s-provision-kwasm --timeout=60s

# Ensure the SystemdCgroup is not set to true
if sudo cat /var/snap/microk8s/current/args/containerd.toml | grep -A5 "spin" | grep -q "SystemdCgroup = true"; then
  echo "Failed: SystemdCgroup is set to true"
  exit 1
else
  echo "Passed: SystemdCgroup is not set to true"
fi

if ! kubectl get pods -n kwasm | grep -q "microk8s-provision-kwasm.*Completed"; then
  echo "Node installer job failed!"
  kubectl logs -n kwasm $(kubectl get pods -n kwasm -o name | grep microk8s-provision-kwasm)
  exit 1
fi

echo "=== Step 4: Apply the workload ==="
kubectl apply -f ./tests/workloads/workload.yaml

echo "Waiting for deployment to be ready..."
if ! kubectl wait --for=condition=Available deployment/wasm-spin --timeout=120s; then
  echo "Deployment failed to become ready!"
  kubectl describe deployment wasm-spin
  exit 1
fi

echo "Checking pod status..."
kubectl get pods

echo "=== Step 5: Test the workload ==="
echo "Waiting for service to be ready..."
sleep 10

echo "Testing workload with curl..."
kubectl port-forward svc/wasm-spin 8888:80 &
FORWARD_PID=$!
sleep 5

MAX_RETRIES=3
RETRY_COUNT=0
SUCCESS=false

while [ $RETRY_COUNT -lt $MAX_RETRIES ] && [ "$SUCCESS" = false ]; do
  if curl -s http://localhost:8888/hello | grep -q "Hello world from Spin!"; then
    SUCCESS=true
    echo "Workload test successful!"
  else
    echo "Retrying in 3 seconds..."
    sleep 3
    RETRY_COUNT=$((RETRY_COUNT+1))
  fi
done

kill $FORWARD_PID || true

if [ "$SUCCESS" = true ]; then
  echo "=== Integration Test Passed! ==="
  exit 0
else
  echo "=== Integration Test Failed! ==="
  echo "Could not get a successful response from the workload."
  kubectl describe pods
  kubectl logs $(kubectl get pods -o name | grep wasm-spin)
  exit 1
fi 