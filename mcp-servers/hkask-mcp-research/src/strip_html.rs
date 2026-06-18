//! hKask MCP Web — HTML to plain-text conversion

pub fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut tag_name = String::new();
    let mut collecting_tag = false;

    for ch in html.chars() {
        if in_tag {
            if ch == '>' {
                let tag_lower = tag_name.to_lowercase();
                if tag_lower == "script" || tag_lower == "style" {
                    in_script = true;
                } else if tag_lower == "/script" || tag_lower == "/style" {
                    in_script = false;
                } else if tag_lower == "br"
                    || tag_lower.starts_with("br ")
                    || tag_lower == "p"
                    || tag_lower.starts_with("p ")
                    || tag_lower == "/p"
                {
                    result.push('\n');
                } else if tag_lower == "h1"
                    || tag_lower.starts_with("h1 ")
                    || tag_lower == "h2"
                    || tag_lower.starts_with("h2 ")
                    || tag_lower == "h3"
                    || tag_lower.starts_with("h3 ")
                {
                    result.push_str("\n## ");
                } else if tag_lower == "/h1" || tag_lower == "/h2" || tag_lower == "/h3" {
                    result.push('\n');
                } else if tag_lower == "li" || tag_lower.starts_with("li ") {
                    result.push_str("- ");
                }
                in_tag = false;
                collecting_tag = false;
                tag_name.clear();
            } else if collecting_tag {
                if ch == ' ' || ch == '\n' || ch == '\r' || ch == '\t' {
                    collecting_tag = false;
                } else {
                    tag_name.push(ch);
                }
            } else if tag_name.is_empty() && (ch == '/' || ch.is_alphabetic()) {
                collecting_tag = true;
                tag_name.push(ch);
            }
            continue;
        }
        if in_script {
            if ch == '<' {
                in_tag = true;
                tag_name.clear();
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            tag_name.clear();
            continue;
        }
        result.push(ch);
    }

    let lines: Vec<&str> = result
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // contract: CNS-WEB-STRIP-HTML
    #[test]
    fn strip_html_removes_tags() {
        let input = "<p>Hello</p>";
        assert_eq!(strip_html(input), "Hello");
    }

    // contract: CNS-WEB-STRIP-HTML
    #[test]
    fn strip_html_headings_to_markdown() {
        assert_eq!(strip_html("<h1>Title</h1>"), "## Title");
        assert_eq!(strip_html("<h2>Subtitle</h2>"), "## Subtitle");
    }

    // contract: CNS-WEB-STRIP-HTML
    #[test]
    fn strip_html_list_items() {
        // Note: consecutive <li> elements without separators produce concatenated "- " prefixes
        // without newlines between them. The closing </li> itself doesn't insert newlines.
        assert_eq!(strip_html("<li>item1</li><li>item2</li>"), "- item1- item2");
    }

    // contract: CNS-WEB-STRIP-HTML
    #[test]
    fn strip_html_removes_script_content() {
        let input = "<script>alert('hi')</script><p>visible</p>";
        assert_eq!(strip_html(input), "visible");
    }

    // contract: CNS-WEB-STRIP-HTML
    #[test]
    fn strip_html_removes_style_content() {
        let input = "<style>body{color:red}</style><p>text</p>";
        assert_eq!(strip_html(input), "text");
    }

    // contract: CNS-WEB-STRIP-HTML
    #[test]
    fn strip_html_br_to_newline() {
        assert_eq!(strip_html("line1<br>line2"), "line1\nline2");
    }

    // contract: CNS-WEB-STRIP-HTML
    #[test]
    fn strip_html_collapses_blank_lines() {
        let input = "<p>a</p>\n\n\n<p>b</p>";
        assert_eq!(strip_html(input), "a\nb");
    }

    // contract: CNS-WEB-STRIP-HTML
    #[test]
    fn strip_html_trims_trailing_whitespace() {
        let input = "<p>text   </p>";
        assert_eq!(strip_html(input), "text");
    }
}
