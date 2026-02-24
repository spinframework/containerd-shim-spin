#! /bin/bash

set -xeuo pipefail

# Cleanup function
cleanup() {
  echo "Cleaning up..."
  docker rm -f docker-test 2>/dev/null || true
}

usage() {
  echo "Usage: $0"
  echo "Environment variables:"
  echo "  DOCKER_VERSION - The version of Docker to use (required). Examples: 27, 27.0, 27.0.1, etc"
  echo "  CONTAINERD_SPIN_SHIM_VERSION - The version of the containerd spin shim to use (optional if SHIM_FILE is set). Example: v0.22.0"
  echo "  SHIM_FILE - Path to a local containerd spin shim binary (optional if CONTAINERD_SPIN_SHIM_VERSION is set)"
}
# Set trap to cleanup on exit (success or failure)
trap cleanup EXIT

# Require that required environment variables are set
if [ -z "$DOCKER_VERSION" ]; then
  echo "Error: DOCKER_VERSION environment variable must be set."
  exit 1
fi

# Require that exactly shim path environment variable is set
if [ -z "${CONTAINERD_SPIN_SHIM_VERSION:-}" ] && [ -z "${SHIM_FILE:-}" ]; then
  echo "Error: Either CONTAINERD_SPIN_SHIM_VERSION or SHIM_FILE must be set."
  exit 1
fi

if [ -n "${SHIM_FILE:-}" ]; then
  # TODO: figure out how to make this work
  echo "Using SHIM_FILE at $SHIM_FILE"
  cp "$SHIM_FILE" "$(dirname "$0")/containerd-shim-spin-v2"
  SHIM_LOCATION_ARG="SHIM_FILE=containerd-shim-spin-v2"
else
  echo "Using CONTAINERD_SPIN_SHIM_VERSION at $CONTAINERD_SPIN_SHIM_VERSION"
  SHIM_LOCATION_ARG="CONTAINERD_SPIN_SHIM_VERSION=$CONTAINERD_SPIN_SHIM_VERSION"
fi

docker build \
    --build-arg $SHIM_LOCATION_ARG \
    --build-arg DOCKER_VERSION=$DOCKER_VERSION \
    -t docker-in-docker:$DOCKER_VERSION \
    -f "$(dirname "$0")/Dockerfile.dind-containerd-spin-shim" .

docker run -d --privileged \
  --name docker-test \
  -p 8080:8080 \
  docker-in-docker:$DOCKER_VERSION

# Wait for Docker to start
sleep 5

# Your docker exec command with timeout
if ! timeout 10 docker exec docker-test docker run -d \
  --name spin \
  --runtime io.containerd.spin.v2 \
  --platform wasi/wasm \
  --publish 8080:80 \
  ghcr.io/spinframework/containerd-shim-spin/examples/spin-rust-hello:v0.20.0 /; then
  echo "✗ Failed to start Spin container (timed out or errored)"
  exit 1
fi

echo "✓ Spin container started successfully"

sleep 2

# Test the endpoint
result=$(curl -s localhost:8080/hello)
if [ "$result" = "Hello world from Spin!" ]; then
  echo "✓ Test passed: Got expected response"
else
  echo "✗ Test failed: Expected 'Hello world from Spin!' but got '$result'"
fi
