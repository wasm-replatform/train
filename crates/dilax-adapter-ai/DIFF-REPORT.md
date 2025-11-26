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

