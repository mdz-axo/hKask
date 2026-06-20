//! PDF assembly — converts captured page PNGs into a single PDF.
//!
//! Minimal hand-rolled PDF writer using the `image` crate for PNG→JPEG re-encoding.
//! Produces valid PDF-1.4 with one DCT-encoded JPEG image per page.
//! Zero external dependencies beyond workspace crates `image` and `base64`.

/// Assembles a vector of PNG byte buffers into a single PDF, one image per page.
/// Re-encodes PNGs as JPEG for smaller file size with DCTDecode.
pub fn assemble(pages: &[Vec<u8>], output_path: &str) -> Result<u64, String> {
    use std::io::Write;

    if pages.is_empty() {
        return Err("No pages to assemble".into());
    }

    struct PageInfo {
        width: u32,
        height: u32,
        data: Vec<u8>,
    }

    let mut infos: Vec<PageInfo> = Vec::with_capacity(pages.len());
    for (i, png) in pages.iter().enumerate() {
        let img = image::load_from_memory(png)
            .map_err(|e| format!("Failed to decode page {i} PNG: {e}"))?;
        let (w, h) = (img.width(), img.height());
        let mut jpg: Vec<u8> = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut jpg),
            image::ImageFormat::Jpeg,
        )
        .map_err(|e| format!("Failed to re-encode page {i} as JPEG: {e}"))?;
        infos.push(PageInfo {
            width: w,
            height: h,
            data: jpg,
        });
    }

    let mut pdf: Vec<u8> = Vec::new();
    let mut offsets: Vec<u64> = Vec::new();

    writeln!(pdf, "%PDF-1.4").unwrap();
    pdf.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n");

    let n = infos.len() as u32;

    // Object 1: Catalog
    offsets.push(pdf.len() as u64);
    writeln!(pdf, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj").unwrap();

    // Object 2: Pages
    offsets.push(pdf.len() as u64);
    write!(pdf, "2 0 obj\n<< /Type /Pages /Kids [").unwrap();
    for i in 0..n {
        write!(pdf, "{} 0 R ", 3 + i * 3).unwrap();
    }
    writeln!(pdf, "] /Count {} >>\nendobj", n).unwrap();

    // Per-page: Page, Image XObject, Content stream
    for (i, info) in infos.iter().enumerate() {
        let page_obj = 3 + i as u32 * 3;
        let img_obj = page_obj + 1;
        let content_obj = page_obj + 2;

        // Image XObject (DCT-encoded JPEG)
        offsets.push(pdf.len() as u64);
        writeln!(
            pdf,
            "{} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream",
            img_obj, info.width, info.height, info.data.len()
        )
        .unwrap();
        pdf.write_all(&info.data).unwrap();
        writeln!(pdf, "\nendstream\nendobj").unwrap();

        // Content stream (scale image to fill page)
        offsets.push(pdf.len() as u64);
        let content = format!("q\n{} 0 0 {} 0 0 cm\n/Im0 Do\nQ", info.width, info.height);
        writeln!(
            pdf,
            "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj",
            content_obj,
            content.len(),
            content
        )
        .unwrap();

        // Page object
        offsets.push(pdf.len() as u64);
        writeln!(
            pdf,
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R /Resources << /XObject << /Im0 {} 0 R >> >> >>\nendobj",
            page_obj, info.width, info.height, content_obj, img_obj
        )
        .unwrap();
    }

    // Cross-reference table
    let xref_offset = pdf.len() as u64;
    writeln!(pdf, "xref\n0 {}", offsets.len() as u32 + 1).unwrap();
    writeln!(pdf, "0000000000 65535 f ").unwrap();
    for off in &offsets {
        writeln!(pdf, "{:010} 00000 n ", off).unwrap();
    }

    // Trailer
    writeln!(
        pdf,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF",
        offsets.len() as u32 + 1
    )
    .unwrap();

    let size = pdf.len() as u64;
    std::fs::write(output_path, &pdf)
        .map_err(|e| format!("Failed to write PDF to '{}': {e}", output_path))?;

    Ok(size)
}
