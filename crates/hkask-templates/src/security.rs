use crate::ports::Result;
use crate::ports::TemplateError;
use hkask_cns::CnsRuntime;
use hkask_types::{CapabilityChecker, CapabilityToken, WebID};
use minijinja::{Environment, UndefinedBehavior};
use percent_encoding::percent_decode_str;
use regex_lite::Regex;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

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

const PATH_TRAVERSAL_PATTERNS: &[&str] = &["..", "/etc/", "/proc/", "/sys/", "//", "\\..", "/.."];

const ALLOWED_FILTERS: &[&str] = &[
    "abs",
    "attr",
    "batch",
    "capitalize",
    "center",
    "count",
    "default",
    "dictsort",
    "escape",
    "filesizeformat",
    "first",
    "float",
    "forceescape",
    "format",
    "groupby",
    "indent",
    "int",
    "join",
    "last",
    "length",
    "list",
    "lower",
    "map",
    "max",
    "min",
    "pprint",
    "random",
    "reject",
    "rejectattr",
    "replace",
    "reverse",
    "round",
    "safe",
    "select",
    "selectattr",
    "slice",
    "sort",
    "string",
    "striptags",
    "sum",
    "title",
    "trim",
    "truncate",
    "unique",
    "upper",
    "urlencode",
    "wordcount",
    "wordwrap",
    "xmlattr",
];

const ALLOWED_TESTS: &[&str] = &[
    "defined",
    "undefined",
    "divisibleby",
    "equalto",
    "escaped",
    "even",
    "ge",
    "gt",
    "in",
    "iterable",
    "le",
    "lt",
    "mapping",
    "none",
    "number",
    "odd",
    "sameas",
    "sequence",
    "string",
    "lower",
    "upper",
    "true",
    "false",
];

pub struct Jinja2TemplateValidator {
    env: Environment<'static>,
    filter_regex: Regex,
    test_regex: Regex,
}

impl Jinja2TemplateValidator {
    pub fn new() -> Self {
        let mut env = Environment::new();
        env.set_auto_escape_callback(|_| minijinja::AutoEscape::None);
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        Self {
            env,
            filter_regex: Regex::new(r"\|\s*([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
            test_regex: Regex::new(r"\bis\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        }
    }

    pub fn validate(&self, source: &str) -> Result<()> {
        self.check_dangerous_patterns(source)?;
        self.try_compile(source)?;
        self.check_allowlists(source)?;
        Ok(())
    }

    fn check_dangerous_patterns(&self, source: &str) -> Result<()> {
        for pattern in JINJA2_DANGEROUS_PATTERNS {
            let patterns_to_check = [
                format!("{{{{{}}}}}", pattern),
                format!("{{{{ {} }}}}", pattern),
                pattern.to_string(),
            ];
            for p in &patterns_to_check {
                if source.contains(p) {
                    return Err(TemplateError::Validation(format!(
                        "Template contains dangerous pattern: {}",
                        pattern
                    )));
                }
            }
        }
        for attr in &[
            "__class__",
            "__mro__",
            "__subclasses__",
            "__globals__",
            "__builtins__",
        ] {
            if source.contains(*attr) {
                return Err(TemplateError::Validation(format!(
                    "Template contains dangerous attribute access: {}",
                    attr
                )));
            }
        }
        Ok(())
    }

    fn try_compile(&self, source: &str) -> Result<()> {
        self.env.template_from_str(source).map_err(|e| {
            TemplateError::Validation(format!("Template compilation failed: {}", e))
        })?;
        Ok(())
    }

    fn check_allowlists(&self, source: &str) -> Result<()> {
        for cap in self.filter_regex.captures_iter(source) {
            if let Some(filter_name) = cap.get(1) {
                let name = filter_name.as_str();
                if !ALLOWED_FILTERS.contains(&name) && !name.starts_with('_') {
                    return Err(TemplateError::Validation(format!(
                        "Filter '{}' is not in allowed filter set",
                        name
                    )));
                }
            }
        }
        for cap in self.test_regex.captures_iter(source) {
            if let Some(test_name) = cap.get(1) {
                let name = test_name.as_str();
                if !ALLOWED_TESTS.contains(&name) && !name.starts_with('_') {
                    return Err(TemplateError::Validation(format!(
                        "Test '{}' is not in allowed test set",
                        name
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Default for Jinja2TemplateValidator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SecurityAdapter {
    capability_checker: CapabilityChecker,
    allowed_paths: HashSet<String>,
    secret: Vec<u8>,
    cns_runtime: Option<Arc<CnsRuntime>>,
    template_validator: Jinja2TemplateValidator,
}

impl SecurityAdapter {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            capability_checker: CapabilityChecker::new(secret),
            allowed_paths: HashSet::new(),
            secret: secret.to_vec(),
            cns_runtime: None,
            template_validator: Jinja2TemplateValidator::new(),
        }
    }

    pub fn with_cns(secret: &[u8], cns_runtime: Arc<CnsRuntime>) -> Self {
        Self {
            capability_checker: CapabilityChecker::new(secret),
            allowed_paths: HashSet::new(),
            secret: secret.to_vec(),
            cns_runtime: Some(cns_runtime),
            template_validator: Jinja2TemplateValidator::new(),
        }
    }

    pub fn get_secret(&self) -> &[u8] {
        &self.secret
    }

    pub fn allow_path(&mut self, path: &str) {
        self.allowed_paths.insert(path.to_string());
    }

    pub fn validate_path(&self, path: &str) -> Result<()> {
        self.validate_template_path(path)
    }

    pub fn validate_template_path(&self, path: &str) -> Result<()> {
        if path.contains('\0') {
            return Err(TemplateError::PathTraversal(
                "Null byte not allowed".to_string(),
            ));
        }
        for pattern in PATH_TRAVERSAL_PATTERNS {
            if path.contains(pattern) {
                return Err(TemplateError::PathTraversal(format!(
                    "Path traversal pattern detected: {}",
                    pattern
                )));
            }
        }
        let decoded = percent_decode_str(path)
            .decode_utf8()
            .map_err(|_| TemplateError::PathTraversal("Invalid UTF-8 in path".to_string()))?;
        if decoded.contains("..") || decoded.starts_with('/') {
            return Err(TemplateError::PathTraversal(
                "Encoded path traversal detected".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_template(&self, template_source: &str) -> Result<()> {
        let result = self.template_validator.validate(template_source);
        if let Err(e) = &result
            && let Some(_cns) = &self.cns_runtime
        {
            let span = hkask_types::Span::Connector("cns.security.jinja2.violation".to_string());
            let observation =
                json!({"error": format!("{}", e), "source_length": template_source.len()});
            let span_emitter = hkask_cns::spans::SpanEmitter::new(WebID::new());
            span_emitter.emit(span, hkask_types::Phase::Observe, observation);
        }
        result
    }

    pub fn verify_signature(&self, token: &CapabilityToken, holder: &WebID) -> bool {
        self.capability_checker.verify(token) && token.delegated_to == *holder
    }

    pub fn sanitize_jinja2_input(&self, input: &str) -> String {
        let mut sanitized = input.to_string();
        for pattern in JINJA2_DANGEROUS_PATTERNS {
            let patterns_to_check = [
                format!("{{{{{}}}}}", pattern),
                format!("{{{{ {} }}}}", pattern),
                pattern.to_string(),
            ];
            for p in &patterns_to_check {
                sanitized = sanitized.replace(p, "{{ BLOCKED }}");
            }
        }
        sanitized
    }

    fn check_capability_scope(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        resource_id: &str,
        current_time: i64,
    ) -> Result<()> {
        if !self.verify_signature(token, holder) {
            return Err(TemplateError::CapabilityDenied(
                "Capability signature invalid or holder mismatch".to_string(),
            ));
        }
        if let Some(exp) = token.expires_at
            && current_time > exp
        {
            return Err(TemplateError::CapabilityDenied(
                "Capability expired".to_string(),
            ));
        }
        if token.resource_id.as_str() != resource_id {
            return Err(TemplateError::CapabilityDenied(format!(
                "Capability not scoped: expected {}, got {:?}",
                resource_id, token.resource_id
            )));
        }
        Ok(())
    }

    pub fn check_template_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        template_id: &str,
        current_time: i64,
    ) -> Result<()> {
        self.check_capability_scope(token, holder, template_id, current_time)
    }

    pub fn check_manifest_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        manifest_id: &str,
        current_time: i64,
    ) -> Result<()> {
        self.check_capability_scope(token, holder, manifest_id, current_time)
    }

    pub fn check_cascade_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        cascade_id: &str,
        current_time: i64,
    ) -> Result<()> {
        self.check_capability_scope(token, holder, cascade_id, current_time)
    }

    pub fn check_stage_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        stage_name: &str,
        current_time: i64,
    ) -> Result<()> {
        self.check_capability_scope(token, holder, stage_name, current_time)
    }

    fn check_stage_capability_with_context(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        stage_name: &str,
        current_time: i64,
        _nonce: &str,
    ) -> Result<()> {
        self.check_stage_capability(token, holder, stage_name, current_time)
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
    ) -> Result<()> {
        self.check_template_capability(token, holder, template_id, current_time)
    }
    fn check_manifest_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        manifest_id: &str,
        current_time: i64,
    ) -> Result<()> {
        self.check_manifest_capability(token, holder, manifest_id, current_time)
    }
    fn check_cascade_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        cascade_id: &str,
        current_time: i64,
    ) -> Result<()> {
        self.check_cascade_capability(token, holder, cascade_id, current_time)
    }
    fn check_stage_capability(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        stage_name: &str,
        current_time: i64,
    ) -> Result<()> {
        self.check_stage_capability(token, holder, stage_name, current_time)
    }
    fn check_stage_capability_with_context(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        stage_name: &str,
        current_time: i64,
        nonce: &str,
    ) -> Result<()> {
        self.check_stage_capability_with_context(token, holder, stage_name, current_time, nonce)
    }
    fn check_recursion_depth(&self, current_depth: u8, max_depth: u8) -> Result<()> {
        if current_depth > max_depth {
            return Err(TemplateError::RecursionLimit { max: max_depth });
        }
        Ok(())
    }
    fn check_energy_budget(&self, requested: u64, remaining: u64) -> Result<()> {
        if requested > remaining {
            return Err(TemplateError::CapabilityDenied(format!(
                "Energy budget exceeded: requested {}, remaining {}",
                requested, remaining
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
        // Use the capability token's built-in attenuation method
        // This ensures proper attenuation chain tracking and expiry reduction
        token.attenuate(new_to, &self.secret, current_time)
    }
    fn validate_path(&self, path: &str) -> Result<()> {
        self.validate_template_path(path)
    }
}






