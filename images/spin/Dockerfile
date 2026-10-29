FROM --platform=${BUILDPLATFORM} rust:1.97 AS build
WORKDIR /opt/build
RUN rustup target add wasm32-wasip2
COPY . .
RUN cargo build --target wasm32-wasip2 --release

FROM --platform=${BUILDPLATFORM} golang:1.27.2-bookworm AS build-go
WORKDIR /opt/build
COPY . .
WORKDIR /opt/build/go-hello
RUN go mod download
RUN go tool componentize-go build -o spin_go_hello.wasm

FROM scratch
COPY --from=build /opt/build/target/wasm32-wasip2/release/spin_rust_hello.wasm ./target/wasm32-wasip2/release/spin_rust_hello.wasm
COPY --from=build /opt/build/spin.toml .
COPY --from=build-go /opt/build/go-hello/spin_go_hello.wasm ./go-hello/spin_go_hello.wasm
