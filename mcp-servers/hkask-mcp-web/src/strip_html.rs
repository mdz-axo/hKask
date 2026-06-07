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

    // P8 invariant: plain text passes through unchanged
    #[test]
    fn strip_html_plain_text() {
        assert_eq!(strip_html("hello world"), "hello world");
    }

    // P8 invariant: HTML tags are removed
    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("Hello <b>world</b>"), "Hello world");
    }

    // P8 invariant: script content is stripped entirely (no newline added at script boundaries)
    #[test]
    fn strip_html_removes_script_content() {
        assert_eq!(
            strip_html("Text<script>alert('xss')</script>More"),
            "TextMore"
        );
    }

    // P8 invariant: style content is stripped entirely (no newline added at style boundaries)
    #[test]
    fn strip_html_removes_style_content() {
        assert_eq!(
            strip_html("Text<style>body{color:red}</style>More"),
            "TextMore"
        );
    }

    // P8 invariant: br tags produce newlines
    #[test]
    fn strip_html_br_produces_newline() {
        assert_eq!(strip_html("Line1<br>Line2"), "Line1\nLine2");
    }

    // P8 invariant: p tags produce newlines
    #[test]
    fn strip_html_p_produces_newline() {
        assert_eq!(strip_html("<p>Para1</p><p>Para2</p>"), "Para1\nPara2");
    }

    // P8 invariant: heading tags produce markdown-style prefix with exact output
    #[test]
    fn strip_html_headings_produce_markdown() {
        assert_eq!(strip_html("<h1>Title</h1>"), "## Title");
        assert_eq!(strip_html("<h2>Subtitle</h2>"), "## Subtitle");
    }

    // P8 invariant: li tags produce list markers with exact output
    #[test]
    fn strip_html_li_produces_list_marker() {
        assert_eq!(strip_html("<li>Item</li>"), "- Item");
    }

    // P8 invariant: attributes in tags don't break parsing
    #[test]
    fn strip_html_tag_with_attributes() {
        assert_eq!(
            strip_html("<a href=\"http://example.com\">link</a>"),
            "link"
        );
    }

    // P8 invariant: br with attributes still produces newline
    #[test]
    fn strip_html_br_with_attributes() {
        assert_eq!(strip_html("Line1<br />Line2"), "Line1\nLine2");
    }

    // P8 invariant: consecutive whitespace is trimmed
    #[test]
    fn strip_html_trims_blank_lines() {
        let result = strip_html("<p>A</p>  <p>B</p>");
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2, "blank lines must be trimmed");
    }

    // P8 invariant: empty input returns empty output
    #[test]
    fn strip_html_empty_input() {
        assert_eq!(strip_html(""), "");
    }
}
