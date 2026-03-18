#!/usr/bin/env sh
set -euo pipefail

# Note: this script is now deprecated and no longer used in the context of more recent versions
# of SpinKube. Instead, the runtime-class-manager project handles installation of the Spin shim.
# See https://github.com/spinframework/runtime-class-manager/blob/main/README.md for more details.

# Based on https://github.com/KWasm/kwasm-node-installer/blob/main/script/installer.sh
# Distilled to only configuring the Spin shim

KWASM_DIR=/opt/kwasm

CONTAINERD_CONF=/etc/containerd/config.toml
IS_MICROK8S=false
IS_K3S=false
IS_RKE2_AGENT=false
IS_K0S_WORKER=false

if pgrep -f snap/microk8s > /dev/null; then
    CONTAINERD_CONF=/var/snap/microk8s/current/args/containerd-template.toml
    IS_MICROK8S=true
    if nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- ls /var/snap/microk8s/current/args/containerd-template.toml > /dev/null 2>&1 ;then
        KWASM_DIR=/var/snap/microk8s/common/kwasm
    else
        echo "Installer seems to run on microk8s but 'containerd-template.toml' not found."
        exit 1
    fi
elif ls $NODE_ROOT/var/lib/rancher/rke2/agent/etc/containerd/config.toml > /dev/null 2>&1 ; then
    IS_RKE2_AGENT=true
    cp $NODE_ROOT/var/lib/rancher/rke2/agent/etc/containerd/config.toml $NODE_ROOT/var/lib/rancher/rke2/agent/etc/containerd/config.toml.tmpl
    CONTAINERD_CONF=/var/lib/rancher/rke2/agent/etc/containerd/config.toml.tmpl
elif ls $NODE_ROOT/var/lib/rancher/k3s/agent/etc/containerd/config.toml > /dev/null 2>&1 ; then
    IS_K3S=true
    cp $NODE_ROOT/var/lib/rancher/k3s/agent/etc/containerd/config.toml $NODE_ROOT/var/lib/rancher/k3s/agent/etc/containerd/config.toml.tmpl
    CONTAINERD_CONF=/var/lib/rancher/k3s/agent/etc/containerd/config.toml.tmpl
elif pgrep -f /var/lib/k0s/bin/kubelet > /dev/null; then
    IS_K0S_WORKER=true
    CONTAINERD_CONF=/etc/k0s/containerd.d/spin.toml
    touch $NODE_ROOT$CONTAINERD_CONF
fi

# If SYSTEMD_CGROUP is not set, default to true except for distros that do not default to systemd
# TODO: detect k3d which defaults to cgroupfs
if [ -z "${SYSTEMD_CGROUP+x}" ] && [ "$IS_MICROK8S" = "true" ]; then
    SYSTEMD_CGROUP=false
else
    : "${SYSTEMD_CGROUP:=true}"
fi

# Install D-Bus if it's not available but systemd cgroups are requested
if [ "$SYSTEMD_CGROUP" = "true" ]; then
    if ! nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- which dbus-daemon >/dev/null 2>&1; then
        if nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- which apt-get >/dev/null 2>&1; then
            nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- apt-get update -y
            nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- apt-get install -y dbus
        elif nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- which yum >/dev/null 2>&1; then
            nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- yum install -y dbus
        elif nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- which dnf >/dev/null 2>&1; then
            nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- dnf install -y dbus
        elif nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- which apk >/dev/null 2>&1; then
            nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- apk add dbus
        else
            echo "WARNING: Could not install D-Bus. No supported package manager found."
            SYSTEMD_CGROUP=false
            echo "SYSTEMD_CGROUP is now set to $SYSTEMD_CGROUP"
        fi
    fi
fi

mkdir -p $NODE_ROOT$KWASM_DIR/bin/

cp /assets/containerd-shim-spin-v2 $NODE_ROOT$KWASM_DIR/bin/

# A bug in containerd makes BinaryName not work with shim not in PATH, so this statically links the kwasm installation to path
# https://github.com/containerd/containerd/issues/11480
mkdir -p $NODE_ROOT/usr/local/bin/
ln -sf $KWASM_DIR/bin/containerd-shim-spin-v2 $NODE_ROOT/usr/local/bin/containerd-shim-spin-v2

if ! grep -q spin $NODE_ROOT$CONTAINERD_CONF; then
    echo "Adding Spin runtime to containerd"
    if grep -q "version = 3" $NODE_ROOT$CONTAINERD_CONF; then
        echo '
[plugins."io.containerd.cri.v1.runtime".containerd.runtimes."spin"]
    runtime_type = "'$KWASM_DIR'/bin/containerd-shim-spin-v2"
' >> $NODE_ROOT$CONTAINERD_CONF
    else
        echo '
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin]
    runtime_type = "'$KWASM_DIR'/bin/containerd-shim-spin-v2"
' >> $NODE_ROOT$CONTAINERD_CONF
    fi
    rm -Rf $NODE_ROOT$KWASM_DIR/active
fi

# Configure Spin runtime options if using systemd cgroups and no options are configured
# TODO: this should allow for some options to already be configured and just additively 
# configure SystemdCgroup
if ! grep -q 'runtimes.spin.options' $NODE_ROOT$CONTAINERD_CONF && [ "$SYSTEMD_CGROUP" = "true" ]; then
    echo "Setting SystemdCgroup to $SYSTEMD_CGROUP in Spin containerd configuration"
    if grep -q "version = 3" $NODE_ROOT$CONTAINERD_CONF; then
        echo '
[plugins."io.containerd.cri.v1.runtime".containerd.runtimes.spin.options]
    SystemdCgroup = '$SYSTEMD_CGROUP'
' >> $NODE_ROOT$CONTAINERD_CONF
    else
        echo '
[plugins."io.containerd.grpc.v1.cri".containerd.runtimes.spin.options]
    SystemdCgroup = '$SYSTEMD_CGROUP'
' >> $NODE_ROOT$CONTAINERD_CONF
    fi
fi

if $IS_K3S; then
    sed -i "s|runtime_type = \"io.containerd.spin.*\"|runtime_type = \"$KWASM_DIR/bin/containerd-shim-spin-v2\"|g" $NODE_ROOT$CONTAINERD_CONF
fi

if [ ! -f $NODE_ROOT$KWASM_DIR/active ]; then
    touch $NODE_ROOT$KWASM_DIR/active
    
    # Ensure D-Bus is running before restarting services if using systemd cgroups
    if [ "$SYSTEMD_CGROUP" = "true" ]; then
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart dbus
    fi
    
    if $IS_MICROK8S; then
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart snap.microk8s.daemon-containerd
    elif $IS_K3S; then
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart k3s
    elif $IS_RKE2_AGENT; then
        nsenter --target 1 --mount --uts --ipc --net -- systemctl restart rke2-agent
    elif $IS_K0S_WORKER; then
        svc_name=k0sworker
        if ! systemctl list-units | grep -q $svc_name; then
            svc_name=k0scontroller
        fi

        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart $svc_name
    elif ls $NODE_ROOT/etc/init.d/containerd > /dev/null 2>&1 ; then
        nsenter --target 1 --mount --uts --ipc --net -- /etc/init.d/containerd restart
    else
        nsenter -m/$NODE_ROOT/proc/1/ns/mnt -- systemctl restart containerd
    fi
else
    echo "No change in containerd/config.toml"
fi
