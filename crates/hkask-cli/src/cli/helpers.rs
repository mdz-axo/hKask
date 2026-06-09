//! CLI helper utilities

use hkask_types::TemplateType as Type;

/// Parse a string into a DataCategory (delegates to DataCategory::parse)
pub fn parse_data_category(s: &str) -> hkask_types::DataCategory {
    hkask_types::DataCategory::parse(s)
}

/// Parse a template type string into a TemplateType enum
pub fn parse_template_type(type_str: &str) -> Option<Type> {
    Type::parse_str(type_str)
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
