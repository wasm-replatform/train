# Copilot Instructions for R9K Position Adapter

## Architecture Overview

This is a **WebAssembly Component Model (WASM)** microservice that transforms R9K train tracking data (XML from KiwiRail) into SmarTrak events via Kafka messaging. The codebase uses a **dual-target architecture**:

- **WASM guest** (`src/lib.rs`): Compiles to `wasm32-wasip2` with `cdylib` output, runs in a WASM runtime with WIT bindings
- **Business logic library** (`crates/r9k-position-adapter`): Pure Rust library consumed by the WASM guest, testable on native targets

The boundary is critical: `src/lib.rs` handles messaging/infrastructure via WIT bindings and delegates domain logic to `r9k-position-adapter` crate.

## Build & Test Workflow

**Use `cargo-make` exclusively** (not raw `cargo` commands):
```bash
make build          # Clean + build all targets
make test           # Run nextest with all features
make check          # Full hygiene: audit, fmt, lint, outdated, unused
make fmt            # Format code (requires nightly rustfmt)
```

**Building WASM for local deployment:**
```bash
cargo build --package r9k --target wasm32-wasip2 --release
docker compose up   # Runs ./target/wasm32-wasip2/release/r9k.wasm
```

The `Makefile` delegates to `Makefile.toml` (cargo-make configuration). Tests use `cargo-nextest` with `--no-fail-fast --all-features`.

## Code Structure Patterns

### Provider Pattern for External Dependencies

The `Provider` trait (in `r9k-position-adapter/src/provider.rs`) abstracts external API calls:
- **Production**: `src/provider.rs` (WASM guest) returns hardcoded mock data (TODO: implement real API calls)
- **Tests**: `tests/provider.rs` implements test fixtures
- Key types: `Key::StopInfo(stop_code)` → GTFS API, `Key::BlockMgt(train_id)` → Block Management API

When implementing features that need external data, extend the `Key` and `SourceData` enums, then implement `Source::fetch()`.

### Handler Pattern with credibil-api

The `credibil-api` crate provides a generic `Handler<Response, Provider>` trait. See `crates/r9k-position-adapter/src/handler.rs`:
- Implement `Handler` on `Request<YourMessage>` 
- Use `#[wasi_otel::instrument]` for tracing (from `sdk-otel` crate)
- Handlers are async and return `Result<Response<YourResponse>>`

### WIT Bindings & Messaging

The WASM guest exports `messaging::incoming_handler::Guest` (see `src/lib.rs`):
- `handle(message)` processes Kafka messages from topic "r9k.request"
- `configure()` returns topic subscriptions
- Use `wit_bindgen::spawn()` for background tasks (e.g., publishing responses)
- OpenTelemetry instrumentation via `#[wasi_otel::instrument]` attributes

## Code Quality Standards

### Linting Configuration
- **clippy.toml**: Defines domain-specific valid identifiers (`R9K`, `SmarTrak`, `KiwiRail`)
- **rustfmt.toml**: Uses `max_width = 100`, `group_imports = "StdExternalCrate"`, requires nightly for unstable features
- **deny.toml**: License checks, bans duplicate `tokio` versions, allows specific duplicates (see `allowed-duplicate-crates`)

### Custom Lints (Cargo.toml)
Workspace enables aggressive linting: `all`, `nursery`, `pedantic`, `cargo` + cherry-picked `restriction` lints following [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines/). Examples:
- `undocumented_unsafe_blocks`, `map_err_ignore`, `renamed_function_params`

## Dependency Management

- **Custom registries**: `credibil` and `at-realtime` via Azure DevOps (see `.cargo/config.toml`)
- **Cargo-vet**: After dependency updates, run `cargo vet regenerate imports/exemptions` (see `supply-chain/README.md`)
- **Version pinning**: Workspace dependencies in `Cargo.toml` [workspace.dependencies]

## Environment & Deployment

- `.env.example` shows required environment variables (GTFS_API_ADDR, BLOCK_MGT_ADDR, OTEL endpoints, etc.)
- **Docker Compose stack**: Kafka, Kafka UI (port 8081), Jaeger (16686), Prometheus (9090), OpenTelemetry Collector
- WASM runtime expects `/r9k.wasm` mounted from `target/wasm32-wasip2/release/`

## Common Patterns

**XML deserialization** (R9K messages):
- Uses `quick-xml` with `serde` features
- Spanish field names mapped via `#[serde(rename(deserialize = "..."))]` (see `r9k.rs`)
- Implement `TryFrom<&[u8]>` for message parsing

**Error handling**:
- Custom `Error` enum in `r9k-position-adapter/src/error.rs`
- Validation errors: `Error::NoUpdate`, `Error::Outdated`, `Error::WrongTime`
- Time constraints: `MAX_DELAY_SECS = 60`, `MIN_DELAY_SECS = -30`

**Metrics/Logging**:
- Use `tracing` macros with structured fields: `info!(gauge.r9k_delay = delay_secs)`
- Counters: `monotonic_counter.processing_errors = 1`
