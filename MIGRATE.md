# Migration

## Code conversion

1. Add template workspace (/src, Cargo.toml, deny.toml, clippy.toml, etc) to project root
2. Add .github/copilot-instructions.md
3. Add related legacy projects to the legacy/ directory (without .git directories)

In Copilot chat, set to `Agent` mode and use the `GPT-5-Codex` model with the following instructions:

```text
#file:copilot-instructions.md
Convert #file:legacy/at_dilax_adapter to Rust
Use #file:crates/r9k-position as a reference implementation
Use code guidelines #fetch https://microsoft.github.io/rust-guidelines/index.html
```

