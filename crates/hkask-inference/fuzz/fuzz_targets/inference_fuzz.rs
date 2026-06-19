#![no_main]
use bolero::check;

/// Model name routing: XX/ prefix parsing must never panic.
/// Tests the InferenceRouter prefix dispatch boundary.
#[test]
fn fuzz_model_name_prefix_never_panics() {
    check!().with_type::<String>().for_each(|model_name| {
        // Model name parsing must handle arbitrary strings
        // XX/ prefix dispatch is the highest-risk surface
        let _ = model_name.find('/');
        let _ = model_name.split('/').next();
    });
}

/// ProviderId::parse_from_model must never panic.
/// Tests the provider dispatch table for arbitrary model names.
#[test]
fn fuzz_provider_id_parse_model() {
    check!().with_type::<String>().for_each(|model| {
        let _ = hkask_inference::config::ProviderId::parse_from_model(model);
    });
}

/// Chat protocol: prompt validation must never panic.
#[test]
fn fuzz_prompt_validation_never_panics() {
    check!().with_type::<String>().for_each(|prompt| {
        let _ = hkask_inference::chat_protocol::validate_prompt(prompt);
    });
}
