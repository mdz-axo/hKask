//! Input Validation Adapter

use crate::ports::security_port::{InputValidationPort, ValidationResult, ValidationError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentPersonaInput {
    pub name: String,
    pub agent_type: String,
    pub version: String,
    pub description: String,
    pub editor: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemplateInput {
    pub template_name: String,
    pub template_type: String,
    pub version: String,
    pub description: String,
}

pub struct InputValidatorAdapter;

impl InputValidatorAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InputValidatorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl InputValidationPort for InputValidatorAdapter {
    fn validate<T>(&self, input: &T) -> ValidationResult<()> {
        use std::any::Any;
        
        let any = input as &dyn Any;
        if let Some(persona) = any.downcast_ref::<AgentPersonaInput>() {
            return self.validate_persona(persona);
        }
        if let Some(template) = any.downcast_ref::<TemplateInput>() {
            return self.validate_template(template);
        }
        Ok(())
    }
}

impl InputValidatorAdapter {
    fn validate_persona(&self, input: &AgentPersonaInput) -> ValidationResult<()> {
        if input.name.is_empty() {
            return Err(ValidationError::MissingField("name".to_string()));
        }
        if input.name.len() > 64 {
            return Err(ValidationError::FieldTooLong { field: "name".to_string(), max: 64 });
        }
        if !input.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(ValidationError::InvalidFormat { field: "name".to_string() });
        }
        if !["bot", "replicant"].contains(&input.agent_type.as_str()) {
            return Err(ValidationError::InvalidFormat { field: "agent_type".to_string() });
        }
        if input.version.is_empty() || input.version.len() > 32 {
            return Err(ValidationError::InvalidFormat { field: "version".to_string() });
        }
        if input.description.len() > 1000 {
            return Err(ValidationError::FieldTooLong { field: "description".to_string(), max: 1000 });
        }
        if input.editor.is_empty() || input.editor.len() > 256 {
            return Err(ValidationError::InvalidFormat { field: "editor".to_string() });
        }
        if input.capabilities.len() > 20 {
            return Err(ValidationError::InvalidFormat { field: "capabilities".to_string() });
        }
        for cap in &input.capabilities {
            if cap.len() > 128 {
                return Err(ValidationError::FieldTooLong { field: "capability".to_string(), max: 128 });
            }
        }
        Ok(())
    }

    fn validate_template(&self, input: &TemplateInput) -> ValidationResult<()> {
        if input.template_name.is_empty() || input.template_name.len() > 128 {
            return Err(ValidationError::InvalidFormat { field: "template_name".to_string() });
        }
        if !["Prompt", "Process", "Cognition"].contains(&input.template_type.as_str()) {
            return Err(ValidationError::InvalidFormat { field: "template_type".to_string() });
        }
        if input.version.is_empty() || input.version.len() > 32 {
            return Err(ValidationError::InvalidFormat { field: "version".to_string() });
        }
        if input.description.len() > 2000 {
            return Err(ValidationError::FieldTooLong { field: "description".to_string(), max: 2000 });
        }
        Ok(())
    }
}
