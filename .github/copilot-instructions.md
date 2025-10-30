# Copilot Instructions for Train Workspace

## Architecture
- Root crate `train` compiles to a WASI guest; `src/lib.rs` wires WIT messaging, splits incoming Kafka topics (R9K vs Dilax), and publishes with `wit_bindgen::spawn`.
- Domain logic lives under `crates/`: `dilax` holds the APC rewrite (processor, detectors, providers, store), `r9k-position` contains the legacy R9K transformer, and `realtime` exposes shared HTTP error helpers.
- Persistent state goes through `KvStore` (`crates/dilax/src/store.rs`); it wraps `wit_bindings::keyvalue` to preserve TTL envelopes and set semantics—avoid calling the raw bucket APIs.

## Dilax Flow
- `DilaxProcessor::process` (`crates/dilax/src/processor.rs`) normalises device labels, resolves fleet/block/GTFS data, updates occupancy, saves `VehicleTripInfo`, then emits a `DilaxEnrichedEvent` back to `src/lib.rs`.
- `migrate_legacy_keys` keeps Redis compatibility with the Node adapter; update both processor and detector if any key names change.
- Occupancy thresholds derive from seating/total capacity and emit `OccupancyStatus` strings consumed downstream—tweak with caution and retain regression warnings.

## State & Redis Keys
- Vehicle trip snapshots sit at `apc:vehicleTripInfo:{vehicle_id}` (48h TTL) containing the most recent Dilax payload (`crates/dilax/src/types.rs`); detectors and publishers expect that schema intact.
- Running passenger state persists under `apc:vehicleIdState:{vehicle_id}`; `update_vehicle_state` handles deduped tokens, trip resets, and occupancy writes—reuse its helpers instead of reimplementing storage.
- `KvStore` set helpers (`add_to_set`, `set_expiry`) back per-day dedupe sets; always pair them so restarts remain idempotent.

## Lost Connection Detection
- `DilaxLostConnectionsDetector` (`crates/dilax/src/detector.rs`) caches day-of allocations (filters diesel `ADL*`) using the injected `Clock` trait for deterministic tests and timezone control (`Config::timezone`, default `Pacific/Auckland`).
- Alerts dedupe via `apc:lostConnections{yyyymmdd}` plus detail keys—logics assume `VehicleTripInfo` matches the processor snapshot.
- When extending detection, maintain cache refresh cadence and reuse `set_members`/`set_json_with_ttl` so retention windows stay consistent.

## HTTP Provider Pattern
- Provider traits in `crates/dilax/src/api.rs` abstract outbound HTTP; the host implements `HttpRequest::fetch` atop `sdk_http::Client`, allowing the WASM guest to stay async.
- Fleet/GTFS providers cache successes for 24h and short misses for minutes (`FLEET_SUCCESS_TTL`, etc.); respect these constants when adding endpoints or altering keys.
- `CcStaticProvider::stops_by_location` issues JSON GETs with 150 m radius and returns minimal `StopInfo` structs—mirror headers (`Accept`, `Content-Type`) to avoid 415 responses.

## Messaging & Configuration
- `src/config.rs` centralises environment defaults for Fleet, BlockMgt, GTFS, and Kafka topics; thread new settings through strongly typed configs instead of reading env vars ad hoc.
- `Messaging::configure()` subscribes to both R9K and Dilax source topics; publishing uses `config::get_dilax_outbound_topic()` with message keys matching `trip_id`/device identifiers.
- Instrument new async code with `sdk_otel::instrument` and increment `monotonic_counter.*` metrics so existing dashboards remain accurate.

## Build & Test Workflows
- `make build`, `make test`, and `make check` run cargo-make (fmt via nightly, clippy, audit, machete) with `RUSTFLAGS=-Dwarnings` enforced.
- CI-grade testing prefers `cargo nextest run --all --no-fail-fast --all-features`; build the deployable guest with `cargo build --package train --target wasm32-wasip2 --release`.
- `compose.yaml` expects `target/wasm32-wasip2/release/r9k_position.wasm`; align environment values with the Confluent defaults documented in `README.md` when running the stack locally.

## Legacy References & Conventions
- Behaviour mirrors `legacy/at_dilax_adapter`; consult it for migration shims, stop resolution, and occupancy intent before diverging.
- Follow `crates/r9k-position` patterns for provider mocks and integration-style tests to keep the Dilax rewrite consistent with the existing pipeline.
- Error propagation should use `anyhow::Context` and `realtime::bad_gateway!`; keep logs ASCII with `vehicle_id`, `trip_id`, and `token` fields for downstream alerting.
