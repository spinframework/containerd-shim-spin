#!/bin/bash

set -euo pipefail


echo "collecting debug info from CI run in 'debug-logs' dir"

mkdir -p debug-logs

echo "-> kind get clusters" > debug-logs/kubernetes.log
kind get clusters >> debug-logs/kubernetes.log
echo "" >> debug-logs/kubernetes.log

echo "-> kubectl get pods -n default -o wide" >> debug-logs/kubernetes.log
kubectl get pods -n default -o wide >> debug-logs/kubernetes.log
echo "" >> debug-logs/kubernetes.log

echo "-> kubectl describe pods -n default" >> debug-logs/kubernetes.log
kubectl describe pods -n default >> debug-logs/kubernetes.log
echo "" >> debug-logs/kubernetes.log

for node in $(kind get nodes --name test-cluster); do
	echo "collecting containerd logs from $node"
	docker cp $node:/var/log/containerd.log debug-logs/$node.containerd.log || echo "containerd.log file not found in $node"
done