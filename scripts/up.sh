#!/bin/bash

set -euo pipefail

cluster_name="test-cluster"        # name of the kind cluster
dockerfile_path="deployments/kind" # path to the Dockerfile
registry_name="test-registry"
registry_port="5000"

DOCKER_IMAGES=("spin" "spin-keyvalue" "spin-outbound-redis" "spin-multi-trigger-app" "spin-static-assets" "spin-mqtt-message-logger")
OUT_DIRS=("test/out_spin" "test/out_spin_keyvalue" "test/out_spin_outbound_redis" "test/out_spin_multi_trigger_app" "test/out_spin_static_assets" "test/out_spin_mqtt_message_logger")
IMAGES=("spin-hello-world" "spin-keyvalue" "spin-outbound-redis" "spin-multi-trigger-app" "spin-static-assets" "spin-mqtt-message-logger")
WKG_IMAGE="spin-hello-world"
WKG_SOURCE="spin"

# build the Docker image for the kind cluster
docker build -t kind-shim-test "$dockerfile_path"

# Start a local registry container
if [ "$(docker inspect -f '{{.State.Running}}' "${registry_name}" 2>/dev/null || true)" != 'true' ]; then
  docker run -d --restart=always -p "${registry_port}:5000" --network bridge --name "${registry_name}" registry:2
fi

# Create a kind cluster using our custom node image
cat <<EOF | kind create cluster --name "$cluster_name" --image kind-shim-test --config=-
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
containerdConfigPatches:
- |-
  [plugins."io.containerd.grpc.v1.cri".registry.mirrors."localhost:${registry_port}"]
    endpoint = ["http://${registry_name}:5000"]
- |-
  [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin]
    runtime_type = "io.containerd.spin.v2"
  [plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin.options]
    SystemdCgroup = false
nodes:
- role: control-plane
  kubeadmConfigPatches:
  - |
    kind: InitConfiguration
    nodeRegistration:
      kubeletExtraArgs:
        eviction-hard: "imagefs.available<1%,nodefs.available<1%"
        eviction-minimum-reclaim: "imagefs.available=1%,nodefs.available=1%"
  extraPortMappings:
  - containerPort: 30080
    hostPort: 8082
    protocol: TCP
- role: worker
- role: worker
EOF

# Connect the registry to the kind network
if [ "$(docker inspect -f='{{json .NetworkSettings.Networks.kind}}' "${registry_name}")" = 'null' ]; then
  docker network connect "kind" "${registry_name}"
fi

kubectl wait --for=condition=ready node --all --timeout=120s

# Install ingress-nginx for local routing in Kind and wait for controller readiness.
kubectl label nodes --all ingress-ready=true --overwrite
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.15.1/deploy/static/provider/kind/deploy.yaml
kubectl -n ingress-nginx patch service ingress-nginx-controller --type merge -p '{"spec":{"type":"NodePort","ports":[{"name":"http","port":80,"targetPort":"http","nodePort":30080},{"name":"https","port":443,"targetPort":"https","nodePort":30443}]}}'
kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=180s

# Iterate through the Docker images and build them
for i in "${!DOCKER_IMAGES[@]}"; do
    docker buildx build -t "${IMAGES[$i]}:latest" "./images/${DOCKER_IMAGES[$i]}" --load
    mkdir -p "${OUT_DIRS[$i]}"
    docker save -o "${OUT_DIRS[$i]}/img.tar" "${IMAGES[$i]}:latest"
    kind load image-archive "${OUT_DIRS[$i]}/img.tar" --name "$cluster_name"

  ## also do spin builds and spin registry push
  ## images pushed as localhost:5000/<namespace>/<app>:<version>
  ## can be pulled as localhost:5000/<namespace>/<app>:<version> from within the kind cluster
  spin build -f "./images/${DOCKER_IMAGES[$i]}/spin.toml"
  ## For the spin-static-assets app, use archive layers to test this functionality in the shim
  if [ "${i}" == "spin-static-assets" ]; then
    export SPIN_OCI_ARCHIVE_LAYERS=1
  fi
  spin registry push "localhost:${registry_port}/spin-registry-push/${IMAGES[$i]}:latest" -f "./images/${DOCKER_IMAGES[$i]}/spin.toml" -k
done

# Build with cargo and push with wkg the simple Spin hello world app. Use ttl.sh since wkg does not support localhost registries with http. See https://github.com/bytecodealliance/wasm-pkg-tools/issues/149.
TEMP_DIR=$( mktemp -d )
cargo build --release --target=wasm32-wasip1 --manifest-path ./images/${WKG_SOURCE}/Cargo.toml --target-dir "$TEMP_DIR"
wkg oci push "ttl.sh/containerd-shim-tests/wkg-oci-push/${WKG_IMAGE}:latest" "$TEMP_DIR/wasm32-wasip1/release/spin_rust_hello.wasm"
rm -rf "$TEMP_DIR"

sleep 5

echo ">>> cluster is ready"
