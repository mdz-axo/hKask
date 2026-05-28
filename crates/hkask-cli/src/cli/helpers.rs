//! CLI helper utilities

use hkask_types::TemplateType as Type;

/// Parse a string into a DataCategory
pub fn parse_data_category(s: &str) -> hkask_types::DataCategory {
    match s {
        "episodic_memory" => hkask_types::DataCategory::EpisodicMemory,
        "semantic_memory" => hkask_types::DataCategory::SemanticMemory,
        "personal_context" => hkask_types::DataCategory::PersonalContext,
        "capability_tokens" => hkask_types::DataCategory::CapabilityTokens,
        "ocap_boundaries" => hkask_types::DataCategory::OcapBoundaries,
        "template_invocations" => hkask_types::DataCategory::TemplateInvocations,
        "hlexicon_terms" => hkask_types::DataCategory::HLexiconTerms,
        "template_registry" => hkask_types::DataCategory::TemplateRegistry,
        _ => hkask_types::DataCategory::Custom(s.to_string()),
    }
}

/// Parse a template type string into a TemplateType enum
pub fn parse_template_type(type_str: &str) -> Option<Type> {
    match type_str.to_lowercase().as_str() {
        "prompt" => Some(Type::Prompt),
        "cognition" => Some(Type::Cognition),
        "process" => Some(Type::Process),
        _ => None,
    }
}

/// Initialize tracing subscriber with optional verbose logging
pub fn init_logging(verbose: bool) {
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env()
    };
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
