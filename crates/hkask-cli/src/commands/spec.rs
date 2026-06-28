//! Spec command handlers for `kask spec` — thin passthrough to SpecStore.
//!
//! Capture and listing delegate directly to the spec store. Validation is
//! handled by `DefaultSpecCurator`, with QA providing `kask qa spec-check`.

use crate::cli::SpecAction;
use hkask_storage::SpecStore;
use hkask_storage::spec_types::{DomainAnchor, GoalSpec, Spec, SpecCategory};

/// Run a spec command.
pub fn run(action: SpecAction) {
    match action {
        SpecAction::Capture {
            name,
            category,
            domain,
            criteria,
        } => {
            let ctx = super::helpers::build_service_context();
            let store = ctx.spec_store();
            let cat = SpecCategory::parse_str(&category).unwrap_or(SpecCategory::Domain);
            let anchor = DomainAnchor::parse_str(&domain).unwrap_or(DomainAnchor::Hkask);

            let mut goal = GoalSpec::new(&name);
            if let Some(crits) = criteria.as_deref() {
                for c in crits.split(',') {
                    let trimmed = c.trim();
                    if !trimmed.is_empty() {
                        goal = goal.with_criterion(trimmed);
                    }
                }
            }

            let spec = Spec::new(&name, cat, anchor).with_goal(goal);
            let complete = spec.is_complete();
            match store.save(&spec) {
                Ok(()) => {
                    println!("Specification captured:");
                    println!("  ID: {}", spec.id);
                    println!("  Name: {}", spec.name);
                    println!("  Category: {}", spec.category.as_str());
                    println!("  Domain: {}", spec.domain_anchor.as_str());
                    println!("  Complete: {}", complete);
                }
                Err(e) => {
                    eprintln!("Failed to capture spec: {e}");
                    std::process::exit(1);
                }
            }
        }
        SpecAction::List { category } => {
            let ctx = super::helpers::build_service_context();
            let store = ctx.spec_store();
            let result = match category.as_deref() {
                Some(cat_str) => {
                    let cat = SpecCategory::parse_str(cat_str).unwrap_or(SpecCategory::Domain);
                    store.list_by_category(cat)
                }
                None => store.list_all(),
            };
            match result {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("No specifications found.");
                    } else {
                        println!("Specifications ({}):", entries.len());
                        for s in entries {
                            println!(
                                "  {} [{}] {} — complete: {}",
                                s.id,
                                s.category.as_str(),
                                s.name,
                                s.is_complete()
                            );
                        }
                    }
                }
                Err(e) => println!("Spec listing failed: {e}"),
            }
        }
        SpecAction::Evaluate { spec_id } => {
            run_validate(&spec_id);
        }
        SpecAction::Validate { spec_id } => {
            run_validate(&spec_id);
        }
        SpecAction::Cultivate { spec_id } => {
            run_cultivate(&spec_id);
        }
        SpecAction::Render { template, spec_id } => {
            use minijinja::UndefinedBehavior;

            let template_path = format!("registry/templates/{}", template);
            let template_content = super::helpers::or_exit(
                std::fs::read_to_string(&template_path),
                "Template not found",
            );

            let ctx = super::helpers::build_service_context();

            let render_ctx = if let Some(sid) = spec_id {
                let store = ctx.spec_store();
                let id = parse_spec_id_or_exit(&sid);
                let spec = super::helpers::or_exit(store.load(id), "Failed to load spec");
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
    }
}

fn run_validate(spec_id: &str) {
    use hkask_agents::DefaultSpecCurator;
    use hkask_storage::spec_types::SpecCurator;

    let ctx = super::helpers::build_service_context();
    let store = ctx.spec_store();
    let id = parse_spec_id_or_exit(spec_id);
    let spec = super::helpers::or_exit(store.load(id), "Failed to load spec");
    let curator = DefaultSpecCurator::default();
    let record = super::helpers::or_exit(
        curator.evaluate(&spec, &[]),
        "Failed to evaluate specification",
    );

    println!("Specification evaluation:");
    println!("  ID: {}", record.spec_id);
    println!("  Decision: {:?}", record.decision);
    println!("  Rationale: {}", record.rationale);
    println!("  Coherence: {:.2}", record.coherence_score);
    println!("  Curated at: {}", record.curated_at);
}

fn run_cultivate(spec_id: &str) {
    use hkask_agents::DefaultSpecCurator;
    use hkask_storage::spec_types::SpecCurator;

    let ctx = super::helpers::build_service_context();
    let store = ctx.spec_store();
    let id = parse_spec_id_or_exit(spec_id);
    let spec = super::helpers::or_exit(store.load(id), "Failed to load spec");
    let curator = DefaultSpecCurator::default();
    let record = super::helpers::or_exit(
        curator.evaluate(&spec, &[]),
        "Failed to validate specification",
    );

    println!("Specification cultivation:");
    println!("  ID: {}", record.spec_id);
    println!("  Decision: {:?}", record.decision);
    println!("  Rationale: {}", record.rationale);
    println!("  Coherence: {:.2}", record.coherence_score);
    println!("  Spec name: {}", spec.name);
    println!();
    println!("  Required categories for full collection coherence:");
    for cat in SpecCategory::all() {
        println!("    - {}", cat.as_str());
    }
}

fn parse_spec_id_or_exit(s: &str) -> hkask_storage::spec_types::SpecId {
    uuid::Uuid::parse_str(s)
        .map(hkask_storage::spec_types::SpecId)
        .unwrap_or_else(|_| {
            eprintln!("Invalid spec ID: {}", s);
            std::process::exit(1);
        })
}
