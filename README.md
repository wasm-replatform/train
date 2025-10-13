# Train

Train-related services.

## Quick Start

To run the project locally:

1. Set the environment variables in a `.env` file in the project root (see `.env.example`). 
2. Build the wasm guest (builds `./target/wasm32-wasip2/release/r9k_position.wasm`)
3. Add a service to `compose.yaml` and run with Docker compose:

```bash
cargo build --package train --target wasm32-wasip2 --release
docker compose up
```
