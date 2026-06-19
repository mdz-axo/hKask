use bolero::check;

/// Triple construction must never panic with arbitrary entity/attribute/value.
#[test]
fn fuzz_triple_construct() {
    check!()
        .with_type::<(String, String, String)>()
        .for_each(|(entity, attr, value)| {
            let v: serde_json::Value = serde_json::Value::String(value.clone());
            let triple =
                hkask_storage::Triple::new(&entity, &attr, v, hkask_types::WebID::default());
            assert!(!triple.id.as_uuid().is_nil());
            assert_eq!(triple.entity.as_str(), entity.as_str());
            assert_eq!(triple.attribute.as_str(), attr.as_str());
        });
}

/// Spec description decomposition must never panic.
#[test]
fn fuzz_spec_decompose() {
    check!().with_type::<String>().for_each(|desc| {
        let _ = hkask_storage::spec_ops::decompose_description(&desc);
    });
}
