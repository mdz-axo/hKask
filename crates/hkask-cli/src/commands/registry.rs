//! Registry command handlers for `kask registry`
//!
//! Implements the CLI display logic for registry import and management.

use crate::cli::RegistryAction;
use hkask_templates::SqliteRegistry;

pub fn run(_rt: &tokio::runtime::Runtime, registry: &mut SqliteRegistry, action: RegistryAction) {
    use crate::commands::russell::RussellMappingConfig;

    match action {
        RegistryAction::ImportRussell {
            source,
            dry_run,
            validate_only,
            output_format,
            transform_rules,
            verbose,
        } => {
            let mut config = if let Some(rules_path) = &transform_rules {
                match RussellMappingConfig::load_from_yaml(rules_path.to_str().unwrap_or("")) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load transform rules from {}: {}. Using defaults.",
                            rules_path.display(),
                            e
                        );
                        RussellMappingConfig::defaults()
                    }
                }
            } else {
                let default_path = "registry/manifests/russell-mapping.yaml";
                match RussellMappingConfig::load_from_yaml(default_path) {
                    Ok(c) => c,
                    Err(_) => RussellMappingConfig::defaults(),
                }
            };

            config.dry_run = dry_run;

            let mapper = crate::commands::russell::RussellMapper::with_config(config.clone());

            if validate_only {
                let assets = super::helpers::or_exit(
                    crate::commands::import_russell(&source, &config, verbose),
                    "Validation failed",
                );
                println!("Validation complete: {} manifests parsed", assets.len());
                for asset in &assets {
                    println!("\n  ID: {} [VALID]", asset.id);
                }
            } else {
                let assets = super::helpers::or_exit(
                    crate::commands::import_russell_with_mapper(&mapper, &source, verbose),
                    "Migration failed",
                );
                let fmt = output_format.to_lowercase();
                match fmt.as_str() {
                    "json" => {
                        let json = serde_json::to_string_pretty(&assets)
                            .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                        println!("{}", json);
                    }
                    "mermaid" => {
                        println!("graph LR");
                        for asset in &assets {
                            println!("  russell[\"{}\"] --> hkask[\"{}\"]", asset.id, asset.id);
                        }
                    }
                    _ => {
                        println!("Migration analysis complete: {} assets", assets.len());
                        for asset in &assets {
                            println!("\n  ID: {}", asset.id);
                            println!("  Type: {:?}", asset.template_type);
                            println!("  Description: {}", asset.description);
                            println!("  Model Tier: {}", asset.model_tier);
                            println!("  Gas Cap: {}", asset.gas_cap);
                        }
                    }
                }
                if !dry_run {
                    for asset in &assets {
                        let entry = hkask_templates::RegistryEntry {
                            id: asset.id.clone(),
                            template_type: asset.template_type,
                            name: asset.id.clone(),
                            lexicon_terms: vec!["russell-migrated".to_string()],
                            description: asset.description.clone(),
                            source_path: format!("russell-migrated:{}", asset.id),
                            required_capabilities: vec![],
                            cascade_level: 0,
                            matroshka_limit: hkask_types::SYSTEM_MAX_RECURSION as u32,
                        };
                        if let Err(e) = registry.register(entry) {
                            eprintln!("Failed to register template {}: {}", asset.id, e);
                        } else if verbose {
                            println!("  Registered: {}", asset.id);
                        }
                    }
                }
            }
        }
        RegistryAction::ListMigrated { origin: _ } => {
            println!("Migrated assets:");
            println!("  (use 'kask registry import-russell --dry-run' to analyze assets)");
        }
    }
}
