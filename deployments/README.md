# Shim Deployment Examples
This directory contains examples of how to deploy the Spin shim.

## Kind Deployment
[This deployment](kind) uses Kind to deploy a local Kubernetes cluster. It illustrates how to customize the Kind node image that is deployed. The image used to run the Kind Kubernetes nodes has the Spin shim copied into the `/bin` directory and the containerd config updated to include runtime bindings for the shim.

## Cluster API Deployment
Coming soon...

## Workloads
In [the workloads directory](./workloads) you will find common workloads that we deploy to the clusters to register runtime classes and deploy Wasm enabled pod workloads.