FROM scratch
COPY ./spin.toml .
COPY --chmod=0755 ./target/wasm32-wasip2/release/spin_dapr.wasm /target/wasm32-wasip2/release/spin_dapr.wasm
ENTRYPOINT ["/target/wasm32-wasip2/release/spin_dapr.wasm"]