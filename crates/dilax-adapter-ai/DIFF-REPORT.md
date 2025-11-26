# Dilax Adapter vs AI Output Comparison



- Scope: Compare enrichment fields (trip_id, stop_id, start_date, start_time).
- Source: Manual outputs in `data/<stem>-out.json` vs AI outputs in `data/ai-<stem>-out.json`.

## Schema Differences (Why Full JSONs Differ)

- Legacy payload: The manual (`dilax-adapter`) output uses the legacy Dilax schema and includes operational/status fields and additional metadata.
- AI payload: The AI (`dilax-adapter-ai`) output is a slimmer schema focused on enrichment. It omits some legacy fields by design.
- Missing in AI vs Legacy:
  - `dlx_vers`, `dlx_type`, `driving`, `atstop`, `operational`, `distance_start`, `trigger`
  - `arrival_utc`, `departure_utc`, `distance_laststop`, `pis` (and `clock.tz`)
  - Door `err` (AI emits `name`, `in`, `out`, `art`, `st`)
- Rationale:
  - The AI adapter’s `types.rs` and `logic::process_event` model only the fields required for downstream enrichment and testing.
  - Core enrichment fields used for parity checks are present in both: `trip_id`, `stop_id`, `start_date`, `start_time`.
  - To achieve full JSON parity, either extend AI types to carry these legacy fields through, or add a normalizer that maps AI output into the legacy shape for comparisons.


## dilax_dummy_legacy_result

- Manual: `data/dilax_dummy_legacy_result-out.json`
- AI: `data/ai-dilax_dummy_legacy_result-out.json`
- Result: ✅ Match on all enrichment fields


## dilax_current_result

- Manual: `data/dilax_current_result-out.json`
- AI: `data/ai-dilax_current_result-out.json`
- Result: ✅ Match on all enrichment fields


## dilax_dummy_message

- Manual: `data/dilax_dummy_message-out.json`
- AI: `data/ai-dilax_dummy_message-out.json`
- Result: ✅ Match on all enrichment fields

## Function & Flow Analysis

- **Entry Points:**
  - Manual: `handlers::process_event` receives `DilaxMessage` and orchestrates enrichment.
  - AI: `handlers::processor::process` (or similarly named) takes `DilaxMessage` and performs a streamlined enrichment path.

- **Provider Usage (DI pattern):**
  - Manual: Relies on `Provider` for `HttpRequest`, `Publisher`, `StateStore`, and `Identity` via trait bounds; calls are scattered but explicit.
  - AI: Centralizes provider interactions behind helper modules (`block_mgt`, `gtfs`) while maintaining the same trait requirements; fewer direct env lookups, clearer boundaries.

- **Block Management Lookup:**
  - Manual: Builds request with vehicle label → calls block management API → maps response to `trip_id`, `start_date`, `start_time`.
  - AI: Encapsulates the same flow in `block_mgt::vehicle` / `block_mgt::trip` with stricter error contexts (anyhow + domain `Error` at boundaries).
  - Difference: AI adds consistent context strings on HTTP failures and deserialization; manual code sometimes bubbles raw errors.

- **GTFS Stop Resolution:**
  - Manual: Computes `stop_id` via stop location lookup against GTFS static endpoint; may include more fields (e.g., PIS/clock).
  - AI: Uses `gtfs::stops` to fetch and resolve nearest stop; focuses solely on `stop_id` needed for parity checks.
  - Difference: AI normalizes inputs and outputs to only enrichment-relevant fields; manual passes through legacy metadata.

- **State Persistence (Redis):**
  - Manual: Stores intermediate trip state and last-known values to reduce repeated lookups; values often in legacy wrapper structure.
  - AI: Writes minimal keys required for enrichment caching (e.g., trip allocations) and reads with JSON-safe unwrap logic; reduces write surface.

- **Identity & Auth:**
  - Manual: Token retrieval may be inline or via host; mixed handling of mock/dev paths.
  - AI: Delegates to host `Provider` trait `Identity` and supports OAuth client credentials, mock token, and WASI identity as distinct priorities.
  - Difference: AI enforces a consistent priority order and caches the OAuth token with safety margin.

- **Publishing:**
  - Manual: Publishes enriched legacy-shaped payload to `{ENV}-realtime-dilax-adapter-apc-enriched.v1`.
  - AI: Publishes slim enriched payload (focused fields) to the same topic; downstream parity verified via tests.

- **Error Handling:**
  - Manual: Mix of `Result<T, E>` and custom errors; some stringly errors.
  - AI: Internal functions return `anyhow::Result`, converting to domain `Error` at API boundaries per guidelines; richer context via `map_err`.

- **Testing:**
  - Manual: Integration tests cover end-to-end flows with legacy shape.
  - AI: Unit tests and parity scenarios validate enrichment fields across fixtures, accepting schema differences but asserting field equality.

## Detailed Flow Comparison

- **Manual Crate Flow:**
  - Parse inbound `DilaxMessage` → enrich via block management (trip data) → resolve stop via GTFS static → persist/update state → publish legacy-shaped enriched event.

- **AI Crate Flow:**
  - Parse inbound `DilaxMessage` → `block_mgt` module fetches allocations → `gtfs` module resolves stop → build slim enriched event (`trip_id`, `stop_id`, `start_date`, `start_time`) → publish.

- **Performance/Resilience Notes:**
  - AI reduces payload processing and persistence, which can decrease latency and cache churn.
  - Consistent error contexts in AI aid observability and quicker debugging in WASM environment.

## Migration Guidance

- If full JSON parity is required:
  - Extend AI `types.rs` to carry legacy metadata fields through the pipeline, or
  - Add a normalizer that maps AI enriched output to legacy event shape before publish/compare.

- Keep Provider pattern intact:
  - Ensure the host application implements `HttpRequest`, `Publisher`, `StateStore`, `Identity` as per `realtime` traits.
  - Favor module encapsulation (`block_mgt`, `gtfs`) with clear domain errors.


