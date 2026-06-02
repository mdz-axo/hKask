//! Template renderer — Jinja2 rendering via minijinja
//
//! Provides template composition and rendering for the manifest executor.
//! Per architecture v0.21.0: Rust renders Jinja2, doesn't own template content.

use crate::ports::{Result, TemplateContract};

/// Parse template contract from source
pub fn parse_contract(source: &str) -> Result<TemplateContract> {
    // Look for [contract] section in template source
    let mut input_fields = vec![];
    let mut output_fields = vec![];

    if let Some(contract_start) = source.find("[contract]") {
        let contract_section = &source[contract_start..];
        if let Some(contract_end) = contract_section.find("\n---") {
            let contract_content = &contract_section[..contract_end];

            for line in contract_content.lines() {
                if line.trim().starts_with("input:") {
                    // Parse input fields from YAML-like syntax
                    input_fields = parse_fields(line);
                } else if line.trim().starts_with("output:") {
                    output_fields = parse_fields(line);
                }
            }
        }
    }

    Ok(TemplateContract {
        input_fields,
        output_fields,
    })
}

fn parse_fields(line: &str) -> Vec<String> {
    let mut fields = vec![];

    // Simple parsing: look for field names after colon
    if let Some(colon_pos) = line.find(':') {
        let field_part = &line[colon_pos + 1..].trim();

        // Handle { field1: type, field2: type } syntax
        if field_part.starts_with('{') && field_part.ends_with('}') {
            let inner = &field_part[1..field_part.len() - 1];
            for item in inner.split(',') {
                let item = item.trim();
                if let Some(colon_pos) = item.find(':') {
                    fields.push(item[..colon_pos].trim().to_string());
                } else if !item.is_empty() {
                    fields.push(item.to_string());
                }
            }
        }
    }

    fields
}
