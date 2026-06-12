ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

ARG CROSS_CMAKE_SYSTEM_PROCESSOR
ARG CROSS_SYSROOT

ENV CROSS_TOOLCHAIN_PREFIX=${CROSS_CMAKE_SYSTEM_PROCESSOR}-linux-musl-

# Build libseccomp from source against the musl sysroot.
# libseccomp-rs requires a static musl-linked libseccomp; we compile it here
# using the cross-rs musl toolchain so we don't depend on any third-party images.
RUN apt-get -y update && \
    apt-get install -y pkg-config gperf curl unzip && \
    PROTOC_VERSION=25.3 && \
    PROTOC_ARCH=$(uname -m | sed 's/x86_64/x86_64/;s/aarch64/aarch_64/') && \
    curl -fsSL "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-${PROTOC_ARCH}.zip" \
        -o /tmp/protoc.zip && \
    unzip /tmp/protoc.zip -d /usr/local bin/protoc "include/*" && \
    rm /tmp/protoc.zip && \
    curl -fsSL https://github.com/seccomp/libseccomp/releases/download/v2.5.3/libseccomp-2.5.3.tar.gz \
        -o /tmp/libseccomp.tar.gz && \
    tar xf /tmp/libseccomp.tar.gz -C /tmp && \
    cd /tmp/libseccomp-2.5.3 && \
    ./configure \
        --host=${CROSS_CMAKE_SYSTEM_PROCESSOR}-linux-musl \
        CC=${CROSS_TOOLCHAIN_PREFIX}gcc \
        --prefix=${CROSS_SYSROOT} \
        --disable-shared \
        --enable-static && \
    make -j$(nproc) && \
    make install && \
    rm -rf /tmp/libseccomp*

# configure libseccomp-rs to use static linking against the musl sysroot
ENV LIBSECCOMP_LINK_TYPE="static"
ENV LIBSECCOMP_LIB_PATH="${CROSS_SYSROOT}/lib"

## as per analysis done in https://github.com/fermyon/spin/pull/2287#issuecomment-1970145410 we need
## to disable the cmake config in cross-rs. Setting CROSS_SYSROOT=/ seems to have done the trick
ENV CROSS_SYSROOT=/
