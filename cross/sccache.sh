#!/usr/bin/env bash

set -euo pipefail

target="${1:?Usage: sccache.sh <target-triple>}"

SCCACHE_VERSION="v0.8.2"
url="https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-${target}.tar.gz"

echo "Installing sccache ${SCCACHE_VERSION} for ${target}..."
curl -fsSL "${url}" -o /tmp/sccache.tar.gz
tar xf /tmp/sccache.tar.gz -C /tmp
install -m 755 "/tmp/sccache-${SCCACHE_VERSION}-${target}/sccache" /usr/bin/sccache
rm -rf /tmp/sccache*

echo "sccache installed: $(sccache --version)"
