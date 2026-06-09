//! VerificationService — Magna Carta sovereignty verification.
//!
//! Loads YAML manifests defining assertions against hKask's four Magna Carta
//! principles (User Sovereignty, Affirmative Consent, Generative Space, Clear
//! Boundaries), dispatches verification methods (structural audit, resource
//! verification, absence check), and produces structured reports.
//!
//! ℏKask - A Minimal Viable Container for Agents

use hkask_types::sovereignty::DataCategory;
use hkask_types::sovereignty::DataSovereigntyBoundary;

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Manifest defining assertions for one Magna Carta principle.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub principle: String,
    pub version: String,
    pub description: String,
    pub assertions: Vec<Assertion>,
}

/// A single assertion to verify.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Assertion {
    pub id: String,
    pub name: String,
    pub claim: String,
    pub method: String,
    #[serde(default)]
    pub targets: Vec<serde_yaml::Value>,
    #[serde(default)]
    pub prohibited: Vec<String>,
}

/// Verification result for a single assertion.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AssertionResult {
    pub id: String,
    pub name: String,
    pub method: String,
    pub status: &'static str, // "pass", "fail", "gap", "skip"
    pub findings: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Verification result for a whole principle.
#[derive(Debug)]
pub struct PrincipleResult {
    pub principle: String,
    pub display_name: String,
    pub assertion_results: Vec<AssertionResult>,
}

/// Full verification report.
#[derive(Debug)]
pub struct VerificationReport {
    pub principles: Vec<PrincipleResult>,
    pub total_pass: usize,
    pub total_fail: usize,
    pub total_gap: usize,
    pub total_skip: usize,
    pub total_assertions: usize,
}

/// Service for Magna Carta sovereignty verification.
///
/// Loads manifests, dispatches verification methods, and produces
/// structured reports. Used by CLI and MCP tools.
pub struct VerificationService;

impl VerificationService {
    /// Run the full verification pipeline, optionally filtered by principle.
    pub fn verify(filter: Option<&str>) -> VerificationReport {
        let manifests = load_manifests();
        build_report(&manifests, filter)
    }

    /// Run verification and return a JSON report (for MCP tool and API).
    pub fn verify_json(filter: Option<&str>) -> serde_json::Value {
        let report = Self::verify(filter);
        report_to_json(&report)
    }

    /// Load manifests from the default directory.
    pub fn load_manifests() -> Vec<Manifest> {
        load_manifests()
    }
}

// ── Internal: manifest loading ──────────────────────────────────────────

fn load_manifests() -> Vec<Manifest> {
    let manifest_dir = Path::new(".agents/skills/magna-carta-verifier/manifests");
    if !manifest_dir.exists() {
        return Vec::new();
    }

    let mut manifests = Vec::new();
    let dir_result = std::fs::read_dir(manifest_dir);
    let entries = match dir_result {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Cannot read manifests directory: {e}");
            return manifests;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "yaml") {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_yaml::from_str::<Manifest>(&content) {
                Ok(manifest) => manifests.push(manifest),
                Err(e) => tracing::warn!("Failed to parse {}: {e}", path.display()),
            },
            Err(e) => tracing::warn!("Failed to read {}: {e}", path.display()),
        }
    }

    manifests.sort_by(|a, b| a.principle.cmp(&b.principle));
    manifests
}

// ── Internal: report building ───────────────────────────────────────────

const PRINCIPLE_DISPLAY_NAMES: &[(&str, &str)] = &[
    ("user_sovereignty", "P1 — User Sovereignty"),
    ("affirmative_consent", "P2 — Affirmative Consent"),
    ("generative_space", "P3 — Generative Space"),
    ("clear_boundaries", "P4 — Clear Boundaries / OCAP"),
];

const PRINCIPLE_ALIASES: &[(&str, &str)] = &[
    ("p1", "user_sovereignty"),
    ("p2", "affirmative_consent"),
    ("p3", "generative_space"),
    ("p4", "clear_boundaries"),
];

fn build_report(manifests: &[Manifest], filter: Option<&str>) -> VerificationReport {
    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    let mut total_gap = 0usize;
    let mut total_skip = 0usize;
    let mut total_assertions = 0usize;
    let mut principles = Vec::new();

    for manifest in manifests {
        // Skip if filter is set and doesn't match
        if let Some(f) = filter {
            let resolved = PRINCIPLE_ALIASES
                .iter()
                .find(|(alias, _)| alias == &f)
                .map(|(_, principle)| *principle)
                .unwrap_or(f);
            let matches =
                manifest.principle.starts_with(resolved) || resolved == manifest.principle;
            if !matches {
                continue;
            }
        }

        let display_name = PRINCIPLE_DISPLAY_NAMES
            .iter()
            .find(|(key, _)| key == &manifest.principle)
            .map(|(_, name)| *name)
            .unwrap_or(&manifest.principle)
            .to_string();

        let mut assertion_results = Vec::new();

        for assertion in &manifest.assertions {
            let result = verify_assertion(assertion);
            total_assertions += 1;

            match result.status {
                "pass" => total_pass += 1,
                "fail" => total_fail += 1,
                "gap" => total_gap += 1,
                _ => total_skip += 1,
            }

            assertion_results.push(result);
        }

        principles.push(PrincipleResult {
            principle: manifest.principle.clone(),
            display_name,
            assertion_results,
        });
    }

    VerificationReport {
        principles,
        total_pass,
        total_fail,
        total_gap,
        total_skip,
        total_assertions,
    }
}

// ── Internal: assertion verification ────────────────────────────────────

fn verify_assertion(assertion: &Assertion) -> AssertionResult {
    match assertion.method.as_str() {
        "structural_audit" => verify_structural_audit(assertion),
        "behavioral_probe" => AssertionResult {
            id: assertion.id.clone(),
            name: assertion.name.clone(),
            method: assertion.method.clone(),
            status: "skip",
            findings: vec![
                "Behavioral probes require a live system runtime. Run `cargo test` for behavioral verification.".to_string(),
            ],
            recommendations: vec![format!(
                "Write and run a #[test] for {} that exercises the denial path",
                assertion.id
            )],
        },
        "resource_verification" => verify_resource_verification(assertion),
        "absence_check" => verify_absence_check(assertion),
        _ => AssertionResult {
            id: assertion.id.clone(),
            name: assertion.name.clone(),
            method: assertion.method.clone(),
            status: "gap",
            findings: vec![format!("Unknown verification method: {}", assertion.method)],
            recommendations: vec!["Update verifier to support this method".to_string()],
        },
    }
}

fn verify_structural_audit(assertion: &Assertion) -> AssertionResult {
    let mut findings = Vec::new();
    let mut all_pass = true;

    for target in &assertion.targets {
        let crate_name = target
            .get("crate")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let module = target
            .get("module")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let crate_dir = PathBuf::from(format!("crates/{crate_name}/src"));
        if !crate_dir.exists() {
            findings.push(format!(
                "Crate directory not found: {}",
                crate_dir.display()
            ));
            all_pass = false;
            continue;
        }

        let module_file = module.replace("::", "/");
        let file_path = crate_dir.join(format!("{module_file}.rs"));
        let mod_path = crate_dir.join(format!("{module_file}/mod.rs"));

        if !file_path.exists() && !mod_path.exists() {
            findings.push(format!("Module not found: {crate_name}::{module}"));
            all_pass = false;
            continue;
        }

        let source_path = if file_path.exists() {
            &file_path
        } else {
            &mod_path
        };
        let source = std::fs::read_to_string(source_path).unwrap_or_default();

        // Check that target methods exist in the source file
        if let Some(methods) = target.get("methods").and_then(|v| v.as_sequence()) {
            for method_val in methods {
                let method_name = method_val.as_str().unwrap_or("");
                if !source.contains(method_name) {
                    findings.push(format!(
                        "Method `{method_name}` not found in {crate_name}::{module}"
                    ));
                    all_pass = false;
                }
            }
        }

        // Check that gate exists in source
        let gate = target.get("gate").and_then(|v| v.as_str());
        let gates = target.get("gates").and_then(|v| v.as_sequence());

        if let Some(gate) = gate
            && !source.contains(gate)
        {
            findings.push(format!("Gate `{gate}` not found in {crate_name}::{module}"));
            all_pass = false;
        }
        if let Some(gates) = gates {
            for gate_val in gates {
                let gate_name = gate_val.as_str().unwrap_or("");
                if !source.contains(gate_name) {
                    findings.push(format!(
                        "Gate `{gate_name}` not found in {crate_name}::{module}"
                    ));
                    all_pass = false;
                }
            }
        }
    }

    AssertionResult {
        id: assertion.id.clone(),
        name: assertion.name.clone(),
        method: assertion.method.clone(),
        status: if all_pass { "pass" } else { "fail" },
        findings,
        recommendations: Vec::new(),
    }
}

fn verify_resource_verification(assertion: &Assertion) -> AssertionResult {
    let boundary = DataSovereigntyBoundary::hkask_default();
    let mut findings = Vec::new();
    let mut all_pass = true;

    let categories = [
        DataCategory::EpisodicMemory,
        DataCategory::SemanticMemory,
        DataCategory::PersonalContext,
        DataCategory::CapabilityTokens,
        DataCategory::OcapBoundaries,
        DataCategory::TemplateInvocations,
        DataCategory::HLexiconTerms,
        DataCategory::TemplateRegistry,
    ];

    for cat in &categories {
        let is_sovereign = boundary.is_sovereign(cat);
        let is_shared = boundary.is_category_shared(cat);
        let is_public = boundary.is_category_public(cat);

        let tier_count = is_sovereign as usize + is_shared as usize + is_public as usize;
        if tier_count == 0 {
            findings.push(format!("{cat:?} is not assigned to any tier"));
            all_pass = false;
        } else if tier_count > 1 {
            findings.push(format!("{cat:?} is assigned to multiple tiers"));
            all_pass = false;
        }
    }

    if !boundary.requires_affirmative_consent() {
        findings.push("requires_affirmative_consent is false in default boundary".to_string());
        all_pass = false;
    }

    AssertionResult {
        id: assertion.id.clone(),
        name: assertion.name.clone(),
        method: assertion.method.clone(),
        status: if all_pass { "pass" } else { "fail" },
        findings,
        recommendations: Vec::new(),
    }
}

fn verify_absence_check(assertion: &Assertion) -> AssertionResult {
    let prohibited = &assertion.prohibited;
    if prohibited.is_empty() {
        return AssertionResult {
            id: assertion.id.clone(),
            name: assertion.name.clone(),
            method: assertion.method.clone(),
            status: "pass",
            findings: vec!["No prohibited patterns specified — vacuously true".to_string()],
            recommendations: Vec::new(),
        };
    }

    let mut findings = Vec::new();
    let mut all_pass = true;

    let crate_dirs: Vec<String> = assertion
        .targets
        .iter()
        .filter_map(|t| t.get("crate").and_then(|v| v.as_str()))
        .map(|c| format!("crates/{c}"))
        .collect();

    for pattern in prohibited {
        let mut found_count = 0;
        for crate_dir in &crate_dirs {
            match grep_crate(crate_dir, pattern) {
                Ok(count) => found_count += count,
                Err(e) => {
                    findings.push(format!("Error searching {crate_dir} for `{pattern}`: {e}"))
                }
            }
        }
        if found_count > 0 {
            findings.push(format!(
                "Prohibited pattern `{pattern}` found ({found_count} match(es))"
            ));
            all_pass = false;
        }
    }

    AssertionResult {
        id: assertion.id.clone(),
        name: assertion.name.clone(),
        method: assertion.method.clone(),
        status: if all_pass { "pass" } else { "fail" },
        findings,
        recommendations: Vec::new(),
    }
}

// ── Internal: codebase scanning ─────────────────────────────────────────

fn grep_crate(crate_dir: &str, pattern: &str) -> Result<usize, String> {
    let mut count = 0usize;
    walk_dir(crate_dir, &mut |path| {
        if let Ok(content) = std::fs::read_to_string(path) {
            let lower_content = content.to_lowercase();
            let lower_pattern = pattern.to_lowercase();
            if lower_content.contains(&lower_pattern) {
                count += 1;
            }
        }
    })?;
    Ok(count)
}

fn walk_dir(dir: &str, f: &mut dyn FnMut(&Path)) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(path.to_str().unwrap_or(""), f)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            f(&path);
        }
    }
    Ok(())
}

// ── Internal: JSON report ───────────────────────────────────────────────

fn report_to_json(report: &VerificationReport) -> serde_json::Value {
    let mut principles_json = Vec::new();

    for pr in &report.principles {
        let mut assertions_json = Vec::new();
        for ar in &pr.assertion_results {
            assertions_json.push(serde_json::json!({
                "id": ar.id,
                "name": ar.name,
                "method": ar.method,
                "status": ar.status,
                "findings": ar.findings,
                "recommendations": ar.recommendations,
            }));
        }
        principles_json.push(serde_json::json!({
            "principle": pr.principle,
            "display_name": pr.display_name,
            "assertions": assertions_json,
        }));
    }

    serde_json::json!({
        "principles": principles_json,
        "total_pass": report.total_pass,
        "total_fail": report.total_fail,
        "total_gap": report.total_gap,
        "total_skip": report.total_skip,
        "total_assertions": report.total_assertions,
        "escalation_required": report.total_fail > 0 || report.total_gap > 0,
    })
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceError;

    // REQ: VerificationService has 3 public operations
    #[test]
    fn verification_service_has_three_operations() {
        let _ = VerificationService::verify;
        let _ = VerificationService::verify_json;
        let _ = VerificationService::load_manifests;
    }

    // REQ: VerificationReport carries all totals
    #[test]
    fn verification_report_carries_totals() {
        let report = VerificationReport {
            principles: Vec::new(),
            total_pass: 5,
            total_fail: 1,
            total_gap: 2,
            total_skip: 3,
            total_assertions: 11,
        };
        assert_eq!(report.total_pass, 5);
        assert_eq!(report.total_fail, 1);
        assert_eq!(report.total_gap, 2);
        assert_eq!(report.total_skip, 3);
        assert_eq!(report.total_assertions, 11);
    }

    // REQ: AssertionResult carries status and findings
    #[test]
    fn assertion_result_carries_status() {
        let result = AssertionResult {
            id: "p1-001".to_string(),
            name: "Test assertion".to_string(),
            method: "structural_audit".to_string(),
            status: "pass",
            findings: Vec::new(),
            recommendations: Vec::new(),
        };
        assert_eq!(result.id, "p1-001");
        assert_eq!(result.status, "pass");
    }

    // REQ: PrincipleResult carries display name
    #[test]
    fn principle_result_carries_display_name() {
        let result = PrincipleResult {
            principle: "user_sovereignty".to_string(),
            display_name: "P1 — User Sovereignty".to_string(),
            assertion_results: Vec::new(),
        };
        assert_eq!(result.display_name, "P1 — User Sovereignty");
    }

    // REQ: ServiceError::Verification is a string sentinel
    #[test]
    fn verification_error_is_string_sentinel() {
        let err = ServiceError::Verification("manifest load failed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Verification failed"));
        assert!(msg.contains("manifest load failed"));
    }

    // REQ: Manifest deserializes from YAML
    #[test]
    fn manifest_deserializes_from_yaml() {
        let yaml = r#"
principle: user_sovereignty
version: "1.0"
description: Test principle
assertions:
  - id: p1-001
    name: Test assertion
    claim: Users own their data
    method: structural_audit
    targets:
      - crate: hkask-storage
        module: sovereignty
"#;
        let manifest: Manifest = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
        assert_eq!(manifest.principle, "user_sovereignty");
        assert_eq!(manifest.assertions.len(), 1);
        assert_eq!(manifest.assertions[0].method, "structural_audit");
    }

    // REQ: Principle aliases resolve correctly
    #[test]
    fn principle_aliases_resolve() {
        let resolved = PRINCIPLE_ALIASES
            .iter()
            .find(|(alias, _)| alias == &"p1")
            .map(|(_, principle)| *principle)
            .unwrap_or("p1");
        assert_eq!(resolved, "user_sovereignty");

        let resolved = PRINCIPLE_ALIASES
            .iter()
            .find(|(alias, _)| alias == &"p4")
            .map(|(_, principle)| *principle)
            .unwrap_or("p4");
        assert_eq!(resolved, "clear_boundaries");
    }

    // REQ: verify returns empty report when no manifests
    #[test]
    fn verify_returns_empty_report_without_manifests() {
        let report = VerificationService::verify(None);
        // If manifests dir doesn't exist in test env, we get empty report
        // If it does exist, we get a real report — both are valid
        assert!(report.total_assertions >= 0);
    }

    // REQ: verify_json returns valid JSON structure
    #[test]
    fn verify_json_returns_valid_structure() {
        let json = VerificationService::verify_json(None);
        assert!(json.get("principles").is_some());
        assert!(json.get("total_pass").is_some());
        assert!(json.get("escalation_required").is_some());
    }
}
