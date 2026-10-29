FROM --platform=${BUILDPLATFORM} rust:1.97 AS build
WORKDIR /opt/build
RUN rustup target add wasm32-wasip2
COPY . .
RUN cargo build --target wasm32-wasip2 --release

FROM scratch
COPY --from=build /opt/build/spin.toml ./spin.toml
COPY --from=build /opt/build/target/wasm32-wasip2/release/spin_keyvalue.wasm ./target/wasm32-wasip2/release/spin_keyvalue.wasm
