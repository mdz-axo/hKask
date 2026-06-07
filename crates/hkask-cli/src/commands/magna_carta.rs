//! Magna Carta verification command handler for `kask sovereignty verify`
//!
//! Loads YAML manifests from the magna-carta-verifier skill, runs verification
//! checks (structural audit, absence check, resource verification), and
//! outputs a human-readable report. Behavioral probes are noted as requiring
//! runtime verification (they need a live system).

use crate::cli::SovereigntyAction;
use hkask_types::sovereignty::DataSovereigntyBoundary;

/// Manifest assertion loaded from YAML
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct Manifest {
    principle: String,
    version: String,
    description: String,
    assertions: Vec<Assertion>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct Assertion {
    id: String,
    name: String,
    claim: String,
    method: String,
    #[serde(default)]
    targets: Vec<serde_yaml::Value>,
    #[serde(default)]
    prohibited: Vec<String>,
}

/// Verification result for a single assertion
#[allow(dead_code)]
#[derive(Clone)]
struct AssertionResult {
    id: String,
    name: String,
    method: String,
    status: &'static str, // "pass", "fail", "gap", "skip"
    findings: Vec<String>,
    recommendations: Vec<String>,
}

/// Verification result for a whole principle
struct PrincipleResult {
    principle: String,
    display_name: String,
    assertion_results: Vec<AssertionResult>,
}

/// Full verification report
struct VerificationReport {
    principles: Vec<PrincipleResult>,
    total_pass: usize,
    total_fail: usize,
    total_gap: usize,
    total_skip: usize,
    total_assertions: usize,
}

pub fn run(action: SovereigntyAction) {
    match action {
        SovereigntyAction::Verify { principle, json } => {
            if json {
                let result = verify_json(principle.as_deref());
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_else(|e| serde_json::json!({"error": e.to_string()}).to_string()));
            } else {
                run_verify(principle.as_deref());
            }
        }
        _ => unreachable!("sovereignty::run dispatched wrong variant"),
    }
}

/// Run verification and return JSON report (for MCP tool)
pub fn verify_json(filter: Option<&str>) -> serde_json::Value {
    let report = build_report(filter);
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

fn run_verify(filter: Option<&str>) {
    let report = build_report(filter);
    if report.principles.is_empty() {
        eprintln!(
            "No Magna Carta manifests found. Expected manifests in .agents/skills/magna-carta-verifier/manifests/"
        );
        std::process::exit(1);
    }

    println!("Magna Carta Verification Report");
    println!("==============================");
    println!();

    for pr in &report.principles {
        let mut principle_pass = 0usize;
        let mut principle_fail = 0usize;
        let mut principle_gap = 0usize;

        println!("## {}", pr.display_name);
        println!();

        for result in &pr.assertion_results {
            let status_icon = match result.status {
                "pass" => "✓",
                "fail" => "✗",
                "gap" => "△",
                "skip" => "—",
                _ => "?",
            };

            println!(
                "  {status_icon} {id} {name}: {status}",
                id = result.id,
                name = result.name,
                status = result.status
            );

            for finding in &result.findings {
                println!("    → {finding}");
            }
            for rec in &result.recommendations {
                println!("    ⚑ {rec}");
            }

            match result.status {
                "pass" => principle_pass += 1,
                "fail" => principle_fail += 1,
                "gap" => principle_gap += 1,
                _ => {}
            }
        }

        println!();
        println!(
            "  Principle summary: {pass} pass, {fail} fail, {gap} gap",
            pass = principle_pass,
            fail = principle_fail,
            gap = principle_gap,
        );
        println!();
    }

    println!("---");
    println!(
        "Total: {} assertions — {} pass, {} fail, {} gap, {} skip",
        report.total_assertions,
        report.total_pass,
        report.total_fail,
        report.total_gap,
        report.total_skip
    );

    if report.total_fail > 0 || report.total_gap > 0 {
        println!();
        println!(
            "⚠ {} assertion(s) failed and {} have gaps.",
            report.total_fail, report.total_gap
        );
        println!("  Escalate to Curator for review with human user or replicant.");
    }
}

fn build_report(filter: Option<&str>) -> VerificationReport {
    let manifests = load_manifests();

    let principle_display_names = [
        ("user_sovereignty", "P1 — User Sovereignty"),
        ("affirmative_consent", "P2 — Affirmative Consent"),
        ("generative_space", "P3 — Generative Space"),
        ("clear_boundaries", "P4 — Clear Boundaries / OCAP"),
    ];

    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    let mut total_gap = 0usize;
    let mut total_skip = 0usize;
    let mut total_assertions = 0usize;
    let mut principles = Vec::new();

    for manifest in &manifests {
        // Skip if filter is set and doesn't match
        if let Some(f) = filter {
            let principle_aliases = [
                ("p1", "user_sovereignty"),
                ("p2", "affirmative_consent"),
                ("p3", "generative_space"),
                ("p4", "clear_boundaries"),
            ];
            let resolved = principle_aliases
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

        let display_name = principle_display_names
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

fn load_manifests() -> Vec<Manifest> {
    let manifest_dir = std::path::Path::new(".agents/skills/magna-carta-verifier/manifests");
    if !manifest_dir.exists() {
        return Vec::new();
    }

    let mut manifests = Vec::new();
    let dir_result = std::fs::read_dir(manifest_dir);
    let entries = match dir_result {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Warning: Cannot read manifests directory: {e}");
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
                Err(e) => eprintln!("Warning: Failed to parse {}: {e}", path.display()),
            },
            Err(e) => eprintln!("Warning: Failed to read {}: {e}", path.display()),
        }
    }

    // Sort by principle for deterministic output
    manifests.sort_by(|a, b| a.principle.cmp(&b.principle));
    manifests
}

fn verify_assertion(assertion: &Assertion) -> AssertionResult {
    match assertion.method.as_str() {
        "structural_audit" => verify_structural_audit(assertion),
        "behavioral_probe" => AssertionResult {
            id: assertion.id.clone(),
            name: assertion.name.clone(),
            method: assertion.method.clone(),
            status: "skip",
            findings: vec!["Behavioral probes require a live system runtime. Run `cargo test` for behavioral verification.".to_string()],
            recommendations: vec![format!("Write and run a #[test] for {} that exercises the denial path", assertion.id)],
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

        // Check that the target crate directory exists
        let crate_dir = std::path::PathBuf::from(format!("crates/{crate_name}/src"));
        if !crate_dir.exists() {
            findings.push(format!(
                "Crate directory not found: {}",
                crate_dir.display()
            ));
            all_pass = false;
            continue;
        }

        // Convert module path to file path (e.g., "pod::context" → "pod/context.rs" or "pod/context/mod.rs")
        let module_file = module.replace("::", "/");
        let file_path = crate_dir.join(format!("{module_file}.rs"));
        let mod_path = crate_dir.join(format!("{module_file}/mod.rs"));

        if !file_path.exists() && !mod_path.exists() {
            findings.push(format!("Module not found: {crate_name}::{module}"));
            all_pass = false;
            continue;
        }

        // Check that target methods exist in the source file
        if let Some(methods) = target.get("methods").and_then(|v| v.as_sequence()) {
            let source = if file_path.exists() {
                std::fs::read_to_string(&file_path).unwrap_or_default()
            } else {
                std::fs::read_to_string(&mod_path).unwrap_or_default()
            };

            for method_val in methods {
                let method_name = method_val.as_str().unwrap_or("");
                // Simple check: method name appears in source (not a full parser, but sufficient for audit)
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

        let source = if file_path.exists() {
            std::fs::read_to_string(&file_path).unwrap_or_default()
        } else {
            std::fs::read_to_string(&mod_path).unwrap_or_default()
        };

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
    // For resource_verification, check DataSovereigntyBoundary categorization
    let boundary = DataSovereigntyBoundary::hkask_default();
    let mut findings = Vec::new();
    let mut all_pass = true;

    // Verify each DataCategory is assigned to exactly one tier
    use hkask_types::sovereignty::DataCategory;
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

    // Verify requires_affirmative_consent is true (default boundary)
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

    // Get target crate directories
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

/// Grep a crate directory for a pattern, return match count.
fn grep_crate(crate_dir: &str, pattern: &str) -> Result<usize, String> {
    let mut count = 0usize;
    walk_dir(crate_dir, &mut |path| {
        if let Ok(content) = std::fs::read_to_string(path) {
            // Case-insensitive search for prohibited patterns
            let lower_content = content.to_lowercase();
            let lower_pattern = pattern.to_lowercase();
            if lower_content.contains(&lower_pattern) {
                count += 1;
            }
        }
    })?;
    Ok(count)
}

/// Walk a directory recursively, calling f for each .rs file
fn walk_dir(dir: &str, f: &mut dyn FnMut(&std::path::Path)) -> Result<(), String> {
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
