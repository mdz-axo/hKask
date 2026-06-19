use bolero::check;

/// Skill front matter parsing must never panic on arbitrary YAML-like input.
#[test]
fn fuzz_skill_front_matter_parse() {
    check!().with_type::<String>().for_each(|s| {
        let _ = hkask_templates::skill_loader::SkillLoader::parse_front_matter(&s);
    });
}

/// Capability validator must never panic with empty inputs.
#[test]
fn fuzz_capability_validate() {
    check!().with_type::<String>().for_each(|template_id| {
        let validator = hkask_templates::capability_validator::CapabilityAwareValidator::new();
        let _ = validator.validate_capabilities(&template_id, &[], &[]);
    });
}
