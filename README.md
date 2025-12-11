# Train

Train-related services.

## Quick Start

To run the project locally:

1. Set the environment variables in a `.env` file in the project root (see `examples/.env.example`).
2. Build the wasm guest (builds `./target/wasm32-wasip2/release/train.wasm`)
3. Run the standalone example

```shell
set -a; source .env; set +a
cargo build --target wasm32-wasip2 --release
cargo run --example train -- run ./target/wasm32-wasip2/release/train.wasm
```

## Crates

This service brings together a number of related services from a legacy code base into a single deployable unit. In this iteration, we have created one crate per legacy service.

See the `README.md` files in the various crates for information on the function of those dimensions of this service. 

In future we expect a more unified service will emerge that may not resemble the legacy architecture so closely and this README should be updated accordingly.

