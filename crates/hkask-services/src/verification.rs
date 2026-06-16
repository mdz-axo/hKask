//! VerificationService — Magna Carta sovereignty verification.
//! # REQ: P4 (Clear Boundaries) — verifies P1-P4 are enforced through OCAP gates.
//!
//! Loads YAML manifests defining assertions against hKask's four Magna Carta
//! principles (User Sovereignty, Affirmative Consent, Generative Space, Clear
//! Boundaries), dispatches verification methods, and produces structured reports.

use hkask_types::sovereignty::{DataCategory, DataSovereigntyBoundary};
use serde::Deserialize;
use std::path::{Path, PathBuf};

// ── Domain types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub principle: String,
    pub version: String,
    pub description: String,
    pub assertions: Vec<Assertion>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub id: String,
    pub name: String,
    pub method: String,
    pub status: &'static str,
    pub findings: Vec<String>,
    pub recommendations: Vec<String>,
}

impl AssertionResult {
    fn new(
        assertion: &Assertion,
        status: &'static str,
        findings: Vec<String>,
        recommendations: Vec<String>,
    ) -> Self {
        Self {
            id: assertion.id.clone(),
            name: assertion.name.clone(),
            method: assertion.method.clone(),
            status,
            findings,
            recommendations,
        }
    }
    fn skip(assertion: &Assertion, reason: String) -> Self {
        Self::new(assertion, "skip", vec![reason], vec![])
    }
    fn gap(assertion: &Assertion, reason: String) -> Self {
        Self::new(
            assertion,
            "gap",
            vec![reason],
            vec!["Update verifier to support this method".to_string()],
        )
    }
    fn verdict(assertion: &Assertion, all_pass: bool, findings: Vec<String>) -> Self {
        Self::new(
            assertion,
            if all_pass { "pass" } else { "fail" },
            findings,
            vec![],
        )
    }
}

#[derive(Debug)]
pub struct PrincipleResult {
    pub principle: String,
    pub display_name: String,
    pub assertion_results: Vec<AssertionResult>,
}

#[derive(Debug)]
pub struct VerificationReport {
    pub principles: Vec<PrincipleResult>,
    pub total_pass: usize,
    pub total_fail: usize,
    pub total_gap: usize,
    pub total_skip: usize,
    pub total_assertions: usize,
}

pub struct VerificationService;

impl VerificationService {
    /// REQ: SVC-232
    /// pre:  filter if Some must be a valid principle name; manifests must be loadable
    /// post: returns VerificationReport with principle results, pass/fail/gap/skip counts, and total assertions
    pub fn verify(filter: Option<&str>) -> VerificationReport {
        build_report(&load_manifests(), filter)
    }
    /// REQ: SVC-233
    /// pre:  filter if Some must be a valid principle name
    /// post: returns serde_json::Value with principles array, totals, and escalation_required flag
    pub fn verify_json(filter: Option<&str>) -> serde_json::Value {
        let report = Self::verify(filter);
        serde_json::json!({
            "principles": report.principles.iter().map(|pr| serde_json::json!({
                "principle": pr.principle, "display_name": pr.display_name,
                "assertions": pr.assertion_results.iter().map(|ar| serde_json::json!({
                    "id": ar.id, "name": ar.name, "method": ar.method,
                    "status": ar.status, "findings": ar.findings,
                    "recommendations": ar.recommendations,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "total_pass": report.total_pass, "total_fail": report.total_fail,
            "total_gap": report.total_gap, "total_skip": report.total_skip,
            "total_assertions": report.total_assertions,
            "escalation_required": report.total_fail > 0 || report.total_gap > 0,
        })
    }
}

// ── Internal: manifest loading ──────────────────────────────────────────

fn load_manifests() -> Vec<Manifest> {
    let manifest_dir = Path::new(".agents/skills/magna-carta-verifier/manifests");
    if !manifest_dir.exists() {
        return Vec::new();
    }
    let mut manifests = Vec::new();
    let entries = match std::fs::read_dir(manifest_dir) {
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

fn resolve_filter(alias: &str) -> &str {
    PRINCIPLE_ALIASES
        .iter()
        .find(|(a, _)| a == &alias)
        .map(|(_, p)| *p)
        .unwrap_or(alias)
}

fn build_report(manifests: &[Manifest], filter: Option<&str>) -> VerificationReport {
    let mut counts = (0usize, 0usize, 0usize, 0usize, 0usize); // pass, fail, gap, skip, total
    let mut principles = Vec::new();
    for manifest in manifests {
        if let Some(f) = filter {
            let resolved = resolve_filter(f);
            if resolved != manifest.principle && !manifest.principle.starts_with(resolved) {
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
            counts.4 += 1;
            match result.status {
                "pass" => counts.0 += 1,
                "fail" => counts.1 += 1,
                "gap" => counts.2 += 1,
                _ => counts.3 += 1,
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
        total_pass: counts.0,
        total_fail: counts.1,
        total_gap: counts.2,
        total_skip: counts.3,
        total_assertions: counts.4,
    }
}

// ── Internal: assertion verification ────────────────────────────────────

fn verify_assertion(assertion: &Assertion) -> AssertionResult {
    match assertion.method.as_str() {
        "structural_audit" => verify_structural_audit(assertion),
        "behavioral_probe" => AssertionResult::skip(assertion,
            "Behavioral probes require a live system runtime. Run `cargo test` for behavioral verification.".to_string()),
        "resource_verification" => verify_resource_verification(assertion),
        "absence_check" => verify_absence_check(assertion),
        _ => AssertionResult::gap(assertion, format!("Unknown verification method: {}", assertion.method)),
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
        if let Some(methods) = target.get("methods").and_then(|v| v.as_sequence()) {
            for mv in methods {
                let name = mv.as_str().unwrap_or("");
                if !source.contains(name) {
                    findings.push(format!(
                        "Method `{name}` not found in {crate_name}::{module}"
                    ));
                    all_pass = false;
                }
            }
        }
        if let Some(gate) = target.get("gate").and_then(|v| v.as_str())
            && !source.contains(gate)
        {
            findings.push(format!("Gate `{gate}` not found in {crate_name}::{module}"));
            all_pass = false;
        }
        if let Some(gates) = target.get("gates").and_then(|v| v.as_sequence()) {
            for gv in gates {
                let gname = gv.as_str().unwrap_or("");
                if !source.contains(gname) {
                    findings.push(format!(
                        "Gate `{gname}` not found in {crate_name}::{module}"
                    ));
                    all_pass = false;
                }
            }
        }
    }
    AssertionResult::verdict(assertion, all_pass, findings)
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
        let tier_count = boundary.is_sovereign(cat) as usize
            + boundary.is_category_shared(cat) as usize
            + boundary.is_category_public(cat) as usize;
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
    AssertionResult::verdict(assertion, all_pass, findings)
}

fn verify_absence_check(assertion: &Assertion) -> AssertionResult {
    let prohibited = &assertion.prohibited;
    if prohibited.is_empty() {
        return AssertionResult::verdict(
            assertion,
            true,
            vec!["No prohibited patterns specified — vacuously true".to_string()],
        );
    }
    let crate_dirs: Vec<String> = assertion
        .targets
        .iter()
        .filter_map(|t| t.get("crate").and_then(|v| v.as_str()))
        .map(|c| format!("crates/{c}"))
        .collect();
    let mut findings = Vec::new();
    let mut all_pass = true;
    for pattern in prohibited {
        let mut found = 0;
        for crate_dir in &crate_dirs {
            match grep_crate(crate_dir, pattern) {
                Ok(count) => found += count,
                Err(e) => {
                    findings.push(format!("Error searching {crate_dir} for `{pattern}`: {e}"))
                }
            }
        }
        if found > 0 {
            findings.push(format!(
                "Prohibited pattern `{pattern}` found ({found} match(es))"
            ));
            all_pass = false;
        }
    }
    AssertionResult::verdict(assertion, all_pass, findings)
}

// ── Internal: codebase scanning ─────────────────────────────────────────

fn grep_crate(crate_dir: &str, pattern: &str) -> Result<usize, String> {
    let mut count = 0usize;
    let lower_pattern = pattern.to_lowercase();
    walk_dir(crate_dir, &mut |path| {
        if let Ok(content) = std::fs::read_to_string(path)
            && content.to_lowercase().contains(&lower_pattern)
        {
            count += 1;
        }
    })?;
    Ok(count)
}

fn walk_dir(dir: &str, f: &mut dyn FnMut(&Path)) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(path.to_str().unwrap_or(""), f)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            f(&path);
        }
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────
