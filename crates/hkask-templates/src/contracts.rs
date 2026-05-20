//! YAML-based contract parsing for templates
//!
//! Replaces string parsing with serde_yaml for [contract] and [inference] sections.

use crate::ports::{TemplateContract, TemplateError};
use hkask_types::TemplateType;
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;

/// Parsed contract section from template frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedContract {
    pub input: Option<YamlValue>,
    pub output: Option<YamlValue>,
}

/// Parsed inference section from template frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedInference {
    pub template_type: Option<String>,
    pub lexicon: Option<Vec<String>>,
    pub model_tier: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// Parse template frontmatter (sections before ---)
pub fn parse_frontmatter(source: &str) -> Result<TemplateFrontmatter, TemplateError> {
    // Split source at --- delimiter
    let parts: Vec<&str> = source.splitn(2, "\n---").collect();

    if parts.len() < 2 {
        return Err(TemplateError::Validation(
            "Template missing '---' delimiter between frontmatter and content".to_string(),
        ));
    }

    let frontmatter_str = parts[0];

    // Parse frontmatter as YAML
    let frontmatter: TemplateFrontmatterYaml = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| TemplateError::Validation(format!("Invalid YAML frontmatter: {}", e)))?;

    // Convert to strongly-typed frontmatter
    Ok(TemplateFrontmatter {
        contract: frontmatter.contract.map(|c| TemplateContract {
            input_fields: extract_field_names(&c.input),
            output_fields: extract_field_names(&c.output),
        }),
        inference: frontmatter.inference.map(|i| InferenceConfig {
            template_type: i.template_type.and_then(|t| TemplateType::parse_str(&t)),
            lexicon_terms: i.lexicon.unwrap_or_default(),
            model_tier: i.model_tier,
            timeout_ms: i.timeout_ms,
        }),
    })
}

/// Extract field names from YAML value
fn extract_field_names(value: &Option<YamlValue>) -> Vec<String> {
    match value {
        Some(YamlValue::Mapping(map)) => map
            .keys()
            .filter_map(|k| k.as_str())
            .map(String::from)
            .collect(),
        Some(YamlValue::String(s)) => {
            // Handle simple string list: "field1, field2, field3"
            s.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => vec![],
    }
}

/// Template frontmatter structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFrontmatterYaml {
    pub contract: Option<ParsedContract>,
    pub inference: Option<ParsedInference>,
}

/// Strongly-typed template frontmatter
#[derive(Debug, Clone)]
pub struct TemplateFrontmatter {
    pub contract: Option<TemplateContract>,
    pub inference: Option<InferenceConfig>,
}

/// Inference configuration from frontmatter
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    pub template_type: Option<TemplateType>,
    pub lexicon_terms: Vec<String>,
    pub model_tier: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// Validate template against hLexicon terms
pub fn validate_lexicon_terms(terms: &[String], valid_terms: &[&str]) -> Result<(), TemplateError> {
    for term in terms {
        if !valid_terms.iter().any(|&t| t == term) {
            return Err(TemplateError::Validation(format!(
                "Unknown hLexicon term: '{}'. Valid terms: {:?}",
                term, valid_terms
            )));
        }
    }
    Ok(())
}


    #[test]
    fn test_parse_frontmatter_missing_delimiter() {
        let source = r#"
contract:
  input: {}
Template content without delimiter
"#;

        let result = parse_frontmatter(source);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("missing"));
    }

    #[test]
    fn test_parse_frontmatter_invalid_yaml() {
        let source = r#"
contract:
  input: {invalid yaml
---
Content
"#;

        let result = parse_frontmatter(source);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("Invalid YAML"));
    }

    #[test]
    fn test_parse_frontmatter_minimal() {
        let source = r#"
inference:
  template_type: Process

---
Minimal template
"#;

        let frontmatter = parse_frontmatter(source).unwrap();

        assert!(frontmatter.contract.is_none());
        assert!(frontmatter.inference.is_some());
        assert_eq!(
            frontmatter.inference.unwrap().template_type,
            Some(TemplateType::Process)
        );
    }

    #[test]
    fn test_extract_field_names_mapping() {
        let yaml: YamlValue = serde_yaml::from_str(
            r#"
field1: string
field2: number
field3: object
"#,
        )
        .unwrap();

        let fields = extract_field_names(&Some(yaml));
        assert_eq!(fields.len(), 3);
        assert!(fields.contains(&"field1".to_string()));
        assert!(fields.contains(&"field2".to_string()));
        assert!(fields.contains(&"field3".to_string()));
    }

    #[test]
    fn test_extract_field_names_string() {
        let yaml: YamlValue = serde_yaml::from_str(r#""field1, field2, field3""#).unwrap();

        let fields = extract_field_names(&Some(yaml));
        assert_eq!(fields.len(), 3);
        assert!(fields.contains(&"field1".to_string()));
        assert!(fields.contains(&"field2".to_string()));
        assert!(fields.contains(&"field3".to_string()));
    }

    #[test]
    fn test_extract_field_names_empty() {
        let fields = extract_field_names(&None);
        assert!(fields.is_empty());

        let yaml: YamlValue = YamlValue::Null;
        let fields = extract_field_names(&Some(yaml));
        assert!(fields.is_empty());
    }

    #[test]
    fn test_validate_lexicon_terms_valid() {
        let terms = vec!["recognize".to_string(), "classify".to_string()];
        let valid = ["recognize", "classify", "match"];

        let result = validate_lexicon_terms(&terms, &valid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_lexicon_terms_invalid() {
        let terms = vec!["invalid_term".to_string()];
        let valid = ["recognize", "classify", "match"];

        let result = validate_lexicon_terms(&terms, &valid);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("Unknown hLexicon term"));
    }

    #[test]
    fn test_validate_lexicon_terms_empty() {
        let terms: Vec<String> = vec![];
        let valid = ["recognize", "classify"];

        let result = validate_lexicon_terms(&terms, &valid);
        assert!(result.is_ok());
    }
}
