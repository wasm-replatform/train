
# Staged Migration Playbook: TypeScript → Rust (wasm32-wasip2)

Purpose: Provide a prescriptive, reviewable, repeatable workflow and prompt framework for migrating individual legacy Node.js/TypeScript modules into Rust crates inside the train WASM codebase. This document is written for a human engineer and GPT-5 Codex acting as a migration agent.

Sources to always reference:
- Architecture & structure: `.github/copilot-structure.md`
- Conventions & provider pattern: `.github/copilot-instructions.md`
- Legacy source to migrate: `legacy/<service>/...`
- Reference Rust patterns: `crates/r9k-adapter`, `crates/dilax-adapter`, `crates/realtime`

---
## Guiding Principle
Break work into small, atomically reviewable steps. Each stage must produce explicit artifacts and pass defined acceptance criteria before proceeding.

---
## IR Schema Definition

The Intermediate Representation (IR) schema serves as a structured, machine-readable artifact capturing the complete semantic model of a legacy TypeScript service. It bridges Stage 1 (prose analysis) and Stage 3 (Rust implementation), enabling precise recall and validation.

**IR Schema Structure (JSON):**
```json
{
  "module": "string (legacy service name)",
  "version": "1.0",
  "entrypoints": [
    {
      "type": "http_handler | kafka_consumer | timer_trigger",
      "path_or_topic": "string (HTTP path or Kafka topic pattern)",
      "handler_function": "string (TypeScript function name)",
      "input_schema": { /* JSON Schema or inline type definition */ },
      "output_destinations": ["string (Kafka topics or HTTP response)"],
      "error_responses": ["string (HTTP status codes or error topic)"]
    }
  ],
  "business_functions": [
    {
      "name": "string (function name)",
      "visibility": "public | private",
      "inputs": [{"name": "string", "type": "string", "nullable": "boolean"}],
      "outputs": {"ok": "string (success type)", "err": "string (error type)"},
      "side_effects": ["http_request | state_store_read | state_store_write | publish_message | time_source"],
      "provider_traits": ["HttpRequest | Publisher | StateStore | Identity"],
      "logic_summary": "string (1-2 sentence description)",
      "edge_cases": ["string (boundary conditions, null handling, etc.)"]
    }
  ],
  "data_types": [
    {
      "name": "string (type name)",
      "kind": "struct | enum | type_alias",
      "fields_or_variants": [
        {"name": "string", "type": "string", "required": "boolean", "default": "any"}
      ],
      "serialization": "json | xml | none",
      "validation_rules": ["string (regex patterns, range constraints, etc.)"]
    }
  ],
  "error_cases": [
    {
      "variant": "string (Rust enum variant name)",
      "typescript_origin": "string (throw site or error class)",
      "trigger_condition": "string (when this error occurs)",
      "recovery_strategy": "string (retry | fail | log_and_continue)"
    }
  ],
  "external_dependencies": [
    {
      "service": "string (Block Management API, GTFS API, Redis, etc.)",
      "operations": ["string (GET /allocations/trips, SET cache:key, etc.)"],
      "timeout_ms": "number",
      "retry_policy": "string (none | exponential_backoff | fixed_interval)",
      "provider_trait": "string (which trait provides this capability)"
    }
  ],
  "constants": [
    {"name": "string", "value": "any", "usage": "string (where/why used)"}
  ],
  "test_scenarios": [
    {
      "name": "string (test case name)",
      "given": "string (preconditions)",
      "when": "string (action)",
      "then": "string (expected outcome)",
      "covers_function": "string (business function name)",
      "error_variant": "string | null (if testing error case)"
    }
  ]
}
```

**Rationale:**
- **Precision**: Eliminates ambiguity in Stage 3 by providing explicit type mappings and error cases.
- **Auditability**: Enables automated diff between IR and Rust AST to catch omissions.
- **Continuity**: Persistent artifact allows multi-session migrations without re-analysis.
- **Validation**: Can verify TypeScript extraction completeness before Rust conversion begins.

**When to Use IR:**
- **Mandatory**: Services with >5 source files, complex state machines, or multiple external dependencies.
- **Optional**: Simple services (≤3 files, single input/output pattern) may skip to preserve velocity.

---
## Roles & Responsibilities
- Human Engineer: Chooses target module, supplies context, approves stage outputs, enforces quality gates.
- GPT-5 Codex (Migration Agent): Generates summaries, scaffolds Rust code, ports logic, writes tests, lists deviations, proposes improvements.

---
## Stage Overview (Definition + Acceptance Criteria)

### Stage 1: Context Gathering
Objective: Isolate only the business logic and direct dependencies of the target TypeScript module.
Actions:
1. Enumerate involved files (module + its direct imports only; exclude infra wrappers, deployment, config noise).
2. Extract business-only functions (exclude HTTP wiring, Kafka client setup, DI container bootstraps).
3. Capture domain constants, enums, interfaces, error shapes.
4. List external side effects (HTTP, Redis, Kafka, FS, time, env).
Deliverables:
- File list table.
- Business logic summary (≤ 10 bullets).
- Side-effect surface list mapped to provider traits.
Acceptance Criteria:
- No unrelated files present.
- All IO mapped to existing or new provider trait methods.
- All domain types identified.

### Stage 1.5: IR Schema Extraction
Objective: Produce a machine-readable intermediate representation capturing all semantic elements from Stage 1 analysis.
Actions:
1. Generate IR JSON file following schema definition above.
2. Populate `entrypoints` from Stage 1 side-effect analysis (HTTP routes, Kafka consumers, timers).
3. Map all TypeScript functions to `business_functions` with complete signatures and side-effect annotations.
4. Extract data types from TypeScript interfaces/types/enums to `data_types` section.
5. Enumerate error cases from throw statements, error classes, and implicit failures (null checks, API errors).
6. Document external dependencies with timeout/retry policies if discoverable from code.
7. Generate test scenarios covering all public functions + error paths.
Deliverables:
- `ir-<module-name>.json` file (e.g., `ir-dilax-adapter.json`).
- Validation report confirming IR completeness against Stage 1 file list.
Acceptance Criteria:
- IR validates against schema (no missing required fields).
- Every TypeScript function from Stage 1 appears in `business_functions` or `entrypoints`.
- All `side_effects` mapped to `provider_traits` (no unmapped IO).
- Error cases include trigger conditions and recovery strategies.
- Test scenarios achieve ≥1 per public function + ≥1 per error variant.
- Can regenerate Stage 1 prose summary from IR (round-trip verification).

### Stage 2: Context Refinement
Objective: Formalize an abstract API contract & test intent before translation, validating against IR schema.
Actions:
1. Validate IR schema completeness (if Stage 1.5 executed).
2. Produce a code signature: public functions, inputs, outputs, error cases (derived from IR `business_functions`).
3. Normalize types (TS → conceptual Rust: interface → struct, enum stays enum, union → enum with variants) using IR `data_types`.
4. Confirm test scenarios from IR cover all public functions and error variants.
Deliverables:
- API signature block (can be generated from IR).
- Type mapping table (TS type → Rust type, referencing IR).
- Test scenario confirmation (validate IR `test_scenarios` section).
Acceptance Criteria:
- Every public function has at least one test scenario.
- Each scenario includes expected result or error variant.
- Type mappings unambiguous (no TBD entries).
- If IR exists, all deliverables traceable to IR sections.

### Stage 3: Conversion
Objective: Create Rust implementation segregating business logic from infrastructure, aligned with provider pattern, using IR as implementation reference.
Actions:
1. Scaffold crate/module layout mirroring reference adapter structure.
2. Implement domain types from IR `data_types` with `serde` derives where serialization occurs.
3. Port pure computations first (no IO), using IR `business_functions` with `side_effects: []`.
4. Introduce provider-backed integration functions (HTTP, Kafka, StateStore, Identity) via `impl Provider` parameters, mapping IR `provider_traits`.
5. Implement error enum from IR `error_cases`; add `From<anyhow::Error>` if chaining context.
6. Write Rust tests using mock provider; implement all IR `test_scenarios`.
Deliverables:
- New Rust source files (business + adapter layer).
- Error enum + `Result<T>` alias.
- Test module(s) with mocks.
Acceptance Criteria:
- All test scenarios implemented and pass locally.
- No direct WASI calls in business functions (only through traits).
- Public API documented with `///` comments.
- Clippy passes for crate scope (excluding justified WASM async allowances).

### Stage 4: Validation & Documentation
Objective: Confirm behavioral parity and document deviations & improvements.
Actions:
1. Run test suite (`cargo make test`).
2. Produce a migration report section: parity, deviations, rationale, follow-ups.
3. List performance/robustness improvements (e.g., explicit error variants, reduced allocations, removal of implicit any).
Deliverables:
- Migration report appended to this playbook or generated `migration-<module>.md`.
- Summary table (TS vs Rust constructs).
Acceptance Criteria:
- All tests green.
- No unresolved TODOs or `unwrap()` in business paths.
- Report includes deviation justifications.

---
## Prompt Style Guidelines (Directive Language)
Use imperative verbs, deterministic phrasing, and bounded output requests.
Patterns:
- "Enumerate only ..."
- "Produce a table with columns: ..."
- "Do not include any code not listed in the file table." 
- "Limit summary to N bullets." 
Avoid vague phrasing like "can you" / "maybe" / "some".
Require explicit acceptance checks: "Confirm all mapped provider traits before proceeding." 

---
## Canonical Migration Prompt Template
```text
You are GPT-5 Codex acting as a Rust migration agent.
Target module: <legacy path>
Reference Rust crate(s): <crates/r9k-adapter>, <optional additional>
Follow stages 1→4 in `Staged-Migration.md` strictly. Do not skip acceptance criteria.

Stage 1 Request:
Enumerate ONLY direct module files and their imports. Output:
1. File table (file | purpose | keep/maybe/exclude)
2. Business logic bullet list (≤10)
3. Side-effect surface mapped to provider traits (HTTP, Publisher, StateStore, Identity). If missing trait, propose extension.
Then WAIT for approval.

Stage 1.5 Request (after Stage 1 approval, MANDATORY for services >5 files):
Generate IR schema JSON following the definition in `Staged-Migration.md`. Include:
1. All entrypoints (HTTP/Kafka/timers) with input/output schemas
2. Complete business_functions list with side_effects and provider_traits
3. All data_types extracted from TypeScript interfaces/enums
4. Error cases with trigger conditions and recovery strategies
5. External dependencies with timeout/retry policies
6. Test scenarios (≥1 per public function, ≥1 per error variant)
Validate completeness: confirm every Stage 1 function appears in IR.
Output: `ir-<module-name>.json` file content.
Then WAIT for approval.

Stage 2 Request (after Stage 1.5 approval, or Stage 1 if IR skipped):
Produce API signature, type mapping table, test scenario list (Given/When/Then). If IR exists, derive from IR schema; otherwise generate from scratch. Validate coverage (≥1 scenario per public function).
Then WAIT for approval.

Stage 3 Request (after approval):
Generate crate scaffold (list planned Rust files). Implement domain types from IR data_types. Implement business functions referencing IR side_effects and provider_traits. Provide patch blocks. Provide mock provider and tests implementing all IR test_scenarios. STOP when tests compile logically (no execution yet). Await run confirmation.

Stage 4 Request (after tests pass):
Generate migration report: parity matrix, deviations with justification, improvement list, follow-up recommendations.

Rules:
- Use provider traits for all IO.
- Keep infrastructure out of handlers unless entrypoint adapter logic.
- Document all public items.
- Avoid magic numbers: extract constants.
- Replace implicit TS error paths with explicit Rust enum variants.
```

---
## Iteration Protocol
1. Each stage ends with a "READY FOR REVIEW" marker.
2. Human either APPROVES or REQUESTS CHANGES with concrete edits.
3. GPT-5 Codex incorporates feedback; never advances on partial approval.
4. If ambiguity arises, GPT-5 Codex enumerates assumptions before proceeding.

---
## Migration Notes (Cross-Cutting Concerns)
Provider Pattern:
- Central to decoupling IO: `impl Provider` passed into domain functions.
- Extensible: add new trait method only if stage analysis proves necessary.

Environment & Topics:
- Kafka topic prefix uses `ENV` (dev/test/prod): follow `{ENV}-realtime-<service>.v1`.
- Avoid hardcoding; inject via provider or configuration constants.

Error Handling:
- Use domain-specific `Error` enums; map foreign errors with context (`anyhow::Context`).
- No stringly errors; prefer semantic variants (`InvalidFormat`, `ProcessingError`, `NotFound`).

Testing:
- Mirror legacy test intent; if absent, infer from code paths & edge cases.
- Use mock provider capturing publishes & HTTP responses.

Serialization:
- `serde` with explicit field names; avoid skipping fields unless legacy contract demands it.

Performance:
- Avoid unnecessary clones; prefer borrowing.
- Pre-allocate vectors when size known.
- Use `&str` for static keys; avoid `String` churn.

Telemetry:
- Add `#[wasi_otel::instrument]` at adapter entrypoints, not deep pure functions.
- Log contextual IDs (vehicle, trip) at publish boundaries only.

Common Pitfalls:
- Direct WASI calls inside business logic (must go through provider).
- Leaking infrastructure concerns (env var reads) into pure modules.
- Over-expanding traits prematurely.
- Using `unwrap()` in production paths.

---
## Quality Gates / Definition of Done
Must satisfy ALL:
1. Stages executed sequentially with approvals.
2. IR schema generated and validated (if service >5 files or complex).
3. Test suite covers all public functions + failure modes (traceable to IR test_scenarios if IR exists).
4. Clippy + fmt pass; justified allows documented.
5. Migration report completed & stored (`migration-<module>.md`).
6. No TODO/FIXME remaining in migrated code.
7. Provider usage consistent; no orphan direct IO calls.
8. If IR exists: Rust implementation covers all IR business_functions and error_cases.

---
## Anti-Patterns (Reject On Sight)
- "Big bang" one-shot full conversion without staged artifacts.
- Mixing infra setup code (Docker, pipeline YAML) in migration patch.
- Introducing global mutable state for caching (use provider/state store).
- Silent error swallowing (`_ = func()` without handling).

---
## Quick Reference Checklist (Pre-Commit)
- [ ] Stages 1–4 accepted (including Stage 1.5 IR if applicable)
- [ ] IR schema validated (if generated)
- [ ] All tests green (`cargo make test`)
- [ ] `cargo make check` clean
- [ ] Public APIs documented
- [ ] Migration report added
- [ ] No direct WASI calls in business layer
- [ ] Rust implementation covers all IR business_functions (if IR exists)

---
## Example Stage 1 Review Comment Template
```
APPROVED WITH NOTES:
- Add missing side-effect for time source (monotonic vs wall clock).
- Confirm if retry logic exists in legacy; if yes, represent explicitly.
Proceed to Stage 2.
```

---
## Guidance Recap (Minimal)
Always: small steps, explicit artifacts, provider abstraction, semantic errors, documented deviations.

READY FOR USE