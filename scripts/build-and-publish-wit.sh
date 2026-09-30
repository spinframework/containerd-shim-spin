#!/bin/bash
# This script builds and publishes the spinkube:containerd-shim-spin WIT package.
# NOTE: The package name and version are inferred from the encoded wasm
# binary, which in turn comes from the `package spinkube:containerd-shim-spin@X.Y.Z`
# declaration in wit/world.wit. That declaration is kept in sync with the shim
# version by scripts/version.sh — the OCI tag below is derived from it automatically.
set -euo pipefail

wasm-tools component wit wit/ -w -o containerd_shim_spin_wit.wasm

# Derive "spinkube:containerd-shim-spin@X.Y.Z" from the encoded binary, then take the
# version so the OCI tag always matches what is inside the wasm.
package=$(wasm-tools component wit containerd_shim_spin_wit.wasm | sed -n 's/^package \(spinkube:containerd-shim-spin@[^;]*\);.*/\1/p' | head -1)
version=${package##*@}

if [[ -z "${version}" ]]; then
  echo "error: could not determine WIT package version from containerd_shim_spin_wit.wasm" >&2
  exit 1
fi

# Guard against publishing a WIT that describes a different shim release
shim_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
if [[ "${version}" != "${shim_version}" ]]; then
  echo "error: WIT package version ${version} does not match shim version ${shim_version} (run scripts/version.sh -s)" >&2
  exit 1
fi

# Publish the package to OCI registry using wkg
wkg oci push "ghcr.io/spinframework/containerd-shim-spin/wit:${version}" containerd_shim_spin_wit.wasm
