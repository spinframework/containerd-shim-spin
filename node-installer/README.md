> Note: the [spinframework/runtime-class-manager](https://github.com/spinframework/runtime-class-manager)
project now handles installing containerd-shim-spin-v2 onto Nodes in SpinKube,
meaning this project's node-installer is no longer used. However, it will still
be offered here for a period of time for use with older installations.

This directory contains resources for a custom node-installer image
used in conjunction with the [Kwasm Operator](https://github.com/KWasm/kwasm-operator).

This version of the image only contains the containerd-shim-spin-v2 shim, as
opposed to the default [kwasm-node-installer image](https://github.com/KWasm/kwasm-node-installer)
which also bundles other shims.

## Integration Tests

The project includes integration test scripts for different Kubernetes distributions in the `tests/` directory:

1. Kind: `make test-kind`
2. MiniKube: `make test-minikube`
3. MicroK8s: `make test-microk8s`
4. K3s: `make test-k3s`

## Build the Image Locally

```bash
make build-dev-installer-image
```

