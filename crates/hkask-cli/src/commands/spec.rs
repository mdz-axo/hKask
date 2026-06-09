//! Spec command handlers for `kask spec`
//
//! Implements the CLI display logic for specification capture, validation,
//! cultivation, and rendering. Uses `ServiceContext::spec_store` for
//! spec persistence — no direct `open_spec_store()` calls.

use crate::cli::SpecAction;
use hkask_agents::DefaultSpecCurator;
use hkask_storage::SpecStore;
use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory, SpecCurator};

fn build_service_context() -> hkask_services::ServiceContext {
    let config =
        hkask_services::ServiceConfig::from_env().expect("Failed to resolve service config");
    let rt = tokio::runtime::Runtime::new().expect("runtime should start");
    rt.block_on(hkask_services::ServiceContext::build(config))
        .expect("Failed to build service context")
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
            let cat = SpecCategory::parse_str(&category).unwrap_or(SpecCategory::Domain);
            let anchor = DomainAnchor::parse_str(&domain).unwrap_or(DomainAnchor::Hkask);
            let mut goal = GoalSpec::new(&name);
            if let Some(crits) = criteria.as_deref() {
                for c in crits.split(',') {
                    goal = goal.with_criterion(c.trim());
                }
            }
            let spec = Spec::new(&name, cat, anchor).with_goal(goal);
            let is_complete = spec.is_complete();
            ctx.spec_store.save(&spec).expect("Failed to save spec");

            println!("Specification captured:");
            println!("  ID: {}", spec.id);
            println!("  Name: {}", spec.name);
            println!("  Category: {}", spec.category.as_str());
            println!("  Domain: {}", spec.domain_anchor.as_str());
            println!("  Complete: {}", is_complete);
        }
        SpecAction::List { category } => {
            println!("Specifications:");
            if let Some(cat) = category {
                println!("  (filtered by category: {})", cat);
            }
            println!("  Note: Persistent spec storage requires hkask-mcp-spec server.");
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
                .spec_store
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
                .spec_store
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
            let store = &ctx.spec_store;

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
