//! EPUB 3.0 export — ZIP archive of XHTML chapters, OPF manifest, and CSS.
//!
//! EPUB is a ZIP of structured XHTML files. No external tool (Calibre) needed.
//! Public surface: 1 function (`export_epub`).

use crate::kindle_zip::types::{escape_xml, split_into_chapters, TocItem};

const EPUB_CONTAINER_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

const EPUB_CSS: &str = "body{font-family:serif;line-height:1.6;margin:2em}h1{text-align:center;margin-top:2em}p{text-indent:1.5em;margin:0.5em 0}";

/// Generate an EPUB 3.0 file as a byte vector.
///
/// Contains: mimetype, container.xml, content.opf (with manifest + spine),
/// nav.xhtml (TOC), style.css, and per-chapter XHTML files.
pub fn export_epub(text: &str, title: &str, author: &str, toc: &[TocItem]) -> Result<Vec<u8>, String> {
    use std::io::{Cursor, Write as _};

    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        // mimetype (must be first, uncompressed per EPUB spec)
        zip.start_file("mimetype", stored).map_err(|e| format!("mimetype: {}", e))?;
        zip.write_all(b"application/epub+zip").map_err(|e| format!("mimetype write: {}", e))?;

        // META-INF/container.xml
        zip.start_file("META-INF/container.xml", opts).map_err(|e| format!("container: {}", e))?;
        zip.write_all(EPUB_CONTAINER_XML.as_bytes()).map_err(|e| format!("container write: {}", e))?;

        // OEBPS/content.opf
        let opf = build_opf(title, author, toc);
        zip.start_file("OEBPS/content.opf", opts).map_err(|e| format!("opf: {}", e))?;
        zip.write_all(opf.as_bytes()).map_err(|e| format!("opf write: {}", e))?;

        // OEBPS/nav.xhtml
        let nav = build_nav(title, toc);
        zip.start_file("OEBPS/nav.xhtml", opts).map_err(|e| format!("nav: {}", e))?;
        zip.write_all(nav.as_bytes()).map_err(|e| format!("nav write: {}", e))?;

        // OEBPS/style.css
        zip.start_file("OEBPS/style.css", opts).map_err(|e| format!("css: {}", e))?;
        zip.write_all(EPUB_CSS.as_bytes()).map_err(|e| format!("css write: {}", e))?;

        // Chapter files
        let chapters = split_into_chapters(text, toc);
        for (i, (ch_title, ch_text)) in chapters.iter().enumerate() {
            let filename = format!("OEBPS/chapter-{:03}.xhtml", i + 1);
            let xhtml = build_chapter(ch_title, ch_text);
            zip.start_file(&filename, opts).map_err(|e| format!("ch{}: {}", i, e))?;
            zip.write_all(xhtml.as_bytes()).map_err(|e| format!("ch{} write: {}", i, e))?;
        }
        zip.finish().map_err(|e| format!("finish: {}", e))?;
    }
    Ok(buf.into_inner())
}

fn build_opf(title: &str, author: &str, toc: &[TocItem]) -> String {
    let mut manifest = String::new();
    let mut spine = String::new();

    manifest.push_str(r#"    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>"#);
    manifest.push('\n');
    manifest.push_str(r#"    <item id="css" href="style.css" media-type="text/css"/>"#);
    manifest.push('\n');
    spine.push_str(r#"    <itemref idref="nav" linear="no"/>"#);
    spine.push('\n');

    let chapters = split_into_chapters("", toc);
    for (i, _) in chapters.iter().enumerate() {
        let id = format!("chapter-{:03}", i + 1);
        let href = format!("chapter-{:03}.xhtml", i + 1);
        manifest.push_str(&format!(r#"    <item id="{}" href="{}" media-type="application/xhtml+xml"/>"#, id, href));
        manifest.push('\n');
        spine.push_str(&format!(r#"    <itemref idref="{}"/>"#, id));
        spine.push('\n');
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" unique-identifier="book-id" xmlns="http://www.idpf.org/2007/opf">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="book-id">urn:uuid:{}</dc:identifier>
    <dc:title>{}</dc:title>
    <dc:creator>{}</dc:creator>
    <dc:language>en</dc:language>
    <meta property="dcterms:modified">{}</meta>
  </metadata>
  <manifest>
{}
  </manifest>
  <spine>
{}
  </spine>
</package>"#,
        uuid::Uuid::new_v4(),
        escape_xml(title),
        escape_xml(author),
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        manifest,
        spine,
    )
}

fn build_nav(title: &str, toc: &[TocItem]) -> String {
    let mut items = String::new();
    let chapters = split_into_chapters("", toc);
    for (i, (ch_title, _)) in chapters.iter().enumerate() {
        items.push_str(&format!(
            r#"      <li><a href="chapter-{:03}.xhtml">{}</a></li>"#,
            i + 1,
            escape_xml(ch_title)
        ));
        items.push('\n');
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>{}</title></head>
<body>
  <nav epub:type="toc"><h1>Table of Contents</h1><ol>
{}
  </ol></nav>
</body>
</html>"#,
        escape_xml(title), items
    )
}

fn build_chapter(title: &str, content: &str) -> String {
    let paragraphs: String = content
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| format!("    <p>{}</p>", escape_xml(p.trim())))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>{0}</title><link rel="stylesheet" type="text/css" href="style.css"/></head>
<body><section epub:type="chapter"><h1>{0}</h1>
{1}
</section></body></html>"#,
        escape_xml(title), paragraphs
    )
}
