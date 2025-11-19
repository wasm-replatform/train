# Train Codebase Structure Guide

## 1. High-Level Overview
- **Purpose**: WASM-based replatform of legacy TypeScript train services into Rust targeting `wasm32-wasip2`.
- **Runtime**: Components export WASI HTTP and messaging handlers for deployment under the Credibil runtime.
- **Key Directories**:
	- `src/`: Host application (WASM entrypoints, provider implementation).
	- `crates/`: Domain libraries (e.g., `dilax-adapter`, `r9k-adapter`, `realtime`).
	- `legacy/`: Reference TypeScript services used during migration.
	- `.github/`: Operational guidance for AI assistants (`copilot-instructions.md`, this file).

## 2. Component Model
### 2.1 WASM Entry Points (`src/lib.rs`)
- Exports two handlers:
	- `wasip3::http::handler::Guest` for HTTP routes (`/jobs/detector`, `/inbound/xml`).
	- `wasi_messaging::incoming_handler::Guest` for Kafka topics.
- Both handlers create a `credibil_api::Client` with the shared provider and forward requests to adapter crates.
- Instrumentation: use `#[wasi_otel::instrument]` and `tracing` metrics (`monotonic_counter.*`).

### 2.2 Runtime Flow
1. Incoming HTTP/XML payloads → `r9k-connector` handler → publishes to Kafka.
2. Kafka messages → `Messaging::handle` → delegates to `r9k_adapter` or `dilax_adapter`.
3. Domain crate processes data, enriches context, and republishes to downstream topics.

## 3. Provider Pattern (Dependency Injection)
### 3.1 Trait Contract (`crates/realtime/src/provider.rs`)
```rust
pub trait Provider: HttpRequest + Identity + Publisher {}
impl<T> Provider for T where T: HttpRequest + Identity + Publisher {}
```
- Sub-traits expose async capabilities as associated futures (HTTP, messaging, state store, identity).
### 3.2 Host Implementation (`src/provider.rs`)
- `Provider` struct implements capability traits using WASI interfaces:
	- `HttpRequest::fetch` → `wasi_http::handle`
	- `Publisher::send` → `wasi_messaging::producer::send`
	- `StateStore` (added in domain crates) → `wasi_keyvalue::cache`
	- `Identity::access_token` → `wasi_identity::credentials`
- Prefix Kafka topics with `ENV` (`LazyLock<String>` from environment).
### 3.3 Testing Adapters (`crates/*/tests/provider.rs`)
- Each adapter defines a `MockProvider` that records published events and stubs HTTP/state interactions.
- Tests seed environment variables and deterministic responses for adapter logic verification.

## 4. Domain Crates
### 4.1 `crates/dilax-adapter`
- Structure:
	- `handlers/processor.rs`: Enrich Dilax payloads with vehicle, stop, trip, and occupancy data.
	- `handlers/detector.rs`: Trigger lost-connection jobs.
	- `block_mgt.rs`, `gtfs.rs`: Provider-backed integrations (vehicle lookup, stop resolution).
	- `trip_state.rs`: Trip state caching via `StateStore`.
	- `error.rs`, `types.rs`: Domain error/type definitions.
- Result alias: `pub type Result<T> = anyhow::Result<T, Error>;` ensures business logic uses structured errors.

### 4.2 `crates/r9k-adapter`
- `handler.rs`: Converts R9K XML-derived updates into SmarTrak events, publishes twice for adherence tracking.
- `stops.rs`: GTFS lookups (stop metadata caching and filtering).
- `smartrak.rs`, `r9k.rs`: Domain models and validation rules.
- `error.rs`: Maps `anyhow` and `quick_xml::DeError` into API-friendly variants.

### 4.3 `crates/r9k-connector`
- Handles raw XML requests, validates envelope, and republishes canonical R9K messages to Kafka.

### 4.4 `crates/realtime`
- Defines shared provider traits, `Message` struct, and common utilities consumed by adapters and host.

## 5. Business Logic vs Infrastructure
- **Business Logic**: Pure domain operations inside `crates/*/src/handlers` and supporting modules; they accept `impl Provider` to remain decoupled.
- **Infrastructure**: WASI bindings, topic naming, environment access located in `src/` and `src/provider.rs`.
- Migration tip: when porting from legacy TypeScript (e.g., `legacy/at_dilax_adapter/src/dilax-lost-connections-detector.ts`), translate core algorithms into Rust handlers and use provider traits instead of direct IO calls.

## 6. Error Handling Conventions
- Domain errors live alongside business logic (`crates/dilax-adapter/src/error.rs`, `crates/r9k-adapter/src/error.rs`).
- Implement `From<anyhow::Error>` to retain context chains.
- Business functions return `Result<_, Error>`; infrastructure functions may return `anyhow::Result`.
- Use descriptive variants (`ProcessingError`, `InvalidFormat`, etc.) instead of string messages.

## 7. Async & WASM Considerations
- Target `wasm32`; crates set `#![cfg(target_arch = "wasm32")]` where necessary.
- Allow `clippy::future_not_send` at crate roots because WASM runtime is single-threaded.
- Avoid blocking operations; when unavoidable (e.g., repeated publish loop), guard with `#[cfg(not(debug_assertions))]` delays as seen in `r9k_adapter::handler`.

## 8. Telemetry & Logging
- Use `tracing` macros with monotonic counters for observability.
- Wrap entry points with `#[wasi_otel::instrument]` to emit OpenTelemetry spans.
- Include contextual fields (vehicle ids, trip ids) to aid debugging.

## 9. State Management
- Trip state updates use helper functions in `trip_state.rs`, persisting via `StateStore` trait.
- Cache keys should be scoped by vehicle id or trip id to avoid collisions.
- Serialization uses `serde_json` and `serde` derives for all payloads.

## 10. Testing Strategy
- **Unit Tests**: Adapter-specific tests with `MockProvider` verifying business logic (`crates/r9k-adapter/tests`).
- **Integration Tests**: Intended to run against Docker Compose stack (Kafka, Redis, OTEL) after building WASM artifact.
- Ensure deterministic responses in mocks and use `pretty_assertions` for diffs where appropriate.

## 11. Formatting & Linting
- `rustfmt.toml`: `max_width = 100`, compressed parameter layout, grouped imports (`StdExternalCrate`).
- `clippy.toml` and `workspace.lints`: enforce `clippy::all`, `::pedantic`, `::nursery`, plus selected restriction lints.
- Add targeted `#[allow(...)]` with justification comments when necessary (e.g., WASM async constraints).

