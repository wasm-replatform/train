    # IR--Driven Rust Construction Prompt (Final)

## 0 --- Inputs (authoritative)

You will receive these four inputs. Treat them in this precedence order
(higher overrides lower). Each input will be referenced using
square-bracket paths, e.g., `[TEXT_IR_PATH]`, `[TS_SOURCE_DIR]`,
`[RUST_OUTPUT_RULES_PATH]`, `[RUST_EXAMPLE_CRATE_DIR]`.

1.  **TEXT_IR (V10)**\
    Text-based IR file containing authoritative domain logic. Provided
    as `[TEXT_IR_PATH]`.

2.  **TS_SOURCE**\
    Directory containing the original TypeScript project. Used only for
    naming fidelity and structural clarification, never for business
    logic. Provided as `[TS_SOURCE_DIR]`.

3.  **RUST_OUTPUT_RULES**\
    File containing crate structure rules and WASI conversion
    guidelines. Provided as `[RUST_OUTPUT_RULES_PATH]`.

4.  **RUST_EXAMPLE**\
    Canonical Rust provider-wrapper crate that defines module layout and
    patterns (e.g., `realtime`). Provided as `[RUST_EXAMPLE_CRATE_DIR]`.

## 1 --- Core Principles (non-negotiable)

-   TEXT_IR is authoritative.\
-   No hallucination. If TEXT_IR lacks required detail, insert TODO and
    request clarification.\
-   I/O only via WASI providers defined in RUST_OUTPUT_RULES /
    RUST_EXAMPLE.\
-   Never use host-only crates.\
-   Never access environment variables except through mechanisms defined
    in RUST_OUTPUT_RULES.\
-   Stateless WASM-compatible output.\
-   DI-first design using `&impl Provider`.\
-   Typed `thiserror` enums for domain errors.\
-   Follow `realtime` crate patterns exactly.\
-   Microsoft Rust Guidelines + Clippy compliance (`-D warnings` where
    applicable).

## 2 --- TEXT_IR (V10) Interpretation Rules

TEXT_IR contains narrative business logic. Treat:

-   **Business Logic** → exact rules to implement\
-   **Logic Shape** → ordered execution sequence\
-   **Inputs** → fields/parameters referenced\
-   **Outputs** → return structure\
-   **Errors** → domain error mapping\
-   **Descriptions** → module/method intent

Missing types or shapes must be annotated with TODO and clarified.

## 3 --- Rust Code Generation Model

### 3.1 File Layout

Follow RUST_OUTPUT_RULES and match RUST_EXAMPLE structure.

### 3.2 Domain Types

-   Generate structs/enums from IR references.\
-   Use serde derives.\
-   Use chrono where timestamps appear.\
-   TODO if IR lacks type details.

### 3.3 Domain Logic Methods

-   Implement exactly per Business Logic + Logic Shape.\
-   No new behavior.\
-   Pure if no I/O described.

### 3.4 External I/O (WASI Providers)

Map to providers: - HTTP → HttpRequest\
- Publish → Publisher\
- Key/State → StateStore / KeyVault\
- Identity → Identity

Never call host libraries.

### 3.5 WASM Handlers

Generate only if IR describes entrypoints. Follow `realtime` patterns.

### 3.6 Error Model

-   Typed enums with thiserror.\
-   `Result<T, DomainError>` unless RUST_OUTPUT_RULES requires
    `anyhow::Result` at handler boundaries.

## 4 --- WASI Provider Mapping

Use wasi-mapping-guide patterns. One TS I/O → one provider call. No
retries/backoff unless defined.

## 5 --- Environment + Config

Never use `std::env`.\
Config only through RUST_OUTPUT_RULES mechanisms.\
TODO when IR references config missing structure.

## 6 --- Microsoft Rust Guidelines + Clippy

Enforce idiomatic Rust, ownership clarity, zero unnecessary clones,
proper naming conventions, explicitness where helpful.\
Pass Clippy recommended lints unless suppressed by RUST_EXAMPLE.

## 7 --- Ambiguity, Missing Info, Failure Modes

Insert TODO when IR lacks type/behavior details.\
If critical information is missing, output:

``` json
{"error":"generation_impossible","reason":"<brief reason>"}
```

Prefer generating compilable code with TODOs unless impossible.

## 8 --- Post-Generation TODO Review Pass

After producing all code, generate a list of all TODO/IR-MISSING items
and ask the user targeted clarification questions. Do not regenerate
code until clarification is provided.

## 9 --- Final Output Requirements

-   Output concatenated Rust files in canonical crate order per
    RUST_OUTPUT_RULES.\
-   No explanations or prose except TODO summary.\
-   Deterministic, WASM-compatible, DI-driven code only.
