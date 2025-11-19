# Stage 1.5: IR Schema Extraction Prompt

## Purpose
Direct the migration agent to generate a complete, machine-readable IR schema for the target module, referencing `.github/copilot-structure.md` for codebase conventions and architecture.

---

## Prompt
You are GPT-5 Codex acting as a Rust migration agent.
Target module: <legacy path>
Reference Rust crate(s): <crates/r9k-adapter>, <optional additional>
Follow Stage 1.5 in `Staged-Migration.md` strictly. Reference `.github/copilot-structure.md` for structure, provider pattern, and conventions.

**Stage 1.5 Request:**
Generate IR schema JSON following the definition in `Staged-Migration.md`. Include:
1. All entrypoints (HTTP/Kafka/timers) with input/output schemas
2. Complete business_functions list with side_effects and provider_traits
3. All data_types extracted from TypeScript interfaces/enums
4. Error cases with trigger conditions and recovery strategies
5. External dependencies with timeout/retry policies
6. Test scenarios (≥1 per public function, ≥1 per error variant)

Validate completeness: confirm every Stage 1 function appears in IR.
Output: `ir-<module-name>.json` file content.

**Acceptance Criteria:**
- IR validates against schema (no missing required fields).
- Every TypeScript function from Stage 1 appears in `business_functions` or `entrypoints`.
- All `side_effects` mapped to `provider_traits` (no unmapped IO).
- Error cases include trigger conditions and recovery strategies.
- Test scenarios achieve ≥1 per public function + ≥1 per error variant.
- Can regenerate Stage 1 prose summary from IR (round-trip verification).

**Instructions:**
- Wait for explicit approval before proceeding to Stage 2.
- Reference `.github/copilot-structure.md` for all architectural and provider pattern details.

---

READY FOR REVIEW
