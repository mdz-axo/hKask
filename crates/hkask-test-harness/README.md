# hkask-test-harness

Shared test fixtures and harness for hKask.

Provides reusable test infrastructure across all crates.

## Key Components

| Component | Description |
|-----------|-------------|
| `TestDb` | In-memory SQLite database with encryption |
| `TestWebId` | Factory for test WebID values |
| `TestKeystore` | In-memory keystore for testing |
| `CNS mocks` | Mock CNS runtime for unit tests |
| `TestTriple` | Test triple factory |
| `TestEvent` | Test event factory |
| `temp_dir` | Temporary directory fixture |
| `strategies` | Proptest strategies for domain types |
| `MockInferencePort` | Mock inference port for testing |
