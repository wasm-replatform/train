# Dilax Adapter: Manual vs AI Implementation – Detailed Differences

This document compares the manually converted Dilax adapter (`crates/dilax-adapter`) against the AI-generated adapter (`crates/dilax-adapter-ai`). It highlights implementation differences and their behavioral impact across configuration, handlers, payloads, enrichment logic, state/Redis, lost-connections, publishing, and error semantics.

## Configuration

- **Env Vars:**
  - Manual: `FLEET_URL`, `BLOCK_MGT_URL`, `CC_STATIC_URL`.
  - AI: `FLEET_API_URL`, `BLOCK_MGT_CLIENT_API_URL`, `CC_STATIC_API_HOST`.
- **Defaults:**
  - AI introduces defaults (TTLs, timezone, search radius, diesel train prefix) and a centralized `Config` loader.
- **Behavior Impact:**
  - AI fails to start in current compose unless env names are aligned. Manual works with existing envs.

## Provider Pattern

- **Traits:** Both define `Provider: HttpRequest + Publisher + StateStore + Identity`.
- **Wrapper:** AI adds `ProviderWrapper` to centralize HTTP/state/token access and expose `Config`. Manual performs env lookups and I/O directly in modules.
- **Behavior Impact:** AI consolidates I/O and configuration validation; manual scatters env usage.

## Handlers

- **Message Handling:**
  - Manual: `processor::process(DilaxMessage, &impl Provider) -> Result<()>` and always attempts to publish enriched event.
  - AI: `handlers::handle_message` loads `Config`, runs `process_event`, publishes only when enrichment returns `Some(enriched)`.
- **Detection Handling:**
  - Manual: `detector` filters allocations, performs dedup with Redis set + per-day envelope; persists detections.
  - AI: `handle_detection` runs `fetch_allocations_for_today` + `detect_lost_connections`, returns candidates without dedup persistence.
- **Behavior Impact:** AI may quietly skip publishing on partial failures; manual either publishes or surfaces an error. AI detection may re-report identical lost connections on each run.

## Payload and Types

- **Dilax Message Schema:**
  - Manual `types::DilaxMessage` mirrors legacy payload: includes `dlx_vers`, `dlx_type`, movement flags, `pis`, `doors`, `speed`, `wpt`, etc.
  - AI `types::DilaxEvent` is reduced (device/clock/doors/wpt only).
- **Enriched Event:** Both include `stop_id`, `trip_id`, `start_date`, `start_time`, but AI wraps the reduced event.
- **Behavior Impact:** AI changes the on-wire JSON shape (drops fields), breaking parity for downstream consumers expecting the full legacy schema.

## Vehicle Label Parsing

- **Logic:** Both map `AM→AMP`, `AD→ADL`, and pad to width 14 before appending digits.
- **Implementation:** Manual uses a character-run splitter; AI uses `Regex` with `OnceLock`.
- **Behavior Impact:** Equivalent outcomes for typical labels; differences only in implementation style.

## Fleet Vehicle Lookup

- **Manual:** `FLEET_URL/vehicles?label=...` with caching headers (`CACHE_CONTROL`, `IF_NONE_MATCH`); selects train vehicles; capacity required (seating/total as `i64`).
- **AI:** `FLEET_API_URL/vehicles?label=...` without caching headers; filters train by type; capacity fields are `Option<u32>` and can be missing.
- **Behavior Impact:** AI may perform more HTTP calls (no cache hints) and returns `None` if capacity missing; manual treats missing capacity as an error.

## Block Management Allocation

- **Manual:** Strongly typed `VehicleAllocation` (non-optional critical fields), envelope `{ current, all }`, fetches `BLOCK_MGT_URL`.
- **AI:** Parses `{ current: [...] }` from `BLOCK_MGT_CLIENT_API_URL`; `VehicleAllocation` uses optional `trip_id/service_date/start_time`.
- **Behavior Impact:** AI handles absent current trip gracefully; manual assumes presence and errors on absence, aiding explicit failure visibility.

## GTFS / Stops

- **Manual:**
  - Location stops via `CC_STATIC_URL/gtfs/stops/geosearch`.
  - Stop types via `GTFS_STATIC_URL/stopstypes/` with train filtering; station detection by matching `parent_stop_code == stop_code`.
- **AI:**
  - Location stops via `CC_STATIC_API_HOST/gtfs/stops/geosearch` with configurable radius.
  - Stop types via `GTFS_STATIC_URL/stopstypes/` with flexible JSON parsing; returns a `message` on errors.
- **Behavior Impact:** AI returns `None` for stop when stop-types fetch fails; manual raises `ProcessingError`. AI is more forgiving, potentially suppressing enrichment.

## Trip State & Redis

- **Manual:**
  - `TripState { count: i64, token: i64, last_trip_id, occupancy_status }`.
  - Backwards-compat migration of legacy keys (`apc:trips`, `apc:vehicleId`).
  - Writes `trip:occupancy` (string code), `apc:vehicleId` (count), `apc:vehicleIdState` (JSON), and `apc:vehicleTripInfo`.
- **AI:**
  - `DilaxState` with `PassengerCount`/`MessageToken` newtypes and `OccupancyStatus` enum→code; `DilaxStateRecord` persisted.
  - Uses `RedisKeys` for namespacing and configurable TTLs.
- **Behavior Impact:** AI’s state JSON differs in shape (newtypes and field names). Existing readers expecting manual JSON may break.

## Occupancy Calculation

- **Manual:** Integer thresholds based on seating/total using `occupancy_threshold`.
- **AI:** Floating-point thresholds with truncation; introduces additional enum values (not used in main logic).
- **Behavior Impact:** Minor boundary differences could shift categories near thresholds.

## Lost Connections

- **Manual:**
  - Filters allocations by service date and excludes diesel (`ADL`).
  - Detects loss when `last_timestamp + 1h <= now` (Auckland TZ); persists dedup in Redis (`apc:lostConnectionsYYYYMMDD` envelope + per-trip members with TTL).
  - Logs detailed detection context (vehicle label, coordinates, formatted timestamp).
- **AI:**
  - Similar allocation filtering and detection threshold via `connection_lost_threshold_mins`.
  - Returns candidates only; a TODO notes missing Redis set operations for dedup.
- **Behavior Impact:** AI will re-emit identical detections each run and lacks persisted audit keys; operational noise and reduced observability.

## Publishing

- **Topic:** Both use `realtime-dilax-adapter-apc-enriched.v1`.
- **Key Header:** Both set `key` from `trip_id` when present.
- **Behavior Impact:** Manual publishes after enrichment regardless; AI publishes only if enrichment succeeds (`Some(enriched)`), reducing message volume under partial failure.

## Error Handling

- **Manual:** Rich `Error` enum with many codes; `From<anyhow::Error)` builds a cause chain; errors propagate to handlers.
- **AI:** Splits into `DomainError` (InvalidEvent, ProcessingError, StateConflict) and `Error` (Domain/System); prefers `Ok(None)` or `Ok(Some(...))` with logs over raising errors.
- **Behavior Impact:** AI quiets failures, reducing backpressure and error signals; manual surfaces failures explicitly.

## Compatibility Risks

- **Payload Schema:** AI’s reduced event removes legacy fields, breaking downstream systems expecting parity.
- **Env Var Names:** AI’s config won’t load in current environment without renaming.
- **Redis State Shape:** AI’s `DilaxStateRecord` differs from manual JSON; readers may fail.
- **Lost-Connections Dedup:** AI lacks Redis set persistence, causing duplicate detections and missing audit trail.

## Recommendations

- **Align Env Vars:** Update AI to use manual names (`FLEET_URL`, `BLOCK_MGT_URL`, `CC_STATIC_URL`) or update compose/env to AI names. Prefer aligning AI for minimal changes.
- **Restore Full Payload:** Match AI `DilaxEvent` to manual `DilaxMessage` to preserve on-wire compatibility.
- **Reintroduce Dedup Persistence:** Implement Redis set/envelope for lost-connections (extend `StateStore` or replicate manual envelope with JSON + TTL).
- **Normalize Occupancy:** Use manual integer threshold math to match legacy behavior.
- **Error Strategy:** Consider surfacing `ProcessingError` on core enrichment failures (vehicle/stop/trip) to improve observability, or add robust metrics if keeping tolerant behavior.
