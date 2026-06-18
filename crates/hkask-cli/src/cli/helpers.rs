//! CLI helper utilities

use hkask_types::template_type::TemplateType as Type;

/// Parse a string into a DataCategory (delegates to DataCategory::parse)
///
/// expect: "I can access all hKask functionality through the kask CLI" [P3]
/// pre:  s is any string
/// post: returns DataCategory parsed from s (defaults to Public on unrecognized)
pub fn parse_data_category(s: &str) -> hkask_types::sovereignty::DataCategory {
    hkask_types::sovereignty::DataCategory::parse(s)
}

/// Parse a template type string into a TemplateType enum
///
/// expect: "I can access all hKask functionality through the kask CLI" [P3]
/// pre:  type_str is any string
/// post: returns Some(Type) if type_str matches a known template type
/// post: returns None if type_str is unrecognized
pub fn parse_template_type(type_str: &str) -> Option<Type> {
    Type::parse_str(type_str)
}

/// Initialize tracing subscriber with optional verbose logging
///
/// expect: "I can access all hKask functionality through the kask CLI" [P3]
/// pre:  verbose is a boolean flag
/// post: if verbose → EnvFilter::new("debug")
/// post: if not verbose → EnvFilter::from_default_env()
/// post: global tracing subscriber is set (panics if already set)
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
