# K3d Shim Deployment
This example shows how one could deploy the Spin shim and use them locally using k3d. The example consists of the following files.

```
$ tree .
.
├── config.toml.tmpl
├── Dockerfile
├── Makefile
└── README.md
```

- **config.toml.tmpl:** is the containerd config template that k3d uses to generate the containerd config. We have added a line to the template to register the shim, so that containerd will understand how to run our Wasm pod's runtime class.
- **Dockerfile:** is the specification for the image run as a Kubernetes node within the k3d cluster. We add the shim to the `/bin` directory and add the containerd config in the k3s prescribed directory.
- **Makefile**: has some helpful tasks to aid in execution.

## How to run the example
The shell script below will create a k3d cluster locally with the Spin shim installed and containerd configured. The script then applies the runtime classes for the shim and an example service and deployment. Finally, we curl the `/hello` and receive a response from the example workload.
```shell
k3d cluster create wasm-cluster --image ghcr.io/spinframework/containerd-shim-spin/k3d:v0.20.0 -p "8081:80@loadbalancer" --agents 2
kubectl apply -f https://github.com/spinkube/containerd-shim-spin/raw/main/deployments/workloads/runtime.yaml
kubectl apply -f https://github.com/spinkube/containerd-shim-spin/raw/main/deployments/workloads/workload.yaml
echo "waiting 5 seconds for workload to be ready"
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
