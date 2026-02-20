#!/bin/bash

set -euo pipefail

cluster_name="test-cluster"       # name of the k3d cluster
dockerfile_path="deployments/k3d" # path to the Dockerfile

DOCKER_IMAGES=("spin" "spin-keyvalue" "spin-outbound-redis" "spin-multi-trigger-app" "spin-static-assets" "spin-mqtt-message-logger")
OUT_DIRS=("test/out_spin" "test/out_spin_keyvalue" "test/out_spin_outbound_redis" "test/out_spin_multi_trigger_app" "test/out_spin_static_assets" "test/out_spin_mqtt_message_logger")
IMAGES=("spin-hello-world" "spin-keyvalue" "spin-outbound-redis" "spin-multi-trigger-app" "spin-static-assets" "spin-mqtt-message-logger")
WKG_IMAGE="spin-hello-world"
WKG_SOURCE="spin"

# build the Docker image for the k3d cluster
docker build -t k3d-shim-test "$dockerfile_path"

k3d cluster create "$cluster_name" \
  --image k3d-shim-test --api-port 6551 -p '8082:80@loadbalancer' --agents 2 \
  --registry-create test-registry:0.0.0.0:5000 \
  --k3s-arg '--kubelet-arg=eviction-hard=imagefs.available<1%,nodefs.available<1%@agent:*' \
  --k3s-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=1%,nodefs.available=1%@agent:*'

kubectl wait --for=condition=ready node --all --timeout=120s

# Iterate through the Docker images and build them
for i in "${!DOCKER_IMAGES[@]}"; do
    docker buildx build -t "${IMAGES[$i]}:latest" "./images/${DOCKER_IMAGES[$i]}" --load
    mkdir -p "${OUT_DIRS[$i]}"
    docker save -o "${OUT_DIRS[$i]}/img.tar" "${IMAGES[$i]}:latest"
    k3d image import "${OUT_DIRS[$i]}/img.tar" -c "$cluster_name"

  ## also do spin builds and spin registry push
  ## images pushed as localhost:5000/<namespace>/<app>:<version>
  ## can be pulled as registry:5000/<namespace>/<app>:<version> from within k3d cluster
  spin build -f "./images/${DOCKER_IMAGES[$i]}/spin.toml"
  ## For the spin-static-assets app, use archive layers to test this functionality in the shim
  if [ "${i}" == "spin-static-assets" ]; then
    export SPIN_OCI_ARCHIVE_LAYERS=1
  fi
  spin registry push "localhost:5000/spin-registry-push/${IMAGES[$i]}:latest" -f "./images/${DOCKER_IMAGES[$i]}/spin.toml" -k
done

# Build with cargo and push with wkg the simple Spin hello world app. Use ttl.sh since wkg does not support localhost registries with http. See https://github.com/bytecodealliance/wasm-pkg-tools/issues/149.
TEMP_DIR=$( mktemp -d )
cargo build --release --target=wasm32-wasip1 --manifest-path ./images/${WKG_SOURCE}/Cargo.toml --target-dir "$TEMP_DIR"
wkg oci push "ttl.sh/containerd-shim-tests/wkg-oci-push/${WKG_IMAGE}:latest" "$TEMP_DIR/wasm32-wasip1/release/spin_rust_hello.wasm"
rm -rf "$TEMP_DIR"

sleep 5

echo ">>> cluster is ready"
