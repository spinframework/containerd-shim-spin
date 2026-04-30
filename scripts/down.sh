#!/bin/bash

set -euo pipefail

cluster_name="test-cluster"
dockerfile_path="deployments/kind"
bin_path="${dockerfile_path}/.tmp/"
registry_name="test-registry"

teardown_test() {
  # delete kind cluster
  kind delete cluster --name "$cluster_name"

  # delete docker image
  docker rmi kind-shim-test || true

  # remove registry container
  docker rm -f "${registry_name}" || true

  # remove test folder
  rm -r ./test || echo "test folder already deleted"

  # delete binaries
  rm -r "$bin_path"
}

teardown_test