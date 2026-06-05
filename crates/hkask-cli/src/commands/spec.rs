//! Spec command handlers for `kask spec`
//!
//! Implements the CLI display logic for specification capture, validation,
//! cultivation, and rendering.

use crate::cli::SpecAction;
use hkask_storage::SpecStore;

pub fn run(action: SpecAction) {
    match action {
        SpecAction::Capture {
            name,
            category,
            domain,
            criteria,
        } => {
            use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory};

            let cat = SpecCategory::parse_str(&category).unwrap_or(SpecCategory::Domain);
            let anchor = DomainAnchor::parse_str(&domain).unwrap_or(DomainAnchor::Hkask);
            let mut goal = GoalSpec::new(&name);
            if let Some(crits) = criteria {
                for c in crits.split(',') {
                    goal = goal.with_criterion(c.trim());
                }
            }
            let spec = Spec::new(&name, cat, anchor).with_goal(goal);
            let complete = spec.is_complete();

            let store = super::helpers::or_exit(
                crate::commands::config::open_spec_store(),
                "Failed to open spec store",
            );
            super::helpers::or_exit(store.save(&spec), "Failed to save specification");

            println!("Specification captured:");
            println!("  ID: {}", spec.id);
            println!("  Name: {}", spec.name);
            println!("  Category: {}", spec.category.as_str());
            println!("  Domain: {}", spec.domain_anchor.as_str());
            println!("  Complete: {}", complete);
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
            use hkask_agents::DefaultSpecCurator;
            use hkask_storage::spec_types::SpecCurator;

            let spec_id = super::helpers::or_exit(
                hkask_storage::spec_types::SpecId::from_string(&id),
                "Invalid spec ID",
            );
            let store = super::helpers::or_exit(
                crate::commands::config::open_spec_store(),
                "Failed to open spec store",
            );
            let spec = super::helpers::or_exit(store.load(spec_id), "Failed to load specification");
            let curator = DefaultSpecCurator::default();
            let record = super::helpers::or_exit(
                curator.evaluate(&spec, &[]),
                "Failed to evaluate specification",
            );

            println!("Specification validation:");
            println!("  ID: {}", record.spec_id);
            println!("  Decision: {:?}", record.decision);
            println!("  Rationale: {}", record.rationale);
            println!("  Coherence: {:.2}", record.coherence_score);
            println!("  Curated at: {}", record.curated_at);
        }
        SpecAction::Cultivate { id } => {
            use hkask_agents::DefaultSpecCurator;
            use hkask_storage::spec_types::{SpecCategory, SpecCurator};

            let spec_id = super::helpers::or_exit(
                hkask_storage::spec_types::SpecId::from_string(&id),
                "Invalid spec ID",
            );
            let store = super::helpers::or_exit(
                crate::commands::config::open_spec_store(),
                "Failed to open spec store",
            );
            let spec = super::helpers::or_exit(store.load(spec_id), "Failed to load specification");
            let curator = DefaultSpecCurator::default();
            let record = super::helpers::or_exit(
                curator.evaluate(&spec, &[]),
                "Failed to cultivate specification",
            );

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

            let store = super::helpers::or_exit(
                crate::commands::config::open_spec_store(),
                "Failed to open spec store",
            );

            let ctx = if let Some(sid) = spec_id {
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
                env.render_str(&template_content, ctx),
                "Template render error",
            );
            println!("{}", rendered);
        }
    }
}
