//! PPTX backend — uses `pptx-to-md` to extract slide text as markdown, then
//! converts to `DocStructure` via the shared `markdown_to_structure` parser.

use super::{BackendError, DocumentBackend, markdown_to_structure};
use hkask_types::document::DocStructure;
use pptx_to_md::{ImageHandlingMode, ParserConfig, PresentationContainer};

/// PPTX/ODP presentation backend.
pub struct PptxBackend;

impl DocumentBackend for PptxBackend {
    fn format(&self) -> &'static str {
        "pptx"
    }

    fn parse(&self, path: &str) -> Result<DocStructure, BackendError> {
        let config = ParserConfig::builder()
            .image_handling_mode(ImageHandlingMode::Skip)
            .build();
        let mut container = PresentationContainer::open(std::path::Path::new(path), config)
            .map_err(|e| BackendError::Parse {
                format: "pptx",
                path: path.to_string(),
                message: e.to_string(),
            })?;
        let markdown = container.convert_to_md().map_err(|e| BackendError::Parse {
            format: "pptx",
            path: path.to_string(),
            message: e.to_string(),
        })?;
        if markdown.trim().is_empty() {
            return Err(BackendError::Parse {
                format: "pptx",
                path: path.to_string(),
                message: "Presentation contained no extractable text".to_string(),
            });
        }
        Ok(markdown_to_structure(&markdown, "pptx"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_name() {
        assert_eq!(PptxBackend.format(), "pptx");
    }
}
