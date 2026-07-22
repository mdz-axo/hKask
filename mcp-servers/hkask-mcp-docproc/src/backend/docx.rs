//! DOCX backend — uses `docx-parser` to extract markdown, then converts to
//! `DocStructure` via the shared `markdown_to_structure` parser.

use super::{BackendError, DocumentBackend, markdown_to_structure};
use hkask_types::document::DocStructure;

/// DOCX document backend.
pub struct DocxBackend;

impl DocumentBackend for DocxBackend {
    fn format(&self) -> &'static str {
        "docx"
    }

    fn parse(&self, path: &str) -> Result<DocStructure, BackendError> {
        let markdown_doc = docx_parser::MarkdownDocument::from_file(path);
        let markdown = markdown_doc.to_markdown(false);
        if markdown.trim().is_empty() {
            return Err(BackendError::Parse {
                format: "docx",
                path: path.to_string(),
                message: "DOCX contained no extractable text".to_string(),
            });
        }
        Ok(markdown_to_structure(&markdown, "docx"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_name() {
        assert_eq!(DocxBackend.format(), "docx");
    }
}
