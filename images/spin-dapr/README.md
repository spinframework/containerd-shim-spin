# Spin Dapr Demo

## Description
This demo application is a simple Spin app that is triggered by the daprs [kubernetes input binding](https://docs.dapr.io/reference/components-reference/supported-bindings/kubernetes-binding/) when it is called with the path `/kevents` it writes the body to a redis running at `redis://localhost:6379` to the key `lastEvent`. All other paths just return the value of the `lastEvent` key.

### Prerequisites
Install dapr cli
```sh
curl -fsSL https://raw.githubusercontent.com/dapr/cli/master/install/install.sh | bash
```

Install spin cli:
```sh
curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash
sudo mv ./spin /usr/local/bin/
```

### Run example with Kind:
```sh
# start the Kind cluster
cat <<EOF | kind create cluster --name wasm-cluster --image ghcr.io/spinframework/containerd-shim-spin/kind:v0.24.0 --config=-
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
containerdConfigPatches:
- |-
	[plugins."io.containerd.cri.v1.runtime".containerd.runtimes.spin]
	   runtime_type = "io.containerd.spin.v2"
	[plugins."io.containerd.cri.v1.runtime".containerd.runtimes.spin.options]
	   SystemdCgroup = true
EOF
# Install Dapr
dapr init -k --wait
# or via helm
# helm repo add dapr https://dapr.github.io/helm-charts/
# helm repo update
# helm upgrade --install dapr dapr/dapr --namespace dapr-system --create-namespace --wait

# build the application
cd images/spin-dapr
spin build
cd -
# create an image and load it into Kind
docker build images/spin-dapr -t spin-dapr:latest --load
kind load docker-image spin-dapr:latest --name wasm-cluster
# Apply the manifest
kubectl apply -f https://github.com/spinframework/containerd-shim-spin/raw/main/deployments/workloads/runtime.yaml
kubectl apply -f images/spin-dapr/deploy.yaml

# When everythin is up, forward the port and get the last kubernetes event
kubectl port-forward svc/spin-dapr 8080:80 &
curl localhost:8080 | jq
```