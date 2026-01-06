# Train Example

Demonstrates `train` using the default (in-memory) implementation of `WASI` services (e.g. `http`, `identity`, `keyvalue`, `messaging`, `otel`).

## Quick Start

```bash
cargo build --package train --target wasm32-wasip2 --release

set -a && source .env && set +a
cargo run --example train -- run ./target/wasm32-wasip2/release/train.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
