
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

### Stage 2: Context Refinement
Objective: Formalize an abstract API contract & test intent before translation.
Actions:
1. Produce a code signature: public functions, inputs, outputs, error cases.
2. Normalize types (TS → conceptual Rust: interface → struct, enum stays enum, union → enum with variants).
3. Summarize existing tests (or inferred test scenarios if absent) as Given/When/Then cases.
Deliverables:
- API signature block.
- Type mapping table.
- Test scenario list (minimum: happy path, validation failure, external dependency failure, edge case data boundary).
Acceptance Criteria:
- Every public function has at least one test scenario.
- Each scenario includes expected result or error variant.
- Type mappings unambiguous (no TBD entries).

### Stage 3: Conversion
Objective: Create Rust implementation segregating business logic from infrastructure, aligned with provider pattern.
Actions:
1. Scaffold crate/module layout mirroring reference adapter structure.
2. Implement domain types with `serde` derives where serialization occurs.
3. Port pure computations first (no IO).
4. Introduce provider-backed integration functions (HTTP, Kafka, StateStore, Identity) via `impl Provider` parameters.
5. Implement error enum; add `From<anyhow::Error>` if chaining context.
6. Write Rust tests using mock provider; mirror Stage 2 test scenarios.
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

Stage 2 Request (after approval):
Produce API signature, type mapping table, test scenario list (Given/When/Then). Validate coverage (≥1 scenario per public function).
Then WAIT for approval.

Stage 3 Request (after approval):
Generate crate scaffold (list planned Rust files). Implement pure logic first. Provide patch blocks. Provide mock provider and tests. STOP when tests compile logically (no execution yet). Await run confirmation.

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
2. Test suite covers all public functions + failure modes.
3. Clippy + fmt pass; justified allows documented.
4. Migration report completed & stored (`migration-<module>.md`).
5. No TODO/FIXME remaining in migrated code.
6. Provider usage consistent; no orphan direct IO calls.

---
## Anti-Patterns (Reject On Sight)
- "Big bang" one-shot full conversion without staged artifacts.
- Mixing infra setup code (Docker, pipeline YAML) in migration patch.
- Introducing global mutable state for caching (use provider/state store).
- Silent error swallowing (`_ = func()` without handling).

---
## Quick Reference Checklist (Pre-Commit)
- [ ] Stages 1–4 accepted
- [ ] All tests green (`cargo make test`)
- [ ] `cargo make check` clean
- [ ] Public APIs documented
- [ ] Migration report added
- [ ] No direct WASI calls in business layer

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