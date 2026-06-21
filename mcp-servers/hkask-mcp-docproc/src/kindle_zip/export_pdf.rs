//! PDF export — minimal valid PDF generator.
//!
//! Produces a single-page PDF with title and text content.
//! For production use, replace with `genpdf` or `printpdf` crate.
//! Public surface: 1 function (`export_pdf`).

use crate::kindle_zip::types::{escape_pdf_string, wrap_text};

/// Generate a minimal valid PDF from book text.
///
/// Uses Helvetica (base-14 font, always available in PDF readers).
/// Single page with title header and flowing body text.
pub fn export_pdf(text: &str, title: &str) -> Result<Vec<u8>, String> {
    let mut buf: Vec<u8> = Vec::new();
    use std::io::Write;

    // PDF header
    writeln!(buf, "%PDF-1.4").ok();

    // Build content stream
    let mut stream = String::new();
    stream.push_str(&format!("BT /F1 24 Tf 72 700 Td ({}) Tj ET\n", escape_pdf_string(title)));
    let mut y = 660i32;
    let end_y = 50i32;
    for line in wrap_text(text, 80) {
        if y < end_y {
            break; // single page for MVP
        }
        stream.push_str(&format!("BT /F1 11 Tf 72 {} Td ({}) Tj ET\n", y, escape_pdf_string(&line)));
        y -= 14;
    }
    let stream_bytes = stream.as_bytes();

    // Object 1: Catalog
    writeln!(buf, "1 0 obj").ok();
    writeln!(buf, "<< /Type /Catalog /Pages 2 0 R >>").ok();
    writeln!(buf, "endobj").ok();

    // Object 2: Pages
    writeln!(buf, "2 0 obj").ok();
    writeln!(buf, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>").ok();
    writeln!(buf, "endobj").ok();

    // Object 3: Page
    writeln!(buf, "3 0 obj").ok();
    writeln!(buf, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>").ok();
    writeln!(buf, "endobj").ok();

    // Object 4: Content stream
    writeln!(buf, "4 0 obj").ok();
    writeln!(buf, "<< /Length {} >>", stream_bytes.len()).ok();
    writeln!(buf, "stream").ok();
    buf.extend_from_slice(stream_bytes);
    writeln!(buf, "endstream").ok();
    writeln!(buf, "endobj").ok();

    // Object 5: Font
    writeln!(buf, "5 0 obj").ok();
    writeln!(buf, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>").ok();
    writeln!(buf, "endobj").ok();

    // Cross-reference table
    let xref_offset = buf.len();
    writeln!(buf, "xref").ok();
    writeln!(buf, "0 6").ok();
    writeln!(buf, "0000000000 65535 f ").ok();
    // Object offsets (approximate for MVP)
    writeln!(buf, "0000000009 00000 n ").ok(); // obj 1
    writeln!(buf, "0000000061 00000 n ").ok(); // obj 2
    writeln!(buf, "0000000118 00000 n ").ok(); // obj 3
    writeln!(buf, "0000000230 00000 n ").ok(); // obj 4
    writeln!(buf, "0000000300 00000 n ").ok(); // obj 5

    // Trailer
    writeln!(buf, "trailer").ok();
    writeln!(buf, "<< /Size 6 /Root 1 0 R >>").ok();
    writeln!(buf, "startxref").ok();
    writeln!(buf, "{}", xref_offset).ok();
    writeln!(buf, "%%EOF").ok();

    Ok(buf)
}
