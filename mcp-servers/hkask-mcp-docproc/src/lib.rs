//! hKask MCP DocProc — Unified document processing MCP server
//!
//! Combines format conversion, OCR, chunking, triple extraction, embedding,
//! QA generation, and caching. Supersedes the former `hkask-mcp-markitdown`
//! and `hkask-mcp-doc-knowledge` servers.

pub mod convert;
pub mod ocr;
pub mod server;
pub mod tools;
