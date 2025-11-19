# Stage 1: Context Gathering Prompt

## Purpose
Guide the migration agent to enumerate and isolate only the business logic and direct dependencies of the target TypeScript module, referencing the codebase structure and conventions in `.github/copilot-structure.md`.

---

## Prompt
You are GPT-5 Codex acting as a Rust migration agent.
Target module: <legacy path>
Reference Rust crate(s): <crates/r9k-adapter>, <optional additional>
Follow Stage 1 in `Staged-Migration.md` strictly. Reference `.github/copilot-structure.md` for architecture, provider pattern, and conventions.

**Stage 1 Request:**
Enumerate ONLY direct module files and their imports. Output:
1. File table (file | purpose | keep/maybe/exclude)
2. Business logic bullet list (≤10)
3. Side-effect surface mapped to provider traits (HTTP, Publisher, StateStore, Identity). If missing trait, propose extension.

**Acceptance Criteria:**
- No unrelated files present.
- All IO mapped to existing or new provider trait methods.
- All domain types identified.

**Instructions:**
- Do not include any code not listed in the file table.
- Limit summary to 10 bullets.
- Wait for explicit approval before proceeding to Stage 1.5.
- Reference `.github/copilot-structure.md` for all architectural and provider pattern details.

---

READY FOR REVIEW
