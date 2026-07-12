# hkask-storage-core

hKask storage foundation — Database, Store trait, lock helpers, path sanitization.

## Purpose

Provides the `Database` connection manager, driver store macros, and path sanitization. This crate has no dependencies on any service or domain crate (only `hkask-types` for `InfrastructureError`).

## Public API

- `Database` — connection manager
- `open_database`, `open_or_repair`, `check_passphrase` — database lifecycle
- `sanitize_path` — path sanitization for safe filesystem operations
- `DatabaseDriverTrait` — macro for implementing driver stores
- `store_macros::define_driver_store` — generates boilerplate for driver-specific stores

## Dependencies

- `hkask-types` — `InfrastructureError`

## See also

- [`hkask-storage`](../hkask-storage/) — full storage layer (SQLite + SQLCipher)
- [`hkask-database`](../hkask-database/) — provider-agnostic database driver abstraction