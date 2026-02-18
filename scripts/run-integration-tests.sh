#!/bin/bash

set -euo pipefail

## Deploy workloads into k3d cluster
if [ "$1" == "workloads-pushed-using-spin-registry-push" ]; then
    make deploy-workloads-pushed-using-spin-registry-push
elif [ "$1" == "workloads-pushed-using-docker-build-push" ]; then
    make deploy-workloads-pushed-using-docker-build-push
elif [ "$1" == "workloads-pushed-using-wkg-oci-push" ]; then
    make deploy-workloads-pushed-using-wkg-oci-push
else
    echo "invalid argument, expected one of: workloads-pushed-using-spin-registry-push, workloads-pushed-using-docker-build-push, workloads-pushed-using-wkg-oci-push"
    exit 1
fi

## Verify pods can be terminated successfully
make pod-terminates-test
	
## Run integration tests
if [ "$1" == "workloads-pushed-using-wkg-oci-push" ]; then
    cargo test -p containerd-shim-spin-tests --features wkg-tests -- --nocapture
else
    cargo test -p containerd-shim-spin-tests -- --nocapture
fi

## tests done, cleanup workloads for next test
make teardown-workloads