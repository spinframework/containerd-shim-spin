SPIN_VERSION = v2
IMAGE_NAME ?= ghcr.io/spinkube/containerd-shim-spin/node-installer
KUBECTL ?= kubectl

# we only support x86_64 and aarch64 on Linux
UNAME := $(shell uname -m)
ifeq ($(UNAME),x86_64)
  PLATFORM ?= linux/amd64
  ARCH ?= x86_64
else ifeq ($(UNAME),aarch64)
  PLATFORM ?= linux/arm64
  ARCH ?= aarch64
else ifeq ($(UNAME),arm64)
  PLATFORM ?= linux/arm64
  ARCH ?= aarch64
else
  $(error Unsupported platform: $(UNAME))
endif

TARGET ?= $(ARCH)-unknown-linux-musl
BINARY := ./.tmp/$(PLATFORM)/containerd-shim-spin-$(SPIN_VERSION)

compile-musl:
	make build-cross-$(TARGET) -C ../

$(BINARY): 
	mkdir -p ./.tmp/$(PLATFORM)
	$(MAKE) compile-musl
	cp ../target/$(TARGET)/release/containerd-shim-spin-$(SPIN_VERSION) $(BINARY)

build-multi-installer-image: $(BINARY)
	docker buildx build -t $(IMAGE_NAME) --platform linux/amd64,linux/arm64 .

build-dev-installer-image: $(BINARY)
	docker buildx build -t $(IMAGE_NAME) --load --platform $(PLATFORM) .

deploy-workload:
	$(KUBECTL) apply -f ./tests/workloads/workload.yaml
	$(KUBECTL) wait --for=condition=Available deployment/wasm-spin --timeout=120s
	$(KUBECTL) get pods

test-workload:
	@sleep 10
	$(eval PID := $(shell $(KUBECTL) port-forward svc/wasm-spin 8888:80 > /dev/null 2>&1 & echo $$!))
	@sleep 5
	@for i in $$(seq 1 3); do \
		if curl -s http://localhost:8888/hello | grep -q "Hello world from Spin!"; then \
			kill $(PID) 2>/dev/null || true; \
			exit 0; \
		else \
			sleep 3; \
		fi; \
	done; \
	kill $(PID) 2>/dev/null || true; \
	echo "Failed to get a successful response from the workload."
	exit 1

test-kind:
	./tests/integration-test-kind.sh

test-minikube:
	./tests/integration-test-minikube.sh

test-microk8s:
	./tests/integration-test-microk8s.sh

test-k3s:
	./tests/integration-test-k3s.sh

test-all: test-kind test-minikube test-microk8s test-k3s
