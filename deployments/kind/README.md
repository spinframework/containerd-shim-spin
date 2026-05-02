# Kind Shim Deployment
This example shows how one could deploy the Spin shim and use them locally using Kind. The example consists of the following files.

```
$ tree .
.
├── Dockerfile
├── DockerSetup.md
└── README.md
```

- **Dockerfile:** is the specification for the image run as a Kubernetes node within the Kind cluster. We add the shim to the `/bin` directory and add the containerd config.

## How to run the example
The shell script below will create a Kind cluster locally with the Spin shim installed and containerd configured. The script then applies the runtime classes for the shim and an example service and deployment. Finally, we curl the `/hello` and receive a response from the example workload.
```shell
docker build -t kind-shim-test deployments/kind
cat <<EOF | kind create cluster --name wasm-cluster --image kind-shim-test --config=-
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
containerdConfigPatches:
- |-
	[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin]
		runtime_type = "io.containerd.spin.v2"
	[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin.options]
		SystemdCgroup = false
EOF
kubectl apply -f https://github.com/spinframework/containerd-shim-spin/raw/main/deployments/workloads/runtime.yaml
kubectl apply -f https://github.com/spinframework/containerd-shim-spin/raw/main/deployments/workloads/workload.yaml
echo "waiting 15 seconds for workload to be ready"
sleep 15
curl -v http://127.0.0.1:8081/hello
```

To tear down the cluster, run the following.
```shell
kind delete cluster --name wasm-cluster
```

## How build get started from source
Go to the root of the repository and run the following commands.
```shell
make up
```
