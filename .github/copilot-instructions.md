# Copilot Instructions for Train Services

## Project Overview

This is a WASM-based replatform of legacy TypeScript train services into Rust, targeting `wasm32-wasip2`. The project processes real-time train data from R9K track sensors and Dilax passenger counting systems, enriching and publishing events to Kafka topics for downstream consumption.

**Legacy → Rust Migration**: Active conversion from `legacy/` TypeScript services to `crates/` Rust implementations. Reference `MIGRATE.md` for conversion workflow.

## Architecture Patterns

### Provider Pattern (Critical)

The codebase uses a **Provider trait pattern** for dependency injection. Each adapter crate defines domain-specific Provider traits:

```rust
// From crates/dilax-adapter/src/lib.rs
pub trait Provider: HttpRequest + Publisher + StateStore + Identity {}
```

- **Domain crates** (`crates/dilax-adapter`, `crates/r9k-adapter`) define `Provider` traits with required capabilities
- **Host application** (`src/provider.rs`) implements these traits using WASI interfaces
- **Tests** (`tests/provider.rs`) implement mock providers for unit testing

**Key trait capabilities**:

- `HttpRequest`: Make HTTP calls via `wasi-http` (block management, GTFS data)
- `Publisher`: Send messages to Kafka topics via `wasi-messaging`
- `StateStore`: Redis key-value operations via `wasi-keyvalue`
- `Identity`: Azure AD token retrieval via `wasi-identity`

### WASM Component Model

The main crate (`src/lib.rs`) compiles to `cdylib` and exports two WASI interfaces:

1. **HTTP handler** (`wasip3::http::handler::Guest`): Receives inbound HTTP requests (R9K connector endpoint)
2. **Messaging handler** (`wasi_messaging::incoming_handler::Guest`): Consumes Kafka messages (R9K and Dilax events)

Domain crates (`crates/*`) compile as `lib` and are linked into the main component.

### Service Boundaries

```
├── r9k-connector/    # Receives R9K XML from track sensors → publishes to Kafka
├── r9k-adapter/      # Transforms R9K data → SmarTrak location events
├── dilax-adapter/    # Enriches Dilax APC data with GTFS/block allocation
└── realtime/         # Shared Provider traits (imported by all adapters)
```

## Build & Development Workflow

### Essential Commands

```bash
# Build WASM component (required before running)
cargo build --package train --target wasm32-wasip2 --release

# Run with Docker Compose (includes Kafka, Redis, OTEL stack)
docker compose up

# Run tests (uses cargo-nextest)
cargo make test

# Lint/format/audit (pre-commit hygiene)
cargo make check

# Full workflow available via cargo-make
make <task>  # delegates to cargo-make via Makefile
```

**Critical**: Always build for `wasm32-wasip2` target before deploying. The `compose.yaml` mounts `./target/wasm32-wasip2/release/train.wasm`.

### Testing Strategy

- **Unit tests**: `crates/*/tests/` with mock providers (see `r9k-adapter/tests/provider.rs`)
- **Integration tests**: Run Docker Compose stack, send test payloads to endpoints
- **No miri**: Tests skip miri with `#![cfg(not(miri))]` due to WASM target incompatibility

## Code Conventions

### Error Handling

Use `anyhow::Result` in domain logic, convert to domain-specific errors (`crate::Error`) at API boundaries:

```rust
// From handlers/processor.rs
pub async fn process(event: DilaxMessage, provider: &impl Provider) -> Result<DilaxEnrichedEvent> {
    let vehicle = block_mgt::vehicle(&label, provider)
        .await
        .map_err(|err| Error::ProcessingError(format!("context: {err}")))?;
    // ...
}
```

### Lint Configuration

Follow Microsoft Rust Guidelines with strict lints enabled (see `Cargo.toml` workspace lints):

- Use `#[allow(clippy::future_not_send)]` for WASM async (single-threaded runtime)
- Document unsafe blocks: `#[allow(clippy::undocumented_unsafe_blocks)]` not permitted
- Prefer semantic errors over string messages

**Domain-specific identifiers** (add to `clippy.toml`): `R9K`, `SmarTrak`, `KiwiRail`

### Environment Variables

Set via `.env` file (see `compose.yaml`):

- `ENV`: Environment prefix for Kafka topics (`dev`, `test`, `prod`)
- `BLOCK_MGT_URL`: Block allocation service endpoint
- `CC_STATIC_URL`: GTFS static data endpoint
- `AZURE_IDENTITY`: For token retrieval

**Topic naming**: `{ENV}-realtime-{service}.v1` (e.g., `dev-realtime-r9k.v1`)

## Migration Notes

When converting legacy TypeScript services:

1. **Reference implementation**: Use `crates/r9k-adapter` as template for structure
2. **Data models**: TypeScript interfaces → Rust structs with `serde` derives
3. **Kafka consumers**: Replace kafkajs with `wasi-messaging` handler pattern
4. **Redis operations**: Replace ioredis with `wasi-keyvalue` StateStore trait
5. **HTTP clients**: Replace axios with `HttpRequest` provider trait

See `MIGRATE.md` for detailed conversion workflow with AI coding agents.

## External Dependencies

- **Credibil registry**: Custom WASI interfaces (`wasi-http`, `wasi-messaging`, etc.) from `credibil` registry
- **Block Management API**: `/allocations/trips` endpoint for vehicle-to-trip mapping
- **GTFS Static Data**: `/gtfs/stops` endpoint for stop location data
- **Kafka Topics**:
  - Input: `{ENV}-realtime-r9k.v1`, `{ENV}-realtime-dilax-apc.v2`
  - Output: `{ENV}-realtime-r9k-to-smartrak.v1`, `{ENV}-realtime-dilax-apc-enriched.v2`

## Key Files

- `src/lib.rs`: WASM entry points (HTTP + messaging handlers)
- `src/provider.rs`: Provider trait implementations using WASI
- `crates/realtime/src/provider.rs`: Shared Provider trait definitions
- `crates/*/src/handlers/`: Domain handler logic (detector jobs, event processors)
- `Makefile.toml`: Cargo-make task definitions (build, test, lint, release)
