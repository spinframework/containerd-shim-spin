#!/bin/bash

set -euo pipefail

# This script generates a Shim resource YAML for the Runtime Class Manager
# Usage: ./generate-shim-resource.sh <release-version> <crd-version> <artifacts-dir> <output-file> [crd-release-tag]

RELEASE_VERSION="${1:?Release version required (e.g., v0.23.0)}"
CRD_VERSION="${2:?CRD version required (e.g., v1alpha1)}"
ARTIFACTS_DIR="${3:?Artifacts directory required}"
OUTPUT_FILE="${4:?Output file required}"
CRD_RELEASE_TAG="${5:-v0.2.0}"  # Default to v0.2.0 which contains v1alpha1 CRD

# Calculate SHA256 checksums for the artifacts
AARCH64_TARBALL="${ARTIFACTS_DIR}/containerd-shim-spin-v2-linux-aarch64/containerd-shim-spin-v2-linux-aarch64.tar.gz"
X86_64_TARBALL="${ARTIFACTS_DIR}/containerd-shim-spin-v2-linux-x86_64/containerd-shim-spin-v2-linux-x86_64.tar.gz"

if [[ ! -f "${AARCH64_TARBALL}" ]]; then
    echo "Error: aarch64 tarball not found at ${AARCH64_TARBALL}" >&2
    exit 1
fi

if [[ ! -f "${X86_64_TARBALL}" ]]; then
    echo "Error: x86_64 tarball not found at ${X86_64_TARBALL}" >&2
    exit 1
fi

AARCH64_SHA256=$(sha256sum "${AARCH64_TARBALL}" | awk '{print $1}')
X86_64_SHA256=$(sha256sum "${X86_64_TARBALL}" | awk '{print $1}')

echo "Generating Shim resource for ${RELEASE_VERSION} (CRD ${CRD_VERSION})"
echo "  aarch64 SHA256: ${AARCH64_SHA256}"
echo "  x86_64 SHA256: ${X86_64_SHA256}"

# Generate the Shim YAML
cat > "${OUTPUT_FILE}" <<EOF
apiVersion: runtime.spinkube.dev/${CRD_VERSION}
kind: Shim
metadata:
  name: spin
  labels:
    app.kubernetes.io/name: spin
    app.kubernetes.io/instance: spin
    app.kubernetes.io/part-of: runtime-class-manager
spec:
  nodeSelector:
    spin: "true"

  fetchStrategy:
    platforms:
      - os: linux
        arch: aarch64
        location: "https://github.com/spinframework/containerd-shim-spin/releases/download/${RELEASE_VERSION}/containerd-shim-spin-v2-linux-aarch64.tar.gz"
        sha256: "${AARCH64_SHA256}"
      - os: linux
        arch: x86_64
        location: "https://github.com/spinframework/containerd-shim-spin/releases/download/${RELEASE_VERSION}/containerd-shim-spin-v2-linux-x86_64.tar.gz"
        sha256: "${X86_64_SHA256}"

  # Each runtime can provide a set of containerd runtime options to be set in the containerd
  # configuration file.
  containerdRuntimeOptions:
    # The following option to pass cgroup driver information is available to runwasi based runtimes.
    # For runwasi, the default cgroup driver is cgroupfs. Failure to configure the correct cgroup
    # driver for runwasi shims may result in pod metrics failing to propagate accurately.
    SystemdCgroup: "true"

  runtimeClass:
    # Note: this name is used by the Spin Operator project as its default:
    # https://github.com/spinframework/spin-operator/blob/main/config/samples/spin-shim-executor.yaml
    name: wasmtime-spin-v2
    handler: spin

  rolloutStrategy:
    type: recreate
EOF

echo "Shim resource written to ${OUTPUT_FILE}"

# Validate the generated Shim resource
echo ""
echo "Validating Shim resource..."

# Check if kubectl-validate is installed
if ! command -v kubectl-validate &> /dev/null; then
    echo "kubectl-validate not found. Installing..."
    if ! command -v go &> /dev/null; then
        echo "Error: Go is required to install kubectl-validate but was not found" >&2
        echo "Skipping validation. Please install Go or kubectl-validate manually." >&2
        exit 1
    fi
    go install sigs.k8s.io/kubectl-validate@v0.0.4

    # Add GOPATH/bin to PATH if not already there
    if [[ -z "${GOPATH:-}" ]]; then
        GOPATH=$(go env GOPATH)
    fi
    export PATH="${GOPATH}/bin:${PATH}"
fi

# Download the CRD schema to a temporary directory
CRD_DIR=$(mktemp -d)
CRD_FILE="${CRD_DIR}/runtime.spinkube.dev_shims.yaml"
trap 'rm -rf ${CRD_DIR}' EXIT

echo "Downloading CRD schema from runtime-class-manager ${CRD_RELEASE_TAG}..."
if ! curl -fsSL -o "${CRD_FILE}" "https://raw.githubusercontent.com/spinframework/runtime-class-manager/refs/tags/${CRD_RELEASE_TAG}/config/crd/bases/runtime.spinkube.dev_shims.yaml"; then
    echo "Error: Failed to download CRD schema from ${CRD_RELEASE_TAG}" >&2
    exit 1
fi

# Validate the Shim resource
echo "Running validation..."
if kubectl-validate --local-crds "${CRD_DIR}" "${OUTPUT_FILE}"; then
    echo "✓ Validation successful!"
else
    echo "✗ Validation failed!" >&2
    exit 1
fi
