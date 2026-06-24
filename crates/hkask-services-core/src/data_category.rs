/// Parse a string into a DataCategory (delegates to DataCategory::parse).
///
/// Known categories map directly; unknown strings become `DataCategory::Custom`.
/// This is the shared canonical location — previously duplicated across
/// CLI helpers, CLI sovereignty, and API routes.
pub fn parse_data_category(s: &str) -> hkask_types::DataCategory {
    hkask_types::DataCategory::parse(s)
}
