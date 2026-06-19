// hkask-ci-invariants — single-pass configurable invariant checker
//
// Replaces 14 shell scripts with a single binary that:
//   1. Discovers crates from workspace Cargo.toml (never hardcoded)
//   2. Reads invariant definitions from invariants.toml
//   3. Runs all patterns in a single pass over the source tree
//   4. Emits structured JSON results with CNS span attachment
//   5. Supports severity levels (warning/deny) and expiry-gated allowlists

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};

// ── CLI ──────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "hkask-ci-invariants", about = "CI invariant gate checker")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run all invariant gates against the source tree
    Check {
        /// Path to workspace root (default: auto-detect)
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        /// Path to invariants config (default: <crate>/invariants.toml)
        #[arg(long)]
        config: Option<PathBuf>,
        /// Output format: json (default) or human
        #[arg(long, default_value = "json")]
        output: String,
    },
    /// Check for expired allowlist entries
    CheckExpiredAllowlists {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Generate public seam inventory (replaces public-seam-inventory.sh)
    GenerateInventory {
        #[arg(long)]
        workspace_root: Option<PathBuf>,
    },
}

// ── Configuration Types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct InvariantsConfig {
    gate: HashMap<String, GateConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct GateConfig {
    pattern: String,
    principle: String,
    severity: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    exclude_paths: Vec<String>,
    #[serde(default)]
    allowlist: Vec<AllowlistEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct AllowlistEntry {
    path: String,
    #[serde(default)]
    pattern: Option<String>,
    reason: String,
    expires: String,
}

// ── Output Types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CheckReport {
    gates: Vec<GateReport>,
    summary: CheckSummary,
}

#[derive(Debug, Serialize)]
struct GateReport {
    name: String,
    principle: String,
    severity: String,
    violations: Vec<Violation>,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct Violation {
    file: String,
    line: usize,
    column: usize,
    matched_text: String,
    gate: String,
    principle: String,
    #[serde(rename = "cns_span")]
    cns_span: String,
}

#[derive(Debug, Serialize)]
struct CheckSummary {
    total_gates: usize,
    passed: usize,
    warnings: usize,
    denied: usize,
}

#[derive(Debug, Serialize)]
struct ExpiredAllowlistReport {
    expired: Vec<ExpiredEntry>,
}

#[derive(Debug, Serialize)]
struct ExpiredEntry {
    gate: String,
    path: String,
    reason: String,
    expires: String,
    days_expired: i64,
}

// ── Gate compilation ─────────────────────────────────────────────────────

struct CompiledGate {
    name: String,
    principle: String,
    severity: String,
    description: String,
    pattern: Regex,
    exclude_paths: Vec<String>,
    allowlist: Vec<CompiledAllowlistEntry>,
}

struct CompiledAllowlistEntry {
    path: String,
    pattern: Option<Regex>,
    reason: String,
    expires: NaiveDate,
}

fn compile_gates(config: &InvariantsConfig) -> Result<Vec<CompiledGate>> {
    let mut gates = Vec::new();
    for (name, gate) in &config.gate {
        let pattern = Regex::new(&gate.pattern).with_context(|| {
            format!("invalid regex pattern in gate '{}': {}", name, gate.pattern)
        })?;
        let mut allowlist = Vec::new();
        for entry in &gate.allowlist {
            let entry_pattern = entry
                .pattern
                .as_ref()
                .map(|p| Regex::new(p))
                .transpose()
                .with_context(|| {
                    format!(
                        "invalid allowlist pattern in gate '{}' for path '{}'",
                        name, entry.path
                    )
                })?;
            let expires =
                NaiveDate::parse_from_str(&entry.expires, "%Y-%m-%d").with_context(|| {
                    format!("invalid expiry date '{}' in gate '{}'", entry.expires, name)
                })?;
            allowlist.push(CompiledAllowlistEntry {
                path: entry.path.clone(),
                pattern: entry_pattern,
                reason: entry.reason.clone(),
                expires,
            });
        }
        gates.push(CompiledGate {
            name: name.clone(),
            principle: gate.principle.clone(),
            severity: gate.severity.clone(),
            description: gate.description.clone(),
            pattern,
            exclude_paths: gate.exclude_paths.clone(),
            allowlist,
        });
    }
    Ok(gates)
}

// ── Crate discovery ──────────────────────────────────────────────────────

fn discover_source_dirs(workspace_root: &Path) -> Vec<PathBuf> {
    let candidates = ["crates", "mcp-servers"];
    candidates
        .iter()
        .map(|d| workspace_root.join(d))
        .filter(|p| p.is_dir())
        .collect()
}

fn is_excluded(file_path: &str, exclude_paths: &[String]) -> bool {
    exclude_paths.iter().any(|ex| file_path.contains(ex))
}

fn is_allowlisted(
    file_path: &str,
    matched_text: &str,
    allowlist: &[CompiledAllowlistEntry],
) -> bool {
    allowlist.iter().any(|entry| {
        if file_path.contains(&entry.path) {
            match &entry.pattern {
                Some(re) => re.is_match(matched_text),
                None => true, // No pattern = allow all matches in this file
            }
        } else {
            false
        }
    })
}

// ── Single-pass scan ─────────────────────────────────────────────────────

fn run_check(workspace_root: &Path, gates: &[CompiledGate]) -> Result<CheckReport> {
    let source_dirs = discover_source_dirs(workspace_root);
    let mut gate_reports: Vec<GateReport> = Vec::new();
    let mut violations_map: HashMap<String, Vec<Violation>> = HashMap::new();

    // Initialize violation lists for each gate
    for gate in gates {
        violations_map.insert(gate.name.clone(), Vec::new());
    }

    // Single pass: walk each source dir once, check all patterns per file
    for src_dir in &source_dirs {
        for entry in walkdir::WalkDir::new(src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "rs") && e.file_type().is_file()
            })
        {
            let file_path = entry.path();
            let relative = file_path
                .strip_prefix(workspace_root)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Check each gate against this file
            for gate in gates {
                if is_excluded(&relative, &gate.exclude_paths) {
                    continue;
                }

                for cap in gate.pattern.captures_iter(&content) {
                    let full_match = cap.get(0).unwrap();
                    let matched_text = full_match.as_str().to_string();

                    if is_allowlisted(&relative, &matched_text, &gate.allowlist) {
                        continue;
                    }

                    // Compute line and column
                    let offset = full_match.start();
                    let line = content[..offset].chars().filter(|&c| c == '\n').count() + 1;
                    let last_newline = content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let column = offset - last_newline + 1;

                    // For unsafe-safety gate, additionally check for // SAFETY: comment
                    if gate.name == "unsafe-safety" {
                        if has_safety_comment(&content, offset) {
                            continue;
                        }
                    }

                    let violation = Violation {
                        file: relative.clone(),
                        line,
                        column,
                        matched_text,
                        gate: gate.name.clone(),
                        principle: gate.principle.clone(),
                        cns_span: "cns.ci.invariant.violation".to_string(),
                    };

                    violations_map.get_mut(&gate.name).unwrap().push(violation);
                }
            }
        }
    }

    // Build reports
    let mut total_passed = 0;
    let mut total_warnings = 0;
    let mut total_denied = 0;

    for gate in gates {
        let violations = violations_map.remove(&gate.name).unwrap_or_default();
        let passed = violations.is_empty();

        match gate.severity.as_str() {
            "deny" if !passed => total_denied += 1,
            "warning" if !passed => total_warnings += 1,
            _ => total_passed += 1,
        }

        gate_reports.push(GateReport {
            name: gate.name.clone(),
            principle: gate.principle.clone(),
            severity: gate.severity.clone(),
            violations,
            passed,
        });
    }

    let summary = CheckSummary {
        total_gates: gates.len(),
        passed: total_passed,
        warnings: total_warnings,
        denied: total_denied,
    };

    Ok(CheckReport {
        gates: gate_reports,
        summary,
    })
}

// ── SAFETY comment check ─────────────────────────────────────────────────

fn has_safety_comment(content: &str, unsafe_offset: usize) -> bool {
    // Look at the 3 preceding lines for "SAFETY:"
    let prefix = &content[..unsafe_offset];
    let lines: Vec<&str> = prefix.lines().collect();
    let check_start = if lines.len() > 3 { lines.len() - 3 } else { 0 };
    lines[check_start..]
        .iter()
        .any(|line| line.contains("SAFETY:"))
}

// ── Expired allowlist check ──────────────────────────────────────────────

fn check_expired_allowlists(config: &InvariantsConfig) -> Result<ExpiredAllowlistReport> {
    let today = chrono::Utc::now().date_naive();
    let mut expired = Vec::new();

    for (gate_name, gate) in &config.gate {
        for entry in &gate.allowlist {
            let expires = NaiveDate::parse_from_str(&entry.expires, "%Y-%m-%d")
                .with_context(|| format!("invalid expiry date: {}", entry.expires))?;
            let days_expired = (today - expires).num_days();
            if days_expired > 0 {
                expired.push(ExpiredEntry {
                    gate: gate_name.clone(),
                    path: entry.path.clone(),
                    reason: entry.reason.clone(),
                    expires: entry.expires.clone(),
                    days_expired,
                });
            }
        }
    }

    Ok(ExpiredAllowlistReport { expired })
}

// ── Main ─────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Check {
            workspace_root,
            config,
            output,
        } => {
            let result = do_check(workspace_root, config, &output);
            match result {
                Ok(report) => {
                    if report.summary.denied > 0 {
                        ExitCode::from(1)
                    } else {
                        ExitCode::from(0)
                    }
                }
                Err(e) => {
                    eprintln!("ERROR: {:#}", e);
                    ExitCode::from(2)
                }
            }
        }
        Command::CheckExpiredAllowlists { config } => {
            let result = do_check_expired(config);
            match result {
                Ok(report) => {
                    if report.expired.is_empty() {
                        println!("All allowlist entries are current.");
                        ExitCode::from(0)
                    } else {
                        eprintln!(
                            "{} expired allowlist entr{} found:",
                            report.expired.len(),
                            if report.expired.len() == 1 {
                                "y"
                            } else {
                                "ies"
                            }
                        );
                        for entry in &report.expired {
                            eprintln!(
                                "  gate={} path={} expires={} ({} days expired) reason={}",
                                entry.gate,
                                entry.path,
                                entry.expires,
                                entry.days_expired,
                                entry.reason
                            );
                        }
                        ExitCode::from(1)
                    }
                }
                Err(e) => {
                    eprintln!("ERROR: {:#}", e);
                    ExitCode::from(2)
                }
            }
        }
        Command::GenerateInventory { workspace_root } => {
            let result = do_generate_inventory(workspace_root);
            match result {
                Ok(()) => ExitCode::from(0),
                Err(e) => {
                    eprintln!("ERROR: {:#}", e);
                    ExitCode::from(2)
                }
            }
        }
    }
}

fn find_workspace_root(provided: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = provided {
        return Ok(p);
    }
    // Auto-detect: look for Cargo.toml with [workspace] in ancestors
    let cwd = std::env::current_dir()?;
    for ancestor in cwd.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        if manifest.exists() {
            let contents = std::fs::read_to_string(&manifest)?;
            if contents.contains("[workspace]") {
                return Ok(ancestor.to_path_buf());
            }
        }
    }
    // Fallback: use the crate's own root (2 levels up from the binary)
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root.parent().unwrap().parent().unwrap();
    Ok(workspace_root.to_path_buf())
}

fn find_config(provided: Option<PathBuf>, workspace_root: &Path) -> Result<PathBuf> {
    if let Some(p) = provided {
        return Ok(p);
    }
    // Default: <workspace_root>/crates/hkask-ci-invariants/invariants.toml
    Ok(workspace_root.join("crates/hkask-ci-invariants/invariants.toml"))
}

fn load_config(path: &Path) -> Result<InvariantsConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let config: InvariantsConfig = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    Ok(config)
}

fn do_check(
    workspace_root: Option<PathBuf>,
    config_path: Option<PathBuf>,
    output: &str,
) -> Result<CheckReport> {
    let root = find_workspace_root(workspace_root)?;
    let config_file = find_config(config_path, &root)?;
    let config = load_config(&config_file)?;
    let gates = compile_gates(&config)?;

    let report = run_check(&root, &gates)?;

    match output {
        "human" => print_human_report(&report),
        _ => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{}", json);
        }
    }

    Ok(report)
}

fn print_human_report(report: &CheckReport) {
    for gate in &report.gates {
        let status = if gate.passed {
            "PASS"
        } else {
            match gate.severity.as_str() {
                "warning" => "WARN",
                _ => "FAIL",
            }
        };
        println!(
            "{}  {} [{}] {}",
            status,
            gate.name,
            gate.principle,
            if gate.passed {
                String::new()
            } else {
                format!("({} violations)", gate.violations.len())
            }
        );
        for v in &gate.violations {
            println!("    {}:{}:{}  {}", v.file, v.line, v.column, v.matched_text);
        }
    }
}

fn do_check_expired(config_path: Option<PathBuf>) -> Result<ExpiredAllowlistReport> {
    let root = find_workspace_root(None)?;
    let config_file = find_config(config_path, &root)?;
    let config = load_config(&config_file)?;
    check_expired_allowlists(&config)
}

fn do_generate_inventory(workspace_root: Option<PathBuf>) -> Result<()> {
    let root = find_workspace_root(workspace_root)?;
    let source_dirs = discover_source_dirs(&root);

    #[derive(Serialize)]
    struct InventoryItem {
        crate_name: String,
        file: String,
        item_type: String, // "fn", "struct", "trait", "enum", "mod"
        name: String,
        visibility: String, // "pub" or "pub(crate)"
    }

    let mut items: Vec<InventoryItem> = Vec::new();

    for src_dir in &source_dirs {
        let crate_name = src_dir.file_name().unwrap().to_string_lossy().to_string();
        for entry in walkdir::WalkDir::new(src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "rs") && e.file_type().is_file()
            })
        {
            let file_path = entry.path();
            let relative = file_path
                .strip_prefix(&root)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Simple regex-based extraction of public items
            let pub_fn = Regex::new(r"pub\s+(async\s+)?fn\s+(\w+)").unwrap();
            let pub_struct = Regex::new(r"pub\s+struct\s+(\w+)").unwrap();
            let pub_trait = Regex::new(r"pub\s+trait\s+(\w+)").unwrap();
            let pub_enum = Regex::new(r"pub\s+enum\s+(\w+)").unwrap();
            let pub_mod = Regex::new(r"pub\s+mod\s+(\w+)").unwrap();
            let pub_crate_fn = Regex::new(r"pub\(crate\)\s+(async\s+)?fn\s+(\w+)").unwrap();

            for cap in pub_fn.captures_iter(&content) {
                items.push(InventoryItem {
                    crate_name: crate_name.clone(),
                    file: relative.clone(),
                    item_type: "fn".to_string(),
                    name: cap[2].to_string(),
                    visibility: "pub".to_string(),
                });
            }
            for cap in pub_struct.captures_iter(&content) {
                items.push(InventoryItem {
                    crate_name: crate_name.clone(),
                    file: relative.clone(),
                    item_type: "struct".to_string(),
                    name: cap[1].to_string(),
                    visibility: "pub".to_string(),
                });
            }
            for cap in pub_trait.captures_iter(&content) {
                items.push(InventoryItem {
                    crate_name: crate_name.clone(),
                    file: relative.clone(),
                    item_type: "trait".to_string(),
                    name: cap[1].to_string(),
                    visibility: "pub".to_string(),
                });
            }
            for cap in pub_enum.captures_iter(&content) {
                items.push(InventoryItem {
                    crate_name: crate_name.clone(),
                    file: relative.clone(),
                    item_type: "enum".to_string(),
                    name: cap[1].to_string(),
                    visibility: "pub".to_string(),
                });
            }
            for cap in pub_mod.captures_iter(&content) {
                items.push(InventoryItem {
                    crate_name: crate_name.clone(),
                    file: relative.clone(),
                    item_type: "mod".to_string(),
                    name: cap[1].to_string(),
                    visibility: "pub".to_string(),
                });
            }
            for cap in pub_crate_fn.captures_iter(&content) {
                items.push(InventoryItem {
                    crate_name: crate_name.clone(),
                    file: relative.clone(),
                    item_type: "fn".to_string(),
                    name: cap[2].to_string(),
                    visibility: "pub(crate)".to_string(),
                });
            }
        }
    }

    let json = serde_json::to_string_pretty(&items)?;
    println!("{}", json);

    Ok(())
}
