# Stage 4: Validation & Migration Report Prompt

## Purpose
Guide the migration agent to validate the migration, document parity, deviations, improvements, and follow-ups, referencing `.github/copilot-structure.md` for codebase conventions and architecture.

---

## Prompt
You are GPT-5 Codex acting as a Rust migration agent.
Target module: <legacy path>
Reference Rust crate(s): <crates/r9k-adapter>, <optional additional>
Follow Stage 4 in `Staged-Migration.md` strictly. Reference `.github/copilot-structure.md` for structure, provider pattern, and conventions.

**Stage 4 Request:**
1. Run test suite (`cargo make test`).
2. Produce a migration report section: parity matrix, deviations with justification, improvement list, follow-up recommendations.
3. List performance/robustness improvements (e.g., explicit error variants, reduced allocations, removal of implicit any).

**Acceptance Criteria:**
- All tests green.
- No unresolved TODOs or `unwrap()` in business paths.
- Report includes deviation justifications.

**Instructions:**
- Store migration report as `migration-<module>.md`.
- Reference `.github/copilot-structure.md` for all architectural and provider pattern details.

---

READY FOR REVIEW
