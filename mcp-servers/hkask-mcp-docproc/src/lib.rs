//! hKask MCP DocProc — Unified document processing MCP server
//!
//! Combines format conversion, OCR, chunking, parsing, and QA generation.
//! Merges the functionality of `hkask-mcp-markitdown` and `hkask-mcp-doc-knowledge`.

pub mod convert;
pub mod ocr;
pub mod server;
pub mod tools;
