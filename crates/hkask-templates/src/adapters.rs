//! Adapters for external services

pub mod memory_adapter;

pub use memory_adapter::{AppMemoryAdapter, MemoryAdapter, StubMemoryPort};
