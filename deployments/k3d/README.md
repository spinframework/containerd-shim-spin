# K3d Shim Deployment
This example shows how one could deploy the Spin shim and use them locally using k3d. The example consists of the following files.

```
$ tree .
.
├── Dockerfile
├── DockerSetup.md
└── README.md
```

- **Dockerfile:** is the specification for the image run as a Kubernetes node within the k3d cluster. We add the shim to the `/bin` directory and add the containerd config in the k3s prescribed directory.

## How to run the example
The shell script below will create a k3d cluster locally with the Spin shim installed and containerd configured. The script then applies the runtime classes for the shim and an example service and deployment. Finally, we curl the `/hello` and receive a response from the example workload.
```shell
k3d cluster create wasm-cluster --image ghcr.io/spinframework/containerd-shim-spin/k3d:v0.23.0 -p "8081:80@loadbalancer" --agents 2
kubectl apply -f https://github.com/spinkube/containerd-shim-spin/raw/main/deployments/workloads/runtime.yaml
kubectl apply -f https://github.com/spinkube/containerd-shim-spin/raw/main/deployments/workloads/workload.yaml
echo "waiting 15 seconds for workload to be ready"
sleep 15
curl -v http://127.0.0.1:8081/hello
```

To tear down the cluster, run the following.
```shell
k3d cluster delete wasm-cluster
```

## How build get started from source
Go to the root of the repository and run the following commands.
```shell
make up
```
