# WASI Mapping Guide for Rust Migration (AI-Focused)

This guide is designed for AI systems performing TypeScript-to-Rust migrations. It provides structured, explicit instructions and examples to automate the migration process. The focus is on reusable patterns, templates, and validation steps to ensure accurate and efficient conversions.

---

## 1. Overview

### Objective
- Automate the migration of TypeScript business logic to Rust using WASI.
- Ensure all external effects (HTTP, Kafka, Redis, Identity) are routed through provider traits.
- Isolate WASI integration to provider implementations and entry points.

### Scope
- Applies to any TypeScript codebase following a provider and publisher structure.
- Includes templates, examples, and validation steps for AI systems.

---

## 2. Provider Pattern (Dependency Injection)

### Purpose
- Abstract IO operations (HTTP, Kafka, Redis, Identity) using provider traits.
- Ensure business logic remains pure and testable.

### Rules and Conventions
- Combine smaller traits (e.g., `HttpRequest`, `Publisher`) into a single `Provider` trait to ensure modularity and flexibility.
- Use `impl<T> Provider for T` to allow any type implementing the required traits to act as a provider.
- All WASI calls must go through the provider implementation.

### Template
```rust
pub trait Provider: HttpRequest + Identity + Publisher {}

impl<T> Provider for T where T: HttpRequest + Identity + Publisher {}
```

### Before (TypeScript):
```typescript
interface Provider {
    httpRequest(url: string, method: string, body?: Uint8Array): Promise<Response>;
    publishMessage(topic: string, message: Uint8Array): Promise<void>;
    getCache(key: string): Promise<Uint8Array | null>;
    setCache(key: string, value: Uint8Array): Promise<void>;
    getIdentityToken(): Promise<string>;
}
```

### After (Rust):
```rust
pub trait Provider: HttpRequest + Identity + Publisher {}

impl<T> Provider for T where T: HttpRequest + Identity + Publisher {}
```

---

## 3. WASI Handler Export

### Purpose
- Define entry points for HTTP and messaging handlers.
- Route requests to domain-specific handlers via provider traits.

### Rules and Conventions
- Each handler should focus on a single task (e.g., processing HTTP requests).
- Instantiate the provider within the handler and pass it to domain logic.
- Use structured error handling to propagate errors back to the caller.

### Template
```rust
use wasip3::http::handler::Guest as HttpHandler;
use wasi_messaging::incoming_handler::Guest as MessagingHandler;

#[wasi_otel::instrument]
pub fn handle_http_request(request: HttpRequest) -> HttpResponse {
    let provider = crate::provider::WasiProvider::new();
    crate::handlers::process_http_request(request, &provider)
}

#[wasi_otel::instrument]
pub fn handle_message(message: KafkaMessage) {
    let provider = crate::provider::WasiProvider::new();
    crate::handlers::process_message(message, &provider)
}
```

### Before (TypeScript):
```typescript
async function handleHttpRequest(request: HttpRequest, provider: Provider): Promise<HttpResponse> {
    return processHttpRequest(request, provider);
}
```

### After (Rust):
```rust
#[wasi_otel::instrument]
pub fn handle_http_request(request: HttpRequest) -> HttpResponse {
    let provider = crate::provider::WasiProvider::new();
    crate::handlers::process_http_request(request, &provider)
}
}
```

---

## 4. Domain Logic Structure

### Purpose
- Organize business logic as methods on domain types.
- Ensure all input types have validation methods.

### Rules and Conventions
- Validate inputs and propagate errors using `Result`.
- Keep domain logic pure; side effects should go through the provider.

### Template
```rust
impl DomainType {
    pub fn validate(&self) -> Result<(), Error> {
        // Validation logic
    }

    pub fn process(&self, provider: &impl Provider) -> Result<OutputType, Error> {
        // Business logic
    }
}
```

### Before (TypeScript):
```typescript
class DomainType {
    validate(): void {
        // Validation logic
    }

    process(provider: Provider): OutputType {
        // Business logic
    }
}
```

### After (Rust):
```rust
impl DomainType {
    pub fn validate(&self) -> Result<(), Error> {
        // Validation logic
    }

    pub fn process(&self, provider: &impl Provider) -> Result<OutputType, Error> {
        // Business logic
    }
}
```

---

## 5. Error Handling

### Purpose
- Define domain-specific error enums.
- Map foreign errors to semantic variants.

### Rules and Conventions
- Use descriptive error variants (e.g., `InvalidFormat`, `ProcessingError`).
- Use `thiserror` to derive `Error` and `serde` for serialization.
- Foreign errors (e.g., `std::io::Error`) should be mapped to domain-specific errors using the `#[from]` attribute in the `thiserror` crate.

### Template
```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Error {
    #[error("code: processing_error, description: {0}")]
    ProcessingError(String),

    #[error("code: invalid_format, description: {0}")]
    InvalidFormat(String),

    #[error("code: outdated, description: {0}")]
    Outdated(String),
}
```

### Before (TypeScript):
```typescript
class ProcessingError extends Error {}
class InvalidFormatError extends Error {}
class OutdatedError extends Error {}
```

### After (Rust):
```rust
#[derive(Error, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Error {
    #[error("code: processing_error, description: {0}")]
    ProcessingError(String),

    #[error("code: invalid_format, description: {0}")]
    InvalidFormat(String),

    #[error("code: outdated, description: {0}")]
    Outdated(String),
}
```

---

## 6. Migration Checklist

### Tasks for Automation
1. **Set Up Crate Structure**:
    ```plaintext
    crates/<domain-adapter>/
      Cargo.toml
      src/
        lib.rs
        handlers.rs
        error.rs
        types.rs
      tests/
        core.rs
        provider.rs
    ```
2. **Extract Business Logic**:
    - Identify TypeScript classes and methods.
    - Translate to Rust structs and methods.
3. **Define Provider Traits**:
    - Map TypeScript interfaces to Rust traits.
4. **Implement WASI Handlers**:
    - Define HTTP and messaging handlers.
5. **Write Error Enums**:
    - Map TypeScript errors to Rust enums.
6. **Validate Outputs**:
    - Compare generated Rust code to TypeScript logic.
7. **Write Integration Tests**:
    - Validate the behavior of the generated Rust code against the original TypeScript logic.
