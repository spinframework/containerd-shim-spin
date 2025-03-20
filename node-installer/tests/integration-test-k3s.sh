#!/bin/bash
set -euo pipefail

: ${IMAGE_NAME:=ghcr.io/spinkube/containerd-shim-spin/node-installer:dev}

echo "Installing K3s..."
curl -sfL https://get.k3s.io | INSTALL_K3S_EXEC="--disable=traefik --write-kubeconfig-mode=644" sh -

echo "Waiting for K3s to be ready..."
sleep 10
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
until kubectl get nodes | grep -q " Ready"; do
  echo "Waiting for node to be ready..."
  sleep 5
done

echo "=== Step 2: Create namespace and deploy RuntimeClass ==="
kubectl create namespace kwasm || true
kubectl apply -f ./tests/workloads/runtime.yaml

echo "=== Step 3: Build and deploy the KWasm node installer ==="
if ! docker image inspect $IMAGE_NAME >/dev/null 2>&1; then
  echo "Building node installer image..."
  IMAGE_NAME=$IMAGE_NAME make build-dev-installer-image
fi

echo "Loading node installer image into K3s..."
docker save $IMAGE_NAME > node-installer.tar
sudo k3s ctr images import node-installer.tar
rm node-installer.tar

NODE_NAME=$(kubectl get nodes -o jsonpath='{.items[0].metadata.name}')
cp ./tests/workloads/kwasm-job.yml k3s-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm/k3s-provision-kwasm/g" k3s-kwasm-job.yml
sed -i "s/spin-test-control-plane-provision-kwasm-dev/k3s-provision-kwasm-dev/g" k3s-kwasm-job.yml
sed -i "s/spin-test-control-plane/${NODE_NAME}/g" k3s-kwasm-job.yml

echo "Applying KWasm node installer job..."
kubectl apply -f ./k3s-kwasm-job.yml

echo "Waiting for node installer job to complete..."
kubectl wait -n kwasm --for=condition=Ready pod --selector=job-name=k3s-provision-kwasm --timeout=90s || true
kubectl wait -n kwasm --for=jsonpath='{.status.phase}'=Succeeded pod --selector=job-name=k3s-provision-kwasm --timeout=60s

if ! kubectl get pods -n kwasm | grep -q "k3s-provision-kwasm.*Completed"; then
  echo "Node installer job failed!"
  kubectl logs -n kwasm $(kubectl get pods -n kwasm -o name | grep k3s-provision-kwasm)
  exit 1
fi

echo "=== Step 4: Apply and test the workload ==="
sudo k3s ctr images pull ghcr.io/spinkube/containerd-shim-spin/examples/spin-rust-hello:v0.18.0
make deploy-workload
if make test-workload; then
  echo "=== Integration Test Passed! ==="
  sudo /usr/local/bin/k3s-uninstall.sh 
  sudo rm -rf /etc/rancher/k3s
  sudo rm -rf /var/lib/rancher/k3s
  exit 0
else
  echo "=== Integration Test Failed! ==="
  echo "Could not get a successful response from the workload."
  kubectl describe pods
  kubectl logs $(kubectl get pods -o name | grep wasm-spin)
  sudo /usr/local/bin/k3s-uninstall.sh 
  sudo rm -rf /etc/rancher/k3s
  sudo rm -rf /var/lib/rancher/k3s
  exit 1
fi 