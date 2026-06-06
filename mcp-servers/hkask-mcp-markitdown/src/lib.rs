//! hKask MCP Markitdown — Document format conversion and OCR MCP server
//!
//! Provides text extraction from documents (PDF, TXT, MD, HTML) with
//! automatic OCR fallback for scanned/image-based PDFs via local vision
//! models through the Okapi inference path.

pub mod convert;
pub mod tools;
