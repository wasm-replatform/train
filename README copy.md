# R9K Position Adapter

The R9K Position Adapter is responsible for transforming R9k data for specific stations into 
Smartrak events. The transformed events are then published to `smartrak_gtfs_adapter`.

## Quick Start

To run the project locally:

1. Set the environment variables in a `.env` file in the project root (see `.env.example`). 
2. Build the wasm guest (builds `./target/wasm32-wasip2/release/r9k_position_adapter.wasm`)
3. Add a service to `compose.yaml` and run with Docker compose:

```bash
cargo build --package r9k --target wasm32-wasip2 --release
docker compose up
```
