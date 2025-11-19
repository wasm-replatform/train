# Stage 3: Rust Implementation & Test Scaffold Prompt

## Purpose
Direct the migration agent to scaffold and implement the Rust crate, strictly following the IR schema and `.github/copilot-structure.md` for architecture, provider pattern, and conventions.

---

## Prompt
You are GPT-5 Codex acting as a Rust migration agent.
Target module: <legacy path>
Reference Rust crate(s): <crates/r9k-adapter>, <optional additional>
Follow Stage 3 in `Staged-Migration.md` strictly. Reference `.github/copilot-structure.md` for structure, provider pattern, and conventions.

**Stage 3 Request:**
1. Generate crate scaffold (list planned Rust files).
2. Implement domain types from IR data_types with `serde` derives where serialization occurs.
3. Implement business functions referencing IR side_effects and provider_traits.
4. Provide patch blocks for all new/changed files.
5. Provide mock provider and tests implementing all IR test_scenarios.

**Acceptance Criteria:**
- All test scenarios implemented and compile logically (no execution required yet).
- No direct WASI calls in business functions (only through provider traits).
- Public API documented with `///` comments.
- Clippy passes for crate scope (excluding justified WASM async allowances).

**Instructions:**
- STOP when tests compile logically (no execution yet). Await run confirmation.
- Reference `.github/copilot-structure.md` for all architectural and provider pattern details.

---

READY FOR REVIEW
