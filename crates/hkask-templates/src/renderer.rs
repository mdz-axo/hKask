//! Template renderer — Jinja2 rendering via minijinja
//!
//! Implements the TemplateRenderer port for template composition.
//! Per architecture v0.21.0: Rust renders Jinja2, doesn't own template content.

use crate::ports::{
    CompositionTemplate, Result, TemplateContract, TemplateError, TemplateRenderer,
};
use hkask_types::TemplateType;
use minijinja::Environment;
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Jinja2 template renderer using minijinja
pub struct TemplateRendererImpl {
    env: Arc<RwLock<Environment<'static>>>,
}

impl TemplateRendererImpl {
    pub fn new() -> Self {
        let mut env = Environment::new();

        // Configure minijinja for hKask templates
        env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);

        Self {
            env: Arc::new(RwLock::new(env)),
        }
    }

    /// Add a template to the environment
    pub fn add_template(&self, name: &str, source: &str) -> Result<()> {
        let mut env = self
            .env
            .write()
            .map_err(|_| TemplateError::Render("Failed to acquire environment lock".to_string()))?;
        env.add_template_owned(name.to_string(), source.to_string())
            .map_err(|e| TemplateError::Render(e.to_string()))?;
        Ok(())
    }

    /// Render a template by name with given bindings
    pub fn render_by_name(&self, name: &str, bindings: &Value) -> Result<String> {
        let env = self
            .env
            .read()
            .map_err(|_| TemplateError::Render("Failed to acquire environment lock".to_string()))?;
        let template = env
            .get_template(name)
            .map_err(|e| TemplateError::NotFound(e.to_string()))?;

        let rendered = template
            .render(bindings)
            .map_err(|e| TemplateError::Render(e.to_string()))?;

        Ok(rendered)
    }
}

impl Default for TemplateRendererImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRenderer for TemplateRendererImpl {
    fn load(&self, path: &Path) -> Result<CompositionTemplate> {
        // Load template from filesystem
        let source = std::fs::read_to_string(path)
            .map_err(|e| TemplateError::NotFound(format!("Failed to load {:?}: {}", path, e)))?;

        // Parse contract from source
        let contract = parse_contract(&source)?;

        // Extract template ID from path (e.g., "registry/templates/foo.j2" -> "foo")
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| TemplateError::NotFound(format!("Invalid path: {:?}", path)))?
            .to_string();

        // Determine template type from source (look for template_type: directive)
        let template_type = extract_template_type(&source).unwrap_or(TemplateType::Prompt);

        // Extract lexicon terms from source
        let lexicon_terms = extract_lexicon_terms(&source);

        Ok(CompositionTemplate {
            id,
            template_type,
            lexicon_terms,
            source,
            contract,
        })
    }

    fn render(&self, template: &CompositionTemplate, bindings: Value) -> Result<String> {
        // Check if template exists, if not add it
        let needs_add = {
            let env = self.env.read().map_err(|_| {
                TemplateError::Render("Failed to acquire environment lock".to_string())
            })?;
            env.get_template(&template.id).is_err()
        };

        if needs_add {
            let mut env = self.env.write().map_err(|_| {
                TemplateError::Render("Failed to acquire environment lock".to_string())
            })?;
            env.add_template_owned(template.id.clone(), template.source.clone())
                .map_err(|e| TemplateError::Render(e.to_string()))?;
        }

        // Render with bindings
        self.render_by_name(&template.id, &bindings)
    }
}

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

/// Extract lexicon terms from template source
fn extract_lexicon_terms(source: &str) -> Vec<String> {
    let mut terms = vec![];

    // Look for lexicon: directive in template source
    for line in source.lines() {
        if line.trim().starts_with("lexicon:") {
            let lexicon_part = line.trim().strip_prefix("lexicon:").unwrap_or("").trim();
            // Parse comma-separated terms
            for term in lexicon_part.split(',') {
                let term = term.trim().trim_matches(|c| c == '"' || c == '\'');
                if !term.is_empty() {
                    terms.push(term.to_string());
                }
            }
        }
    }

    terms
}

/// Extract template type from template source
fn extract_template_type(source: &str) -> Option<TemplateType> {
    // Look for template_type: directive in [inference] section
    for line in source.lines() {
        if line.trim().starts_with("template_type:") {
            let type_str = line
                .trim()
                .strip_prefix("template_type:")
                .unwrap_or("")
                .trim();
            return TemplateType::parse_str(type_str);
        }
    }
    None
}

/// Validate template against hLexicon terms
pub fn validate_lexicon(template: &CompositionTemplate, valid_terms: &[&str]) -> Result<()> {
    for term in &template.lexicon_terms {
        if !valid_terms.iter().any(|&t| t == term) {
            return Err(TemplateError::Validation(format!(
                "Unknown hLexicon term: {}",
                term
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_new() {
        let renderer = TemplateRendererImpl::new();
        // Just verify it constructs without error
        drop(renderer);
    }

    #[test]
    fn test_renderer_add_and_render() {
        let renderer = TemplateRendererImpl::new();

        renderer
            .add_template("test", "Hello, {{ name }}! You are {{ age }} years old.")
            .unwrap();

        let mut bindings = serde_json::Map::new();
        bindings.insert("name".to_string(), Value::String("Alice".to_string()));
        bindings.insert("age".to_string(), Value::Number(30.into()));

        let result = renderer
            .render_by_name("test", &Value::Object(bindings))
            .unwrap();

        assert!(result.contains("Hello, Alice!"));
        assert!(result.contains("30 years old"));
    }

    #[test]
    fn test_parse_contract() {
        let source = r#"
[inference]
template_type: Prompt

[contract]
input: {raw_prompt: string, context: object}
output: {result: string, confidence: float}

---
Template content here
"#;

        let contract = parse_contract(source).unwrap();

        assert!(contract.input_fields.contains(&"raw_prompt".to_string()));
        assert!(contract.input_fields.contains(&"context".to_string()));
        assert!(contract.output_fields.contains(&"result".to_string()));
        assert!(contract.output_fields.contains(&"confidence".to_string()));
    }

    #[test]
    fn test_validate_lexicon() {
        let template = CompositionTemplate {
            id: "test".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec!["recognize".to_string(), "classify".to_string()],
            source: "test".to_string(),
            contract: TemplateContract {
                input_fields: vec![],
                output_fields: vec![],
            },
        };

        let valid_terms = ["recognize", "classify", "match"];
        assert!(validate_lexicon(&template, &valid_terms).is_ok());

        let invalid_template = CompositionTemplate {
            lexicon_terms: vec!["invalid_term".to_string()],
            ..template
        };
        assert!(validate_lexicon(&invalid_template, &valid_terms).is_err());
    }

    #[test]
    fn test_parse_fields() {
        let line = "input: {field1: string, field2: number, field3: object}";
        let fields = parse_fields(line);

        assert_eq!(fields.len(), 3);
        assert!(fields.contains(&"field1".to_string()));
        assert!(fields.contains(&"field2".to_string()));
        assert!(fields.contains(&"field3".to_string()));
    }

    #[test]
    fn test_extract_lexicon_terms() {
        let source = r#"
[inference]
template_type: Prompt
lexicon: recognize, classify, match

---
Content here
"#;

        let terms = extract_lexicon_terms(source);
        assert_eq!(terms.len(), 3);
        assert!(terms.contains(&"recognize".to_string()));
        assert!(terms.contains(&"classify".to_string()));
        assert!(terms.contains(&"match".to_string()));
    }

    #[test]
    fn test_extract_template_type() {
        let source = r#"
[inference]
template_type: Cognition
lexicon: detect, calibrate

---
Content here
"#;

        let template_type = extract_template_type(source);
        assert_eq!(template_type, Some(TemplateType::Cognition));

        let no_type_source = "---\nNo template_type here";
        assert_eq!(extract_template_type(no_type_source), None);
    }

    #[test]
    fn test_load_template_from_filesystem() {
        let renderer = TemplateRendererImpl::new();
        let path = Path::new("registry/templates/prompt_render.j2");

        // This test will only pass if the template file exists
        let result = renderer.load(path);

        // Check if file exists before asserting
        if path.exists() {
            let template = result.unwrap();
            assert_eq!(template.id, "prompt_render");
            assert!(!template.source.is_empty());
            assert!(template.lexicon_terms.contains(&"render".to_string()));
        }
    }
}
