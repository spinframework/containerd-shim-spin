#!/bin/bash
set -euo pipefail

: ${IMAGE_NAME:=ghcr.io/spinframework/containerd-shim-spin/node-installer:dev}

echo "=== Step 1: Create a kind cluster ==="
if kind get clusters | grep -q "spin-test"; then
  echo "Deleting existing cluster..."
  kind delete cluster --name spin-test
fi

echo "Creating kind cluster..."
kind create cluster --config - << EOF
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
name: spin-test
nodes:
- role: control-plane
  extraPortMappings:
  - containerPort: 80
    hostPort: 8080
    protocol: TCP
EOF
kubectl --context=kind-spin-test wait --for=condition=Ready nodes --all --timeout=90s

echo "=== Step 2: Create namespace and deploy RuntimeClass ==="
kubectl --context=kind-spin-test create namespace kwasm || true
kubectl --context=kind-spin-test apply -f ./tests/workloads/runtime.yaml

echo "=== Step 3: Build and deploy the KWasm node installer ==="
if ! docker image inspect $IMAGE_NAME >/dev/null 2>&1; then
  echo "Building node installer image..."
  IMAGE_NAME=$IMAGE_NAME make build-dev-installer-image
fi

echo "Loading node installer image into kind..."
kind load docker-image $IMAGE_NAME --name spin-test

echo "Applying KWasm node installer job..."
kubectl --context=kind-spin-test apply -f ./tests/workloads/kwasm-job.yml

echo "Waiting for node installer job to complete..."
kubectl --context=kind-spin-test wait -n kwasm --for=condition=Ready pod --selector=job-name=spin-test-control-plane-provision-kwasm --timeout=90s || true
kubectl --context=kind-spin-test wait -n kwasm --for=jsonpath='{.status.phase}'=Succeeded pod --selector=job-name=spin-test-control-plane-provision-kwasm --timeout=60s

# Verify the SystemdCgroup is set to true
if docker exec spin-test-control-plane cat /etc/containerd/config.toml | grep -A5 "spin" | grep -q "SystemdCgroup = true"; then
  echo "SystemdCgroup is set to true"
else
  echo "SystemdCgroup is not set to true"
  exit 1
fi

if ! kubectl --context=kind-spin-test get pods -n kwasm | grep -q "spin-test-control-plane-provision-kwasm.*Completed"; then
  echo "Node installer job failed!"
  kubectl --context=kind-spin-test logs -n kwasm $(kubectl --context=kind-spin-test get pods -n kwasm -o name | grep spin-test-control-plane-provision-kwasm)
  exit 1
fi

echo "=== Step 4: Apply the workload ==="
kubectl --context=kind-spin-test apply -f ./tests/workloads/workload.yaml

echo "Waiting for deployment to be ready..."
kubectl --context=kind-spin-test wait --for=condition=Available deployment/wasm-spin --timeout=120s

echo "Checking pod status..."
kubectl --context=kind-spin-test get pods

echo "=== Step 5: Test the workload ==="
echo "Waiting for service to be ready..."
sleep 10

echo "Testing workload with curl..."
MAX_RETRIES=5
RETRY_COUNT=0
SUCCESS=false

while [ $RETRY_COUNT -lt $MAX_RETRIES ] && [ "$SUCCESS" = false ]; do
  kubectl --context=kind-spin-test port-forward svc/wasm-spin 8888:80 &
  FORWARD_PID=$!
  sleep 5

  if curl -s http://localhost:8888/hello | grep -q "Hello world from Spin!"; then
    SUCCESS=true
    echo "Workload test successful!"
  else
    echo "Retrying in 3 seconds..."
    sleep 3
    RETRY_COUNT=$((RETRY_COUNT+1))
  fi
  kill $FORWARD_PID 2>/dev/null || true
done

if [ "$SUCCESS" = true ]; then
  echo "=== Integration Test Passed! ==="
  kind delete cluster --name spin-test
  exit 0
else
  echo "=== Integration Test Failed! ==="
  echo "Could not get a successful response from the workload."
  kubectl --context=kind-spin-test describe pods
  kubectl --context=kind-spin-test logs $(kubectl --context=kind-spin-test get pods -o name | grep wasm-spin)
  kind delete cluster --name spin-test
  exit 1
fi 