use bolero::check;

/// Skill front matter parsing must never panic on arbitrary YAML-like input.
#[test]
fn fuzz_skill_front_matter_parse() {
    check!().with_type::<String>().for_each(|s| {
        let _ = hkask_templates::skill_loader::parse_front_matter(&s);
    });
}

/// Capability validator must never panic.
#[test]
fn fuzz_capability_validate() {
    check!()
        .with_type::<(String, String)>()
        .for_each(|(template_id, cap)| {
            let validator = hkask_templates::capability_validator::CapabilityValidator::new();
            let _ = validator.validate_capabilities(&template_id, &[cap]);
        });
}
