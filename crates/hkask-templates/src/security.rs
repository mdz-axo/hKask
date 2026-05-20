//! Security Adapter Stub
//!
//! TODO: Implement full security adapter with Jinja2TemplateValidator

use crate::ports::Result;
use crate::ports::TemplateError;
use hkask_cns::CnsRuntime;
use hkask_types::{CapabilityChecker, CapabilityToken, WebID};
use std::collections::HashSet;
use std::sync::Arc;

/// Security adapter for composition operations
pub struct SecurityAdapter {
    capability_checker: CapabilityChecker,
    allowed_paths: HashSet<String>,
    secret: Vec<u8>,
    cns_runtime: Option<Arc<CnsRuntime>>,
}

impl SecurityAdapter {
    /// Create new security adapter with secret key
    pub fn new(secret: &[u8]) -> Self {
        Self {
            capability_checker: CapabilityChecker::new(secret),
            allowed_paths: HashSet::new(),
            secret: secret.to_vec(),
            cns_runtime: None,
        }
    }

    /// Create security adapter with CNS runtime for span emission
    pub fn with_cns(secret: &[u8], cns_runtime: Arc<CnsRuntime>) -> Self {
        Self {
            capability_checker: CapabilityChecker::new(secret),
            allowed_paths: HashSet::new(),
            secret: secret.to_vec(),
            cns_runtime: Some(cns_runtime),
        }
    }

    /// Get the secret key (for cascade context)
    pub fn get_secret(&self) -> &[u8] {
        &self.secret
    }

    /// Allow specific path prefix
    pub fn allow_path(&mut self, path: &str) {
        self.allowed_paths.insert(path.to_string());
    }

    /// Validate path (alias for validate_template_path for SecurityPort compatibility)
    pub fn validate_path(&self, path: &str) -> Result<()> {
        self.validate_template_path(path)
    }

    /// Validate template/manifest path (prevent path traversal)
    pub fn validate_template_path(&self, path: &str) -> Result<()> {
        for pattern in ["..", "/etc/", "/proc/", "/sys/", "//"] {
            if path.contains(pattern) {
                return Err(TemplateError::PathTraversal(format!(
                    "Path traversal pattern detected: {}",
                    pattern
               )));
            }
        }
        Ok(())
    }

    /// Verify capability token signature
    pub fn verify_signature(&self, token: &CapabilityToken, holder: &WebID) -> bool {
        self.capability_checker.verify(token) && token.delegated_to == *holder
    }

    /// Check recursion depth (prevent DoS via infinite recursion)
    pub fn check_recursion_depth(&self, current_depth: u8, max_depth: u8) -> Result<()> {
        if current_depth > max_depth {
            return Err(TemplateError::RecursionLimit { max: max_depth });
        }
        Ok(())
    }

    /// Check energy budget (prevent resource exhaustion)
    pub fn check_energy_budget(&self, requested: u64, remaining: u64) -> Result<()> {
        if requested > remaining {
            return Err(TemplateError::CapabilityDenied(
                format!("Energy budget exceeded: requested {}, remaining {}", requested, remaining)
            ));
        }
        Ok(())
    }

    /// Sanitize Jinja2 input (prevent injection attacks) - stub implementation
    pub fn sanitize_jinja2_input(&self, input: &str) -> String {
        // TODO: Implement proper sanitization
        input.to_string()
    }

    /// Verify capability for template operation
    pub fn check_template_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        template_id: &str,
        current_time: i64,
    ) -> Result<()> {
        use hkask_types::CapabilityResource;

        if !self.capability_checker.verify_with_time(token, current_time) {
            return Err(TemplateError::CapabilityDenied(
                "Token expired or invalid".to_string(),
            ));
        }

        if token.delegated_to != *holder {
            return Err(TemplateError::CapabilityDenied(
                "Token not delegated to holder".to_string(),
            ));
        }

        if !token.grants_resource(CapabilityResource::Template) {
            return Err(TemplateError::CapabilityDenied(
                "Token does not grant template access".to_string(),
            ));
        }

        if token.resource_id != template_id && token.resource_id != "*" {
            return Err(TemplateError::CapabilityDenied(format!(
                "Token does not grant access to template: {}",
                template_id
            )));
        }

        Ok(())
    }

    /// Verify capability for manifest operation
    pub fn check_manifest_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        manifest_id: &str,
        current_time: i64,
    ) -> Result<()> {
        use hkask_types::CapabilityResource;

        if !self.capability_checker.verify_with_time(token, current_time) {
            return Err(TemplateError::CapabilityDenied(
                "Token expired or invalid".to_string(),
            ));
        }

        if token.delegated_to != *holder {
            return Err(TemplateError::CapabilityDenied(
                "Token not delegated to holder".to_string(),
            ));
        }

        if !token.grants_resource(CapabilityResource::Manifest) {
            return Err(TemplateError::CapabilityDenied(
                "Token does not grant manifest access".to_string(),
            ));
        }

        if token.resource_id != manifest_id && token.resource_id != "*" {
            return Err(TemplateError::CapabilityDenied(format!(
                "Token does not grant access to manifest: {}",
                manifest_id
            )));
        }

        Ok(())
    }

    /// Verify capability for cascade operation
    pub fn check_cascade_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        cascade_id: &str,
        current_time: i64,
    ) -> Result<()> {
        use hkask_types::CapabilityResource;

        if !self.capability_checker.verify_with_time(token, current_time) {
            return Err(TemplateError::CapabilityDenied(
                "Token expired or invalid".to_string(),
            ));
        }

        if token.delegated_to != *holder {
            return Err(TemplateError::CapabilityDenied(
                "Token not delegated to holder".to_string(),
            ));
        }

        if !token.grants_resource(CapabilityResource::Cascade) {
            return Err(TemplateError::CapabilityDenied(
                "Token does not grant cascade access".to_string(),
            ));
        }

        if token.resource_id != cascade_id && token.resource_id != "*" {
            return Err(TemplateError::CapabilityDenied(format!(
                "Token does not grant access to cascade: {}",
                cascade_id
            )));
        }

        Ok(())
    }

    /// Create attenuated capability for delegation
    pub fn attenuate_capability(
        &self,
        token: &CapabilityToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<CapabilityToken> {
        self.capability_checker.attenuate(token, new_to, current_time)
    }

    /// Get capability checker reference
    pub fn checker(&self) -> &CapabilityChecker {
        &self.capability_checker
    }
}

impl crate::ports::SecurityPort for SecurityAdapter {
    fn verify_signature(&self, token: &CapabilityToken, holder: &WebID) -> bool {
        self.verify_signature(token, holder)
    }

    fn check_template_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        template_id: &str,
        current_time: i64,
    ) -> crate::ports::Result<()> {
        self.check_template_capability(token, holder, template_id, current_time)
    }

    fn check_manifest_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        manifest_id: &str,
        current_time: i64,
    ) -> crate::ports::Result<()> {
        self.check_manifest_capability(token, holder, manifest_id, current_time)
    }

    fn check_cascade_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        cascade_id: &str,
        current_time: i64,
    ) -> crate::ports::Result<()> {
        self.check_cascade_capability(token, holder, cascade_id, current_time)
    }

    fn check_stage_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        stage_name: &str,
        current_time: i64,
    ) -> crate::ports::Result<()> {
        use hkask_types::CapabilityResource;

        if !self.capability_checker.verify_with_time(token, current_time) {
            return Err(crate::ports::TemplateError::CapabilityDenied(
                "Token expired or invalid".to_string(),
            ));
        }

        if token.delegated_to != *holder {
            return Err(crate::ports::TemplateError::CapabilityDenied(
                "Token not delegated to holder".to_string(),
            ));
        }

        // Stages are considered template resources
        if !token.grants_resource(CapabilityResource::Template) {
            return Err(crate::ports::TemplateError::CapabilityDenied(
                "Token does not grant template/stage access".to_string(),
            ));
        }

        // Allow wildcard or specific stage access
        if token.resource_id != format!("stage/{}", stage_name) && token.resource_id != "*" {
            return Err(crate::ports::TemplateError::CapabilityDenied(format!(
                "Token does not grant access to stage: {}",
                stage_name
            )));
        }

        Ok(())
    }

    fn attenuate_capability(
        &self,
        token: &CapabilityToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<CapabilityToken> {
        self.attenuate_capability(token, new_to, current_time)
    }

    fn validate_path(&self, path: &str) -> crate::ports::Result<()> {
        self.validate_template_path(path)
    }

    fn check_recursion_depth(&self, current_depth: u8, max_depth: u8) -> crate::ports::Result<()> {
        self.check_recursion_depth(current_depth, max_depth)
    }

    fn check_energy_budget(&self, requested: u64, remaining: u64) -> crate::ports::Result<()> {
        self.check_energy_budget(requested, remaining)
    }
}

impl Default for SecurityAdapter {
    fn default() -> Self {
        Self::new(b"default-security-key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_adapter_new() {
        let secret = b"test_secret_key_32_bytes_long_test_secret_key";
        let adapter = SecurityAdapter::new(secret);
        assert_eq!(adapter.get_secret(), secret);
    }

    #[test]
    fn test_validate_path_traversal() {
        let secret = b"test_secret_key_32_bytes_long_test_secret_key";
        let adapter = SecurityAdapter::new(secret);

        assert!(adapter.validate_path("valid/path").is_ok());
        assert!(adapter.validate_path("../etc/passwd").is_err());
        assert!(adapter.validate_path("/etc/shadow").is_err());
    }

    #[test]
    fn test_recursion_depth_check() {
        let secret = b"test_secret_key_32_bytes_long_test_secret_key";
        let adapter = SecurityAdapter::new(secret);

        assert!(adapter.check_recursion_depth(5, 7).is_ok());
        assert!(adapter.check_recursion_depth(8, 7).is_err());
    }

    #[test]
    fn test_energy_budget_check() {
        let secret = b"test_secret_key_32_bytes_long_test_secret_key";
        let adapter = SecurityAdapter::new(secret);

        assert!(adapter.check_energy_budget(100, 200).is_ok());
        assert!(adapter.check_energy_budget(300, 200).is_err());
    }
}