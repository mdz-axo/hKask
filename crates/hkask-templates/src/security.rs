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
use hkask_cns::CnsRuntime;
use hkask_types::{CapabilityChecker, CapabilityToken, WebID};
use percent_encoding::percent_decode_str;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

/// Jinja2 dangerous patterns to block (base patterns without whitespace variations)
const JINJA2_DANGEROUS_PATTERNS: &[&str] = &[
    "{% set %}",
    "{% import %}",
    "{% from %}",
    "{% include %}",
    "config",
    "self",
    "globals",
    "dict.__mro__",
    "''.__class__",
    "().__class__",
];

/// Path traversal patterns to block
const PATH_TRAVERSAL_PATTERNS: &[&str] = &["..", "/etc/", "/proc/", "/sys/", "//", "\\..", "/.."];

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

    /// Get the templates directory path (delegates to Registry)
    fn get_templates_path() -> std::path::PathBuf {
        use crate::registry::Registry;
        Registry::get_templates_path()
    }

    /// Validate template/manifest path (defense in depth)
    /// 
    /// **Layer 1**: Reject obvious attacks (null bytes, obvious traversal)
    /// **Layer 2**: URL decode and re-check (blocks %2e%2e attacks)
    /// **Layer 3**: Double-decode and normalize (blocks %252e%252e attacks)
    /// **Layer 4**: Pattern matching against known traversal patterns
    /// **Layer 5**: Canonical path verification (blocks symlink attacks)
    pub fn validate_template_path(&self, path: &str) -> Result<()> {
        // Layer 1: Reject obvious attacks
        if path.contains('\0') {
            return Err(TemplateError::PathTraversal("Null byte not allowed".to_string()));
        }
        if path.contains("..") {
            return Err(TemplateError::PathTraversal("Path traversal pattern '..' not allowed".to_string()));
        }
        if path.starts_with('/') || path.starts_with('\\') {
            return Err(TemplateError::PathTraversal("Absolute path not allowed".to_string()));
        }
        
        // Layer 2: URL decode and re-check
        let decoded = percent_decode_str(path)
            .decode_utf8()
            .map_err(|_| TemplateError::PathTraversal("Invalid UTF-8 in path".to_string()))?;
        
        if decoded.contains("..") || decoded.starts_with('/') {
            return Err(TemplateError::PathTraversal("Encoded path traversal detected".to_string()));
        }
        
        // Layer 3: Double-decode to catch %252e%252e attacks
        let fully_decoded = percent_decode_str(decoded.as_ref())
            .decode_utf8()
            .unwrap_or_else(|_| decoded.clone());
        
        if fully_decoded.contains("..") || fully_decoded.starts_with('/') {
            return Err(TemplateError::PathTraversal("Double-encoded path traversal detected".to_string()));
        }
        
        // Layer 4: Normalize and validate patterns
        let normalized = self.normalize_path(&fully_decoded);
        
        if normalized.starts_with('/') || normalized.starts_with('\\') {
            return Err(TemplateError::PathTraversal("Normalized absolute path not allowed".to_string()));
        }
        
        for pattern in PATH_TRAVERSAL_PATTERNS {
            if normalized.contains(pattern) {
                return Err(TemplateError::PathTraversal(format!(
                    "Path traversal pattern detected: {}",
                    pattern
                )));
            }
        }
        
        // Layer 5: Canonical path verification (blocks symlink attacks)
        // Only perform if path exists (skip for new templates)
        let base_path = Self::get_templates_path();
        let full_path = base_path.join(&normalized);
        
        if full_path.exists() {
            match full_path.canonicalize() {
                Ok(canonical) => {
                    if !canonical.starts_with(&base_path) {
                        return Err(TemplateError::PathTraversal(
                            "Canonical path escapes template directory (symlink attack?)".to_string()
                        ));
                    }
                }
                Err(_) => {
                    // canonicalize failed, but path exists - suspicious, allow but log
                    tracing::warn!(
                        target: "hkask.security",
                        "Failed to canonicalize path: {}",
                        full_path.display()
                    );
                }
            }
        }
        
        // Check against allowed paths if configured (allowlist override)
        if !self.allowed_paths.is_empty() {
            if !self
                .allowed_paths
                .iter()
                .any(|allowed| normalized.starts_with(allowed))
            {
                return Err(TemplateError::PathTraversal(format!(
                    "Path not in allowed set: {}",
                    normalized
                )));
            }
        }
        
        Ok(())
    }

    /// Normalize path by removing redundant components
    fn normalize_path(&self, path: &str) -> String {
        // Check if path is absolute before normalization
        let is_absolute = path.starts_with('/') || path.starts_with('\\');

        // Remove redundant slashes (replace // with /)
        let mut normalized = path.replace("//", "/");
        while normalized.contains("//") {
            normalized = normalized.replace("//", "/");
        }

        // Remove trailing slashes (except for root)
        if normalized.len() > 1 {
            normalized = normalized.trim_end_matches('/').to_string();
        }

        // Remove . components and empty parts
        let parts: Vec<&str> = normalized.split('/').collect();
        let mut result = Vec::new();
        for part in parts {
            if part != "." && !part.is_empty() {
                result.push(part);
            }
        }

        let normalized = result.join("/");

        // If original was absolute, preserve that information
        if is_absolute && normalized.is_empty() {
            "/".to_string()
        } else if is_absolute && !normalized.starts_with('/') {
            format!("/{}", normalized)
        } else {
            normalized
        }
    }

    /// Sanitize Jinja2 input (prevent injection attacks)
    pub fn sanitize_jinja2_input(&self, input: &str) -> String {
        let mut sanitized = input.to_string();

        // Block dangerous patterns (with flexible whitespace matching)
        for pattern in JINJA2_DANGEROUS_PATTERNS {
            // Match pattern with or without {{ }} wrapper and flexible whitespace
            let patterns_to_check = vec![
                format!("{{{{{}}}}}", pattern),     // {{config}}
                format!("{{{{ {} }}}}", pattern),   // {{ config }}
                format!("{{{{\t{}\t}}}}", pattern), // {{	tab	}}
                format!("{{{{\n{}\n}}}}", pattern), // {{\nconfig\n}}
                pattern.to_string(),                // Direct pattern match
            ];

            for p in patterns_to_check {
                if sanitized.contains(&p) {
                    sanitized = sanitized.replace(&p, "{{ BLOCKED }}");
                }
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

    /// Emit CNS span for security event
    /// 
    /// Note: CNS span emission is asynchronous (best-effort audit trail).
    /// Security decisions are NOT dependent on CNS emission success.
    /// This is by design: audit should not block authorization decisions.
    fn emit_security_span(&self, event_type: &str, outcome: &str, _details: serde_json::Value) {
        if let Some(ref cns) = self.cns_runtime {
            // Use variety tracking for security events
            let domain = match event_type {
                "template_capability_check" => "security.template",
                "manifest_capability_check" => "security.manifest",
                "cascade_capability_check" => "security.cascade",
                "path_validation" => "security.path",
                _ => "security",
            };
            let state = format!("{}_{}", event_type, outcome);
            // Spawn async task - audit is best-effort, never blocks security
            tokio::spawn({
                let cns = Arc::clone(cns);
                async move {
                    cns.increment_variety(domain, &state).await;
                }
            });
        }
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

        let result = (|| -> Result<()> {
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
        })();

        // Emit CNS span for security audit
        self.emit_security_span(
            "template_capability_check",
            if result.is_ok() { "allowed" } else { "denied" },
            json!({
                "template_id": template_id,
                "holder": holder.to_string(),
                "token_id": token.id,
                "context_nonce": token.context_nonce,
            }),
        );

        result
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

        if !self
            .capability_checker
            .verify_with_time(token, current_time)
        {
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
    fn test_validate_template_path_url_encoded() {
        let adapter = SecurityAdapter::new(b"test-secret");
        // URL-encoded path traversal should be blocked
        assert!(adapter.validate_template_path("%2e%2e/etc/passwd").is_err());
        assert!(adapter.validate_template_path("..%2fetc%2fpasswd").is_err());
        // Double-encoded should also be blocked
        assert!(
            adapter
                .validate_template_path("%252e%252e/etc/passwd")
                .is_err()
        );
    }

    #[test]
    fn test_validate_template_path_normalized() {
        let adapter = SecurityAdapter::new(b"test-secret");
        // Redundant slashes should be normalized
        assert!(adapter.validate_template_path("prompt//test").is_ok());
        // Dot components should be removed
        assert!(adapter.validate_template_path("prompt/./test").is_ok());
        // Trailing slashes should be removed
        assert!(adapter.validate_template_path("prompt/test/").is_ok());
    }

    #[test]
    fn test_normalize_path_redundant_slashes() {
        let adapter = SecurityAdapter::new(b"test-secret");
        assert_eq!(adapter.normalize_path("a//b"), "a/b");
        assert_eq!(adapter.normalize_path("a///b"), "a/b");
        assert_eq!(adapter.normalize_path("a/b//c//d"), "a/b/c/d");
    }

    #[test]
    fn test_normalize_path_dot_components() {
        let adapter = SecurityAdapter::new(b"test-secret");
        assert_eq!(adapter.normalize_path("a/./b"), "a/b");
        assert_eq!(adapter.normalize_path("./a/b"), "a/b");
        assert_eq!(adapter.normalize_path("a/b/."), "a/b");
    }

    #[test]
    fn test_normalize_path_trailing_slashes() {
        let adapter = SecurityAdapter::new(b"test-secret");
        assert_eq!(adapter.normalize_path("a/b/"), "a/b");
        assert_eq!(adapter.normalize_path("a/b//"), "a/b");
        assert_eq!(adapter.normalize_path("/"), "/");
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

    #[cfg(test)]
    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_validate_path_no_traversal(path in "[a-zA-Z0-9/_-]{1,50}") {
                let adapter = SecurityAdapter::new(b"test-secret");
                // Valid paths without traversal should always pass
                if !path.contains("..") && !path.starts_with('/') && !path.contains("/etc") {
                    prop_assert!(adapter.validate_template_path(&path).is_ok());
                }
            }

            #[test]
            fn test_validate_path_blocks_traversal(path in ".*\\..*") {
                let adapter = SecurityAdapter::new(b"test-secret");
                // Paths with .. should be blocked
                if path.contains("..") {
                    prop_assert!(adapter.validate_template_path(&path).is_err());
                }
            }

            #[test]
            fn test_validate_path_blocks_absolute(path in "/.*") {
                let adapter = SecurityAdapter::new(b"test-secret");
                // Absolute paths should be blocked
                prop_assert!(adapter.validate_template_path(&path).is_err());
            }

            #[test]
            fn test_normalize_path_preserves_valid_segments(
                base in "[a-zA-Z][a-zA-Z0-9]{0,20}",
                mid in "[a-zA-Z][a-zA-Z0-9]{0,20}",
                end in "[a-zA-Z][a-zA-Z0-9]{0,20}"
            ) {
                let adapter = SecurityAdapter::new(b"test-secret");
                let path = format!("{}/{}/{}", base, mid, end);
                let normalized = adapter.normalize_path(&path);
                // Valid paths should be unchanged
                prop_assert_eq!(normalized, path);
            }

            #[test]
            fn test_normalize_path_removes_redundant_slashes(
                a in "[a-z]+",
                b in "[a-z]+",
                slashes in 2..=5usize
            ) {
                let adapter = SecurityAdapter::new(b"test-secret");
                let slashes_str = "/".repeat(slashes);
                let path = format!("{}{}{}", a, slashes_str, b);
                let normalized = adapter.normalize_path(&path);
                // Should normalize to single slash
                let expected = format!("{}/{}", a, b);
                prop_assert!(normalized.contains(&expected));
                prop_assert!(!normalized.contains(&slashes_str));
            }

            #[test]
            fn test_sanitize_jinja2_blocks_dangerous_patterns(input in ".*\\{\\{.*config.*\\}\\}.*") {
                let adapter = SecurityAdapter::new(b"test-secret");
                let sanitized = adapter.sanitize_jinja2_input(&input);
                // Should block config patterns (with or without spaces)
                let has_config = sanitized.contains("{{ config }}") || sanitized.contains("{{config}}");
                prop_assert!(!has_config);
            }

            #[test]
            fn test_check_recursion_depth_property(
                current in 0u8..=10,
                max in 0u8..=10
            ) {
                let adapter = SecurityAdapter::new(b"test-secret");
                let result = adapter.check_recursion_depth(current, max);
                // Should fail only if current > max
                if current > max {
                    prop_assert!(result.is_err());
                } else {
                    prop_assert!(result.is_ok());
                }
            }

            #[test]
            fn test_check_energy_budget_property(
                requested in 0u64..=2000,
                remaining in 0u64..=2000
            ) {
                let adapter = SecurityAdapter::new(b"test-secret");
                let result = adapter.check_energy_budget(requested, remaining);
                // Should fail only if requested > remaining
                if requested > remaining {
                    prop_assert!(result.is_err());
                } else {
                    prop_assert!(result.is_ok());
                }
            }

            #[test]
            fn test_security_composition(
                path in "[a-zA-Z0-9/_-]{1,50}",
                depth in 0u8..=10,
                max_depth in 0u8..=10,
                energy_req in 0u64..=2000,
                energy_rem in 0u64..=2000
            ) {
                let adapter = SecurityAdapter::new(b"test-secret");
                
                // All security checks should work independently
                let path_ok = adapter.validate_template_path(&path).is_ok() 
                    || path.contains("..") || path.starts_with('/');
                let depth_ok = adapter.check_recursion_depth(depth, max_depth).is_ok() 
                    == (depth <= max_depth);
                let energy_ok = adapter.check_energy_budget(energy_req, energy_rem).is_ok()
                    == (energy_req <= energy_rem);
                
                // Composition: all checks should be consistent
                prop_assert!(path_ok);
                prop_assert!(depth_ok);
                prop_assert!(energy_ok);
            }
        }
    }
}
