# SmarTrak GTFS domain

This crate hosts the shared SmarTrak/GTFS domain logic that powers the WASI component in `train`.

It encapsulates:

- Fleet metadata lookups and caching helpers
- God Mode override orchestration compatible with the legacy Node.js adapter
- Trip management, serial data, and passenger count processors backed by the provider interfaces defined in `realtime`

The library is consumed by the WASM guest and unit tests via trait-based providers so that the same code can run in production and in local mocks.
