//! Spec command handlers for `kask spec` — delegates to SpecService.
//!
//! Implements the CLI display logic for specification capture, validation,
//! cultivation, and rendering.

use crate::cli::SpecAction;
use hkask_agents::DefaultSpecCurator;
use hkask_services::{SpecCaptureRequest, SpecService};
use hkask_storage::SpecStore;
use hkask_storage::spec_types::{SpecCategory, SpecCurator};

fn build_service_context() -> hkask_services::AgentService {
    super::helpers::build_service_context()
}

pub fn run(action: SpecAction) {
    match action {
        SpecAction::Capture {
            name,
            category,
            domain,
            criteria,
        } => {
            let ctx = build_service_context();
            let resp = SpecService::capture(
                &ctx,
                SpecCaptureRequest {
                    name_or_description: name,
                    category: Some(category),
                    domain: Some(domain),
                    criteria: Some(criteria.unwrap_or_default()),
                    context: None,
                },
            )
            .expect("Failed to capture spec");

            println!("Specification captured:");
            println!("  ID: {}", resp.spec_id);
            println!("  Name: {}", resp.name);
            println!("  Category: {}", resp.category);
            println!("  Domain: {}", resp.domain_anchor);
            println!("  Complete: {}", resp.complete);
        }
        SpecAction::List { category } => {
            let ctx = build_service_context();
            match SpecService::list(&ctx, category.as_deref()) {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("No specifications found.");
                    } else {
                        println!("Specifications ({}):", entries.len());
                        for e in entries {
                            println!(
                                "  {} [{}] {} — complete: {}",
                                e.spec_id, e.category, e.name, e.complete
                            );
                        }
                    }
                }
                Err(e) => println!("Spec listing failed: {e}"),
            }
        }
        SpecAction::Evaluate { spec_id } => {
            println!("Evaluating specification: {}", spec_id);
            println!("  Note: Evaluation requires hkask-mcp-spec server.");
        }
        SpecAction::Validate { id } => {
            let spec_id = super::helpers::or_exit(
                hkask_storage::spec_types::SpecId::from_string(&id),
                "Invalid spec ID",
            );
            let ctx = build_service_context();
            let spec = ctx
                .spec_store()
                .load(spec_id)
                .map_err(hkask_services::ServiceError::Spec)
                .expect("Failed to load specification");
            let curator = DefaultSpecCurator::default();
            let record = curator
                .evaluate(&spec, &[])
                .expect("Failed to evaluate specification");

            println!("Specification validation:");
            println!("  ID: {}", record.spec_id);
            println!("  Decision: {:?}", record.decision);
            println!("  Rationale: {}", record.rationale);
            println!("  Coherence: {:.2}", record.coherence_score);
            println!("  Curated at: {}", record.curated_at);
        }
        SpecAction::Cultivate { id } => {
            let spec_id = super::helpers::or_exit(
                hkask_storage::spec_types::SpecId::from_string(&id),
                "Invalid spec ID",
            );
            let ctx = build_service_context();
            let spec = ctx
                .spec_store()
                .load(spec_id)
                .map_err(hkask_services::ServiceError::Spec)
                .expect("Failed to load specification");
            let curator = DefaultSpecCurator::default();
            let record = curator
                .evaluate(&spec, &[])
                .expect("Failed to cultivate specification");

            println!("Specification cultivation:");
            println!("  ID: {}", record.spec_id);
            println!("  Decision: {:?}", record.decision);
            println!("  Rationale: {}", record.rationale);
            println!("  Coherence: {:.2}", record.coherence_score);
            println!("  Spec completeness: {}", spec.is_complete());
            println!("  Spec coherence: {:.2}", spec.coherence());
            println!();
            println!("  Required categories for full collection coherence:");
            for cat in SpecCategory::all() {
                println!("    - {}", cat.as_str());
            }
        }
        SpecAction::Render { template, spec_id } => {
            use minijinja::UndefinedBehavior;

            let template_path = format!("registry/templates/{}", template);
            let template_content = super::helpers::or_exit(
                std::fs::read_to_string(&template_path),
                "Template not found",
            );

            let ctx = build_service_context();
            let store = &ctx.spec_store();

            let render_ctx = if let Some(sid) = spec_id {
                let parsed_id = super::helpers::or_exit(
                    hkask_storage::spec_types::SpecId::from_string(&sid),
                    "Invalid spec ID",
                );
                let spec = super::helpers::or_exit(store.load(parsed_id), "Failed to load spec");
                minijinja::context! {
                    spec_id => spec.id.to_string(),
                    goal_name => spec.name,
                    spec_category => spec.category.as_str(),
                    domain_anchor => spec.domain_anchor.as_str(),
                    goals => spec.goals.iter().map(|g| minijinja::context! {
                        text => g.text,
                        depth => g.depth,
                        criteria => g.criteria.iter().map(|c| minijinja::context! {
                            description => c.description,
                            satisfied => c.satisfied,
                        }).collect::<Vec<_>>(),
                    }).collect::<Vec<_>>(),
                }
            } else {
                minijinja::context! {}
            };

            let mut env = minijinja::Environment::new();
            env.set_undefined_behavior(UndefinedBehavior::Strict);
            let rendered = super::helpers::or_exit(
                env.render_str(&template_content, render_ctx),
                "Template render error",
            );
            println!("{}", rendered);
        }
        SpecAction::TestInvariant {
            spec_id,
            seam,
            invariant,
            category,
            cycle,
        } => {
            println!("Test invariant recorded:");
            println!("  Spec ID: {}", spec_id);
            println!("  Seam: {}", seam);
            println!("  Invariant: {}", invariant);
            println!("  Category: {}", category);
            if let Some(ref c) = cycle {
                println!("  Cycle: {}", c);
            }
            println!(
                "  Invariant ID: {}:{}:{}",
                spec_id,
                seam,
                category.to_lowercase()
            );
            println!("  Status: recorded");
            println!();
            println!("  Note: Persistent traceability requires hkask-mcp-spec server.");
        }
        SpecAction::TestVerify { seam, category } => {
            println!("Test coverage verification:");
            if let Some(ref s) = seam {
                println!("  Filtered by seam: {}", s);
            }
            if let Some(ref c) = category {
                println!("  Filtered by category: {}", c);
            }
            println!();
            println!("  Note: Full verification requires hkask-mcp-spec server.");
        }
    }
}
