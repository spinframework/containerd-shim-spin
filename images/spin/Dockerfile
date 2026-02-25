FROM --platform=${BUILDPLATFORM} rust:1.91 AS build
WORKDIR /opt/build
RUN rustup target add wasm32-wasip1
COPY . .
RUN cargo build --target wasm32-wasip1 --release

FROM --platform=linux/amd64 golang:1.24.3-bullseye AS build-go
WORKDIR /opt/build
RUN curl -LO https://github.com/tinygo-org/tinygo/releases/download/v0.38.0/tinygo_0.38.0_amd64.deb && dpkg -i tinygo_0.38.0_amd64.deb
COPY . .
WORKDIR /opt/build/go-hello
RUN if [ -f go.mod ]; then go mod download; fi
RUN tinygo build -target=wasi -o spin_go_hello.wasm main.go

FROM scratch
COPY --from=build /opt/build/target/wasm32-wasip1/release/spin_rust_hello.wasm ./target/wasm32-wasip1/release/spin_rust_hello.wasm
COPY --from=build /opt/build/spin.toml .
COPY --from=build-go /opt/build/go-hello/spin_go_hello.wasm ./go-hello/spin_go_hello.wasm
