//! Bundle command handlers for `kask bundle`
//!
//! # REQ: P11 (Digital Public/Private Sphere) — CLI surface for bundle management
//!
//! Delegates to `BundleService` for all business logic. Formatting and
//! terminal output are CLI-only concerns; composition, listing, and evolution
//! are performed by the shared service layer.

use std::sync::Arc;

use hkask_services::{BundleService, ServiceError};
use hkask_types::Visibility;
use hkask_types::ports::InferencePort;

use crate::block_on;
use crate::cli::BundleAction;
use crate::commands;

/// Resolve an inference port for bundle composition operations.
///
/// Bundle composition requires LLM inference to classify skill polarities,
/// detect conflicts, and determine cascade order. This creates a fresh
/// inference port for standalone CLI commands — the REPL reuses its shared
/// port, but standalone commands (`kask bundle compose`) create one on demand.
fn resolve_composition_port(rt: &tokio::runtime::Runtime) -> Arc<dyn InferencePort> {
    let okapi_base_url = std::env::var("OKAPI_BASE_URL")
        .unwrap_or_else(|_| hkask_services::DEFAULT_OKAPI_BASE_URL.to_string());
    let ctx =
        hkask_services::InferenceContext::from_parts(None, "deepseek-v4-pro", &okapi_base_url);
    commands::helpers::or_exit(
        hkask_services::InferenceService::resolve_port(&ctx, "deepseek-v4-pro"),
        "Failed to initialize inference port for bundle composition",
    )
}

/// Resolve the replicant name for editor attribution.
fn resolve_editor() -> String {
    hkask_services::resolve_replicant_name()
}

pub fn run_bundle(rt: &tokio::runtime::Runtime, action: BundleAction) {
    // Build the shared service context.
    let ctx = commands::helpers::build_service_context();

    match action {
        BundleAction::Compose {
            skills,
            name,
            visibility,
        } => {
            let vis = Visibility::parse_str(&visibility).unwrap_or(Visibility::Private);
            let inference_port = resolve_composition_port(rt);
            let editor = resolve_editor();
            let result = block_on!(
                rt,
                BundleService::compose(
                    &ctx,
                    &skills,
                    name.as_deref(),
                    vis,
                    inference_port,
                    &editor,
                ),
                "Bundle composition failed"
            );

            let manifest = &result.manifest;
            println!();
            println!("  \x1b[1mComposed Bundle: {}\x1b[0m", manifest.name);
            println!("  ID:          {}", manifest.id);
            println!("  Version:     {}", manifest.version);
            println!("  Visibility:  {}", manifest.visibility.as_str());
            println!("  Skills ({}) :", manifest.skills.len());
            for skill in &manifest.skills {
                println!(
                    "    - {} ({}) [{}]",
                    skill.id,
                    skill.polarity.as_str(),
                    skill.content_hash.chars().take(12).collect::<String>()
                );
            }
            if !manifest.conflicts.is_empty() {
                println!("  Conflicts ({}):", manifest.conflicts.len());
                for conflict in &manifest.conflicts {
                    println!(
                        "    - {} ↔ {}: {}",
                        conflict.skills.join(", "),
                        conflict.conflict_type.as_str(),
                        conflict.resolution.as_str()
                    );
                }
            }
            if !manifest.complementarities.is_empty() {
                println!(
                    "  Complementarities ({}):",
                    manifest.complementarities.len()
                );
                for comp in &manifest.complementarities {
                    println!(
                        "    - {} → {}: {}",
                        comp.skills.join(", "),
                        comp.complementarity_type.as_str(),
                        comp.detail
                    );
                }
            }
            println!("  Steps ({}):", manifest.steps.len());
            for step in &manifest.steps {
                println!(
                    "    {}. {} [{}] — gas: {}, timeout: {}s",
                    step.ordinal,
                    step.description,
                    step.phase.as_str(),
                    step.gas_cap,
                    step.timeout_seconds
                );
            }
            if !manifest.convergence.threshold.is_nan() {
                println!(
                    "  Convergence: threshold={}, max_iterations={}",
                    manifest.convergence.threshold, manifest.convergence.max_iterations
                );
            }
            if !result.warnings.is_empty() {
                println!("  \x1b[33mWarnings:\x1b[0m");
                for warning in &result.warnings {
                    println!("    \x1b[33m⚠ {}\x1b[0m", warning);
                }
            }
            println!();
            println!(
                "  To apply this bundle: \x1b[36mkask bundle apply {}\x1b[0m",
                manifest.id
            );
            println!();
        }

        BundleAction::Apply { bundle_id } => {
            let manifest = block_on!(
                rt,
                BundleService::apply(&ctx, &bundle_id),
                "Bundle apply failed"
            );
            println!("  \x1b[1mApplied Bundle: {}\x1b[0m", manifest.name);
            println!("  {} skills active", manifest.skills.len());
            println!();
        }

        BundleAction::List => {
            let bundles = block_on!(rt, BundleService::list(&ctx), "Bundle list failed");
            if bundles.is_empty() {
                println!("  No bundles registered.");
            } else {
                println!("  \x1b[1mSkill Bundles ({})\x1b[0m", bundles.len());
                for b in &bundles {
                    println!();
                    println!(
                        "  \x1b[1m{}\x1b[0m  id={}  v={}  visibility={}",
                        b.name,
                        b.id,
                        b.version,
                        b.visibility.as_str()
                    );
                    println!("    {}", b.description);
                    for skill in &b.skills {
                        println!("    - [{}] {}", skill.polarity.as_str(), skill.id);
                    }
                }
                println!();
            }
        }

        BundleAction::Show { bundle_id } => {
            match block_on!(
                rt,
                BundleService::get(&ctx, &bundle_id),
                "Bundle show failed"
            ) {
                Some(manifest) => {
                    let json = serde_json::to_string_pretty(&manifest).unwrap_or_default();
                    println!("{}", json);
                }
                None => {
                    eprintln!("Bundle '{}' not found.", bundle_id);
                    std::process::exit(1);
                }
            }
        }

        BundleAction::Evolve { bundle_id } => {
            let inference_port = resolve_composition_port(rt);
            let editor = resolve_editor();
            let result = block_on!(
                rt,
                BundleService::evolve(&ctx, &bundle_id, inference_port, &editor),
                "Bundle evolution failed"
            );
            println!(
                "  \x1b[1mEvolved Bundle: {}\x1b[0m ({} skills, {} steps)",
                result.manifest.name,
                result.manifest.skills.len(),
                result.manifest.steps.len()
            );
            if !result.warnings.is_empty() {
                println!("  \x1b[33mWarnings:\x1b[0m");
                for warning in &result.warnings {
                    println!("    \x1b[33m⚠ {}\x1b[0m", warning);
                }
            }
            println!();
        }

        BundleAction::Skills => {
            let skills = block_on!(rt, BundleService::list_skills(&ctx), "Skill list failed");
            if skills.is_empty() {
                println!(
                    "  No skills loaded. Skills are loaded from .agents/skills/ at REPL startup."
                );
            } else {
                println!("  \x1b[1mAvailable Skills ({})\x1b[0m", skills.len());
                for s in &skills {
                    let polarity = s.polarity.map(|p| p.as_str()).unwrap_or("-");
                    let hash = s
                        .content_hash
                        .as_deref()
                        .map(|h| &h[..h.len().min(12)])
                        .unwrap_or("-");
                    println!(
                        "  {:30}  domain={:10}  polarity={:12}  visibility={:8}  zone={}  hash={}",
                        s.id,
                        s.domain.as_str(),
                        polarity,
                        s.visibility.as_str(),
                        s.zone.as_str(),
                        hash,
                    );
                }
                println!();
            }
        }

        BundleAction::Off => {
            // Deactivation is a no-op — bundles are session-scoped.
            // When not in a REPL session, there is no active bundle to deactivate.
            println!("  Bundle deactivated.");
            println!();
        }
    }
}
