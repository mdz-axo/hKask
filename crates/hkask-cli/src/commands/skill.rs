//! Skill CLI command — corpus health audit.
//!
//! The CLI exposes only the structural audit (the CI gate). Discovery, status,
//! publishing, and derivation operate on the live registry and remain in the
//! REPL (`/skill`) or MCP tools. Mirrors `hkask_repl::handlers::skill::skill_audit`
//! but adds `--fail-below` enforcement for CI.

use crate::cli::SkillAction;
use hkask_ports::{RegistryIndex, SkillRegistryIndex};
use hkask_services_skill::SkillAuditor;
use hkask_templates::SqliteRegistry;
use std::path::PathBuf;

/// Run the `kask skill` CLI command.
pub fn run(action: SkillAction) {
    match action {
        SkillAction::Audit { fail_below, json } => run_audit(fail_below, json),
    }
}

fn run_audit(fail_below: f64, json: bool) {
    let root = project_root();

    // Fresh in-memory registry — same approach as the REPL handler. The audit
    // reads manifests from disk; the registry is only needed for index lookups.
    let registry = match SqliteRegistry::new(None) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize registry: {e}");
            std::process::exit(1);
        }
    };

    let registry_ref: &dyn RegistryIndex = &registry;
    let skill_index_ref: &dyn SkillRegistryIndex = &registry;
    let auditor = SkillAuditor::new(registry_ref, skill_index_ref, &root);

    let report = match auditor.audit_all() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Audit failed: {e}");
            std::process::exit(1);
        }
    };

    if json {
        match report.to_json() {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("JSON serialize failed: {e}");
                std::process::exit(1);
            }
        }
    } else {
        println!("Skill Audit Report");
        println!("Active skills: {}", report.active_count());
        println!("Non-skill crates: {}", report.non_skill_count());
        println!();
        for entry in &report.entries {
            let icon = if entry.is_active() { "✓" } else { "△" };
            println!(
                "  {icon} {} — score: {:.2}",
                entry.skill_name, entry.health_score
            );
            for defect in &entry.defects {
                println!("    → {defect}");
            }
        }
        if report.entries.is_empty() {
            println!("(no skills found)");
        }
        println!();
    }

    // CI gate: any skill below the threshold fails the audit.
    let failing: Vec<_> = report
        .entries
        .iter()
        .filter(|e| e.category == "skill" && e.health_score < fail_below)
        .collect();
    if !failing.is_empty() {
        eprintln!(
            "⚠ {} skill(s) below threshold {:.2}:",
            failing.len(),
            fail_below
        );
        for f in &failing {
            eprintln!("  ✗ {} — score: {:.2}", f.skill_name, f.health_score);
        }
        std::process::exit(1);
    }
}

fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
