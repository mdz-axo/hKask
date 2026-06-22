//! Style composition command — generate prose with exemplar retrieval and centroid validation
//!
//! Delegates to `ComposeService::compose()` which encapsulates the full
//! style synthesizer pipeline (DB → SemanticMemory → KNN →
//! Jinja2 system prompt → inference → centroid validation).
//!
//! Cognition: registry/styles/*/...-synthesizer.yaml and registry/styles/*/...-mashup.yaml

use hkask_inference::InferenceConfig;
use hkask_services::{CognitionConfig, ComposeRequest, ComposeService, InferenceContext};

use std::path::PathBuf;

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; prompt is non-empty; cognition is a valid YAML config path; db is a valid DB path; passphrase is the DB passphrase
/// post: composes prose using the style synthesizer pipeline; prints generated prose and validation results; exits on config/embedding failure
pub fn run(
    rt: &tokio::runtime::Runtime,
    prompt: String,
    cognition: PathBuf,
    db: PathBuf,
    passphrase: String,
    no_validate: bool,
) {
    let config_str = std::fs::read_to_string(&cognition).unwrap_or_else(|e| {
        eprintln!(
            "Failed to read cognition config {}: {}",
            cognition.display(),
            e
        );
        std::process::exit(1);
    });
    let config: CognitionConfig = serde_yaml_neo::from_str(&config_str).unwrap_or_else(|e| {
        eprintln!("Failed to parse cognition config YAML: {}", e);
        std::process::exit(1);
    });
    eprintln!(
        "Compose: model={}, dim={}, centroid={}",
        config.embedding.model, config.embedding.dim, config.embedding.centroid_entity_ref
    );
    let centroid_ref = config.embedding.centroid_entity_ref.clone();

    let inference_ctx =
        InferenceContext::from_parts(None, &config.embedding.model, InferenceConfig::from_env());

    let request = ComposeRequest {
        prompt,
        db_path: db,
        db_passphrase: passphrase,
        cognition: config,
        inference_ctx,
        no_validate,
    };

    let result = match rt.block_on(ComposeService::compose(request)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Compose failed: {}", e);
            std::process::exit(1);
        }
    };

    eprintln!("\n{}", result.generated_prose);

    // Publish the composed style as a public artifact for Curator indexing.
    // Other agents can discover and reuse this style through the artifact index.
    let agent_name = std::env::var("HKASK_REPLICANT").unwrap_or_else(|_| "anonymous".to_string());
    let _ = hkask_types::agent_paths::publish_artifact(&agent_name, "style", &centroid_ref, "");

    if let Some(ref validation) = result.validation {
        eprintln!("\nValidating style centroid distance...");
        eprintln!(
            "Centroid distance: {:.4} (threshold: {:.4})",
            validation.distance, validation.threshold
        );
        if validation.passed {
            eprintln!("✓ Style validation PASSED — prose is within style cluster");
        } else {
            eprintln!(
                "✗ Style validation FAILED — prose exceeds style cluster boundary ({:.4} > {:.4})",
                validation.distance, validation.threshold
            );
            eprintln!("Consider regenerating with stricter adherence to syntactic constraints.");
        }
    }

    if result.exemplar_count == 0 {
        eprintln!(
            "Warning: No exemplar passages found. \
             The style corpus may not be embedded yet. Run `kask embed-corpus` first."
        );
    } else if result.exemplar_count < 3 {
        eprintln!(
            "Warning: Only {} exemplar passages found. \
             Consider widening the distance threshold or embedding more corpus texts.",
            result.exemplar_count
        );
    }

    eprintln!("\nDone.");
}
