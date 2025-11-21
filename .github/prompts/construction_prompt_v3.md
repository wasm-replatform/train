# Construction Prompt V3 --- Fully Refined for IR V12

## 1. IR Version Alignment

The generator must treat the provided TEXT_IR as V12 IR with sections: -
Algorithm - Preconditions - Postconditions - Edge Cases - Complexity
Notes - Tagging - Explicit Constants - Unknowns All must be consumed and
implemented.

## 2. Interpretation of IR Tags

## 3. Algorithm Fidelity

Implement each algorithm step sequentially. Preconditions become checks;
postconditions inform outputs; edge cases branch accordingly.

## 4. Unknown Handling Rules

unknown --- not present in source =\> TODO. unknown --- external symbol
=\> TODO. unknown --- ambiguous control flow =\> TODO. unknown ---
semantics missing =\> TODO.

## 5. WASI Mapping Rules

HTTP =\> wasi::http Messaging =\> wasi::messaging Key Vault =\>
wasi::keyvalue Secrets =\> wasi::keyvalue or wasi::secrets Identity =\>
wasi::identity Filesystem =\> wasi::filesystem Environment =\>
wasi::config

## 6. Type Construction Rules

Generate Rust structs and enums for domain objects. Map
camelCase→snake_case, PascalCase→PascalCase. Unknown fields =\> TODO.

## 7. Algorithm Order Enforcement

Follow IR order exactly; no reordering.

## 8. Behavioral Equivalence

Match IR behavior exactly: branches, constants, comparisons, errors.

## 9. WASM Safety

No host networking, filesystem, env, threads, blocking I/O, or unsafe
unless IR demands.

## 10. Naming Conversion

camelCase→snake_case, PascalCase→PascalCase, CONSTANT→CONST.

## 11. File Layout

types.rs, providers.rs, module.rs. Domain logic separated cleanly.

## 12. Error Handling

IR errors → Rust enums. Throwing → Result::Err.

## 13. TODO Review Rules

List all TODOs after generation and ask for clarification if blocking.
