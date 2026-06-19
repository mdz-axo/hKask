use bolero::check;

/// Settings model name resolution must never panic.
#[test]
fn fuzz_model_name_resolve() {
    check!()
        .with_type::<(String, String, String)>()
        .for_each(|(env, settings, default)| {
            let _ =
                hkask_services_core::settings::HkaskSettings::resolve_model(env, settings, default);
        });
}
