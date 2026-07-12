//! Utility helpers for the embedding pipeline.

/// Strip a recognized 2-letter provider prefix from a model name.
///
/// "KC/qwen/qwen3-235b-a22b-2507" → "qwen/qwen3-235b-a22b-2507"
/// "qwen/qwen3.5-35b-a3b"        → "qwen/qwen3.5-35b-a3b" (no recognized prefix)
///
/// Used before sending model IDs to classifier base URLs, which determine
/// the provider independently of the model string.
pub(crate) fn strip_provider_prefix(model: &str) -> &str {
    for prefix in ["DI/", "KC/", "FA/", "TG/", "OR/", "RP/", "BT/"] {
        if let Some(rest) = model.strip_prefix(prefix) {
            return rest;
        }
    }
    model
}
