# Stage 2: API Contract & Test Intent Prompt

## Purpose
Guide the migration agent to formalize the abstract API contract and test intent, validating against the IR schema if present, and referencing `.github/copilot-structure.md` for codebase conventions.

---

## Prompt
You are GPT-5 Codex acting as a Rust migration agent.
Target module: <legacy path>
Reference Rust crate(s): <crates/r9k-adapter>, <optional additional>
Follow Stage 2 in `Staged-Migration.md` strictly. Reference `.github/copilot-structure.md` for structure, provider pattern, and conventions.

**Stage 2 Request:**
Produce:
1. API signature block (public functions, inputs, outputs, error cases; derive from IR if available)
2. Type mapping table (TypeScript type → Rust type, referencing IR data_types)
3. Test scenario list (Given/When/Then, referencing IR test_scenarios)

Validate coverage:
- Every public function has at least one test scenario.
- Each scenario includes expected result or error variant.
- Type mappings unambiguous (no TBD entries).
- If IR exists, all deliverables traceable to IR sections.

**Acceptance Criteria:**
- Every public function has at least one test scenario.
- Each scenario includes expected result or error variant.
- Type mappings unambiguous (no TBD entries).
- If IR exists, all deliverables traceable to IR sections.

**Instructions:**
- Wait for explicit approval before proceeding to Stage 3.
- Reference `.github/copilot-structure.md` for all architectural and provider pattern details.

---

READY FOR REVIEW
