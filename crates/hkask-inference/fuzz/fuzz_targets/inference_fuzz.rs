use bolero::check;

/// Model name prefix parsing must never panic.
#[test]
fn fuzz_model_name_prefix_never_panics() {
    check!().with_type::<String>().for_each(|model_name| {
        let _ = model_name.find('/');
        let _ = model_name.split('/').next();
    });
}

/// ProviderId::parse_from_model must never panic.
#[test]
fn fuzz_provider_id_parse_model() {
    check!().with_type::<String>().for_each(|model| {
        let _ = hkask_inference::config::ProviderId::parse_from_model(model);
    });
}

/// Prompt validation must never panic.
#[test]
fn fuzz_prompt_validation_never_panics() {
    check!().with_type::<String>().for_each(|prompt| {
        let _ = hkask_inference::chat_protocol::validate_prompt(prompt);
    });
}
