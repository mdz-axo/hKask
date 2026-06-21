//! PDF export — minimal valid PDF generator with correct xref table.
//!
//! Produces a single-page PDF with title and text content.
//! For production use, replace with `genpdf` or `printpdf` crate.

use std::io::Write;

use crate::kindle_zip::types::{escape_pdf_string, wrap_text};

/// Generate a minimal valid PDF from book text.
///
/// Uses Helvetica (base-14 font, always available in PDF readers).
/// Single page — multi-page support requires `genpdf`/`printpdf`.
pub fn export_pdf(text: &str, title: &str) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = Vec::new();

    // Build content stream
    let mut stream = String::new();
    stream.push_str(&format!(
        "BT /F1 24 Tf 72 700 Td ({}) Tj ET\n",
        escape_pdf_string(title)
    ));
    let mut y = 660i32;
    for line in wrap_text(text, 80) {
        if y < 50 {
            break;
        }
        stream.push_str(&format!(
            "BT /F1 11 Tf 72 {} Td ({}) Tj ET\n",
            y,
            escape_pdf_string(&line)
        ));
        y -= 14;
    }
    let stream_bytes = stream.as_bytes();

    // Write objects with tracked offsets
    writeln!(buf, "%PDF-1.4").ok();

    let obj1_off = buf.len();
    writeln!(buf, "1 0 obj").ok();
    writeln!(buf, "<< /Type /Catalog /Pages 2 0 R >>").ok();
    writeln!(buf, "endobj").ok();

    let obj2_off = buf.len();
    writeln!(buf, "2 0 obj").ok();
    writeln!(buf, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>").ok();
    writeln!(buf, "endobj").ok();

    let obj3_off = buf.len();
    writeln!(buf, "3 0 obj").ok();
    writeln!(
        buf,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
    )
    .ok();
    writeln!(buf, "endobj").ok();

    let obj4_off = buf.len();
    writeln!(buf, "4 0 obj").ok();
    writeln!(buf, "<< /Length {} >>", stream_bytes.len()).ok();
    writeln!(buf, "stream").ok();
    buf.extend_from_slice(stream_bytes);
    writeln!(buf, "\nendstream").ok();
    writeln!(buf, "endobj").ok();

    let obj5_off = buf.len();
    writeln!(buf, "5 0 obj").ok();
    writeln!(
        buf,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"
    )
    .ok();
    writeln!(buf, "endobj").ok();

    // Cross-reference table with real offsets
    let xref_off = buf.len();
    writeln!(buf, "xref").ok();
    writeln!(buf, "0 6").ok();
    writeln!(buf, "0000000000 65535 f ").ok();
    writeln!(buf, "{:010} 00000 n ", obj1_off).ok();
    writeln!(buf, "{:010} 00000 n ", obj2_off).ok();
    writeln!(buf, "{:010} 00000 n ", obj3_off).ok();
    writeln!(buf, "{:010} 00000 n ", obj4_off).ok();
    writeln!(buf, "{:010} 00000 n ", obj5_off).ok();

    writeln!(buf, "trailer").ok();
    writeln!(buf, "<< /Size 6 /Root 1 0 R >>").ok();
    writeln!(buf, "startxref").ok();
    writeln!(buf, "{}", xref_off).ok();
    writeln!(buf, "%%EOF").ok();

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_valid_pdf_header() {
        let pdf = export_pdf("Hello world.", "Test").unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn contains_stream_data() {
        let pdf = export_pdf("Hello world.", "Test Title").unwrap();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("Test Title"));
        assert!(text.contains("Hello world"));
        assert!(text.contains("/Type /Catalog"));
        assert!(text.contains("/BaseFont /Helvetica"));
    }

    #[test]
    fn xref_table_present() {
        let pdf = export_pdf("A", "B").unwrap();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("xref"));
        assert!(text.contains("trailer"));
        assert!(text.contains("startxref"));
    }
}
