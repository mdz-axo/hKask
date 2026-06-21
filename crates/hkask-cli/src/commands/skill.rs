//! Skill command handlers for `kask skill`
//!
//! Implements CLI display logic for skill visibility management.
//! Two-zone model: `.agents/skills/` (source) → `skills/` (export surface).

use crate::cli::SkillAction;
use hkask_ports::SkillZone;
use hkask_services::skill;
use hkask_services::skills::{SkillAuditor, SkillStatus};
use hkask_templates::{Registry, SkillLoader};
use hkask_types::visibility::Visibility;

use std::path::PathBuf;

/// Default project root (current directory).
fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Run a skill command.
///
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  action is valid
/// post: skill command executed
pub fn run_skill(action: SkillAction) {
    match action {
        SkillAction::List { visibility } => {
            list_skills(visibility.as_deref());
        }
        SkillAction::Status { name } => {
            skill_status(&name);
        }
        SkillAction::Publish { name } => {
            skill_publish(&name);
        }
        SkillAction::Audit { fail_below, json } => {
            skill_audit(fail_below, json);
        }
    }
}

/// List skills, optionally filtered by visibility.
fn list_skills(visibility_filter: Option<&str>) {
    let root = project_root();
    let vis_filter = visibility_filter.and_then(Visibility::parse_str);

    for zone in [SkillZone::Private, SkillZone::Public] {
        let zone_dir = root.join(zone.directory());
        if !zone_dir.exists() {
            continue;
        }

        let skill_infos = match skill::discover_skills(&zone_dir) {
            Ok(dirs) => dirs,
            Err(e) => {
                eprintln!("Error scanning {}: {}", zone_dir.display(), e);
                continue;
            }
        };

        if skill_infos.is_empty() {
            continue;
        }

        println!("  {} zone ({}):", zone.as_str(), zone_dir.display());

        for info in &skill_infos {
            // Apply filter
            if let Some(filter) = vis_filter
                && info.visibility != filter
            {
                continue;
            }

            let hash_display = info
                .content_hash
                .as_deref()
                .map(|h| &h[..h.len().min(12)])
                .unwrap_or("-");
            let ns_display = info.namespace.as_deref().unwrap_or("-");

            println!(
                "    {:30} visibility={:8} namespace={:12} hash={}",
                info.name,
                info.visibility.as_str(),
                ns_display,
                hash_display
            );
        }
    }
}

/// Show skill status — compare private source vs published copy.
fn skill_status(name: &str) {
    let root = project_root();
    let private_dir = root.join(SkillZone::Private.directory()).join(name);

    let public_dir = skill::find_public_skill(&root, name);

    if !private_dir.exists() {
        eprintln!("Skill '{}' not found in private zone.", name);
        return;
    }

    let private_vis = skill::read_skill_visibility(&private_dir.join("SKILL.md"));
    let private_hash = skill::compute_file_hash(&private_dir.join("SKILL.md"));
    let private_ns = skill::read_skill_namespace(&private_dir.join("SKILL.md"));

    println!("Skill: {}", name);
    println!("  Private zone: {}", private_dir.display());
    println!("  Visibility:   {}", private_vis.as_str());
    if let Some(ref ns) = private_ns {
        println!("  Namespace:    {}", ns);
    }
    println!(
        "  Source hash:  {}",
        private_hash.as_deref().unwrap_or("(error)")
    );

    if let Some(ref pub_dir) = public_dir {
        let public_hash = skill::compute_file_hash(&pub_dir.join("SKILL.md"));
        let pub_namespace = skill::read_skill_namespace(&pub_dir.join("SKILL.md"));
        println!("  Public zone:  {}", pub_dir.display());
        if let Some(ref ns) = pub_namespace {
            println!("  Published by: {}", ns);
        }
        println!(
            "  Public hash:  {}",
            public_hash.as_deref().unwrap_or("(error)")
        );

        match (private_hash, public_hash) {
            (Some(ph), Some(pubh)) if ph == pubh => {
                println!("  Status:       in sync");
            }
            (Some(_), Some(_)) => {
                println!(
                    "  Status:       local changes since last publish — run `kask skill publish {}` to update",
                    name
                );
            }
            _ => {
                println!("  Status:       unable to compare hashes");
            }
        }
    } else {
        println!("  Public zone:  (not published)");
        if private_vis == Visibility::Public {
            println!(
                "  Status:       public but not yet exported — run `kask skill publish {}`",
                name
            );
        } else {
            println!("  Status:       private (not exported)");
        }
    }
}

/// Publish a skill from the private zone to the public zone.
fn skill_publish(name: &str) {
    let root = project_root();

    match skill::publish_skill(&root, name) {
        Ok(result) => {
            println!(
                "Published '{}' as '{}' to public zone: {}",
                result.name,
                result.namespaced_name,
                result.public_dir.display()
            );
            println!("  Sortable by replicant: {}", result.namespace);
            println!("  Sortable by skill:    {}", result.name);
        }
        Err(e) => {
            eprintln!("Publish failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Run the dual-layer skill audit and emit a report.
///
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  fail_below is in [0.0, 1.0]
/// post: JSON or table report printed; process exits 1 if any score < fail_below
fn skill_audit(fail_below: f64, json: bool) {
    let root = project_root();
    let mut registry = Registry::new();
    let loader = SkillLoader::new(&root);
    let _load_result = loader.load_into(&mut registry);

    let auditor = SkillAuditor::new(&registry, &registry, &root);

    let report = match auditor.audit_all() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Audit failed: {e}");
            std::process::exit(1);
        }
    };

    let threshold = fail_below.clamp(0.0, 1.0);
    let mut failed = false;

    if json {
        match report.to_json() {
            Ok(output) => println!("{output}"),
            Err(e) => {
                eprintln!("Failed to serialize audit report: {e}");
                std::process::exit(1);
            }
        }
    } else {
        println!("Skill audit report (fail_below={threshold:.2})");
        println!();
        println!(
            "{:<30} {:>6} {:>14} {:>8} defects",
            "skill", "score", "status", "active"
        );
        for entry in &report.entries {
            let active = if entry.is_active() { "yes" } else { "no" };
            let status_label = status_label(entry.status);
            println!(
                "{:<30} {:>6.2} {:>14} {:>8} {}",
                entry.skill_name,
                entry.health_score,
                status_label,
                active,
                entry.defects.len()
            );
            if !entry.defects.is_empty() {
                for defect in &entry.defects {
                    println!("    - {defect}");
                }
            }
            if entry.health_score < threshold {
                failed = true;
            }
        }
        println!();
        println!("Active: {}/{}", report.active_count(), report.entries.len(),);
    }

    if failed {
        eprintln!("Audit failed: one or more skills are below threshold.");
        std::process::exit(1);
    }
}

fn status_label(status: SkillStatus) -> &'static str {
    match status {
        SkillStatus::Active => "active",
        SkillStatus::StaleWarning => "stale",
        SkillStatus::Critical => "critical",
        SkillStatus::RecommendDeprecation => "deprecate",
    }
}
