//! Security Adapter for Pragmatic Composition
//!
//! Implements runtime security checks per Bruce Schneier threat model.
//! All inbound operations must pass security validation.
//!
//! **Security Checks:**
//! - Path traversal prevention (template/manifest paths)
//! - Jinja2 injection sanitization (variable input)
//! - Signature verification (capability tokens)
//! - Recursion depth limits (DoS prevention)
//! - Energy budget enforcement (resource exhaustion)
//!
//! **Threat Model (STRIDE):**
//! - **S**poofing → Capability token forgery
//! - **T**ampering → Template/manifest modification
//! - **R**epudiation → Missing audit trail
//! - **I**nformation disclosure → Path traversal
//! - **D**enial of service → Recursion exhaustion
//! - **E**levation of privilege → Capability attenuation bypass

use crate::ports::Result;
use crate::ports::TemplateError;
use hkask_types::{CapabilityChecker, CapabilityToken, WebID};
use std::collections::HashSet;

/// Jinja2 dangerous patterns to block
const JINJA2_DANGEROUS_PATTERNS: &[&str] = &[
    "{% set %}",
    "{% import %}",
    "{% from %}",
    "{% include %}",
    "{{ config }}",
    "{{ self }}",
    "{{ globals }}",
    "{{ dict.__mro__ }}",
    "{{ ''.__class__ }}",
    "{{ ().__class__ }}",
];

/// Path traversal patterns to block
const PATH_TRAVERSAL_PATTERNS: &[&str] = &["..", "/etc/", "/proc/", "/sys/", "//", "\\..", "/.."];

/// Maximum recursion depth (Miller's law: 7 ± 2)
const MAX_RECURSION_DEPTH: u8 = 7;

/// Security adapter for composition operations
pub struct SecurityAdapter {
    capability_checker: CapabilityChecker,
    allowed_paths: HashSet<String>,
}

impl SecurityAdapter {
    /// Create new security adapter with secret key
    pub fn new(secret: &[u8]) -> Self {
        Self {
            capability_checker: CapabilityChecker::new(secret),
            allowed_paths: HashSet::new(),
        }
    }

    /// Allow specific path prefix
    pub fn allow_path(&mut self, path: &str) {
        self.allowed_paths.insert(path.to_string());
    }

    /// Validate template/manifest path (prevent path traversal)
    pub fn validate_template_path(&self, path: &str) -> Result<()> {
        // Reject absolute paths
        if path.starts_with('/') || path.starts_with('\\') {
            return Err(TemplateError::PathTraversal(format!(
                "Absolute path not allowed: {}",
                path
            )));
        }

        // Reject path traversal patterns
        for pattern in PATH_TRAVERSAL_PATTERNS {
            if path.contains(pattern) {
                return Err(TemplateError::PathTraversal(format!(
                    "Path traversal pattern detected: {}",
                    pattern
                )));
            }
        }

        // Reject null bytes
        if path.contains('\0') {
            return Err(TemplateError::PathTraversal(
                "Null byte not allowed".to_string(),
            ));
        }

        // Check against allowed paths if configured
        if !self.allowed_paths.is_empty() {
            let normalized = path.trim_matches(|c| c == '/' || c == '\\');
            if !self
                .allowed_paths
                .iter()
                .any(|allowed| normalized.starts_with(allowed))
            {
                return Err(TemplateError::PathTraversal(format!(
                    "Path not in allowed set: {}",
                    path
                )));
            }
        }

        Ok(())
    }

    /// Sanitize Jinja2 input (prevent injection attacks)
    pub fn sanitize_jinja2_input(&self, input: &str) -> String {
        let mut sanitized = input.to_string();

        // Block dangerous patterns
        for pattern in JINJA2_DANGEROUS_PATTERNS {
            if sanitized.contains(pattern) {
                // Replace with safe placeholder
                sanitized = sanitized.replace(pattern, "{{ BLOCKED }}");
            }
        }

        sanitized
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
            return Err(TemplateError::Manifest(format!(
                "Energy budget exceeded: requested {}, remaining {}",
                requested, remaining
            )));
        }
        Ok(())
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

        if !self
            .capability_checker
            .verify_with_time(token, current_time)
        {
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

        if !self
            .capability_checker
            .verify_with_time(token, current_time)
        {
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

        if !self
            .capability_checker
            .verify_with_time(token, current_time)
        {
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
        self.capability_checker
            .attenuate(token, new_to, current_time)
    }

    /// Get capability checker reference
    pub fn checker(&self) -> &CapabilityChecker {
        &self.capability_checker
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
    use hkask_types::CapabilityAction;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn current_time() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[test]
    fn test_validate_template_path_ok() {
        let adapter = SecurityAdapter::new(b"test-secret");
        assert!(adapter.validate_template_path("prompt/test").is_ok());
        assert!(
            adapter
                .validate_template_path("process/memory/recall")
                .is_ok()
        );
    }

    #[test]
    fn test_validate_template_path_absolute() {
        let adapter = SecurityAdapter::new(b"test-secret");
        assert!(adapter.validate_template_path("/etc/passwd").is_err());
        assert!(adapter.validate_template_path("/tmp/test").is_err());
    }

    #[test]
    fn test_validate_template_path_traversal() {
        let adapter = SecurityAdapter::new(b"test-secret");
        assert!(adapter.validate_template_path("../etc/passwd").is_err());
        assert!(adapter.validate_template_path("test/../../../etc").is_err());
    }

    #[test]
    fn test_sanitize_jinja2_input() {
        let adapter = SecurityAdapter::new(b"test-secret");
        // Test with {{ config }} which is in the dangerous patterns list
        let input = "Hello {{ name }} {{ config }}";
        let sanitized = adapter.sanitize_jinja2_input(input);
        assert!(sanitized.contains("BLOCKED"));
        assert!(!sanitized.contains("{{ config }}"));
    }

    #[test]
    fn test_sanitize_jinja2_safe() {
        let adapter = SecurityAdapter::new(b"test-secret");
        let input = "Hello {{ name }}";
        let sanitized = adapter.sanitize_jinja2_input(input);
        assert_eq!(sanitized, input);
    }

    #[test]
    fn test_verify_signature() {
        let adapter = SecurityAdapter::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();
        let token = adapter.checker().grant_tool("test".to_string(), from, to);

        assert!(adapter.verify_signature(&token, &to));
        assert!(!adapter.verify_signature(&token, &from));
    }

    #[test]
    fn test_check_recursion_depth() {
        let adapter = SecurityAdapter::new(b"test-secret");
        assert!(adapter.check_recursion_depth(0, 7).is_ok());
        assert!(adapter.check_recursion_depth(6, 7).is_ok());
        assert!(adapter.check_recursion_depth(7, 7).is_ok());
        assert!(adapter.check_recursion_depth(8, 7).is_err());
    }

    #[test]
    fn test_check_energy_budget() {
        let adapter = SecurityAdapter::new(b"test-secret");
        assert!(adapter.check_energy_budget(100, 1000).is_ok());
        assert!(adapter.check_energy_budget(1000, 1000).is_ok());
        assert!(adapter.check_energy_budget(1001, 1000).is_err());
    }

    #[test]
    fn test_check_template_capability() {
        let adapter = SecurityAdapter::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();
        let token = adapter.checker().grant_template(
            "prompt/test".to_string(),
            CapabilityAction::Render,
            from,
            to,
        );

        assert!(
            adapter
                .check_template_capability(&token, &to, "prompt/test", current_time())
                .is_ok()
        );
        assert!(
            adapter
                .check_template_capability(&token, &from, "prompt/test", current_time())
                .is_err()
        );
    }

    #[test]
    fn test_attenuate_capability() {
        let adapter = SecurityAdapter::new(b"test-secret");
        let from = WebID::new();
        let to = WebID::new();
        let new_to = WebID::new();

        let token = adapter.checker().grant_template(
            "prompt/test".to_string(),
            CapabilityAction::Render,
            from,
            to,
        );
        assert!(token.can_attenuate());

        let attenuated = adapter.attenuate_capability(&token, new_to, current_time());
        assert!(attenuated.is_some());
        assert_eq!(attenuated.unwrap().attenuation_level, 1);
    }
}
