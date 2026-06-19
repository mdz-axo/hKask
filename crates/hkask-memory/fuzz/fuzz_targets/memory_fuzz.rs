use bolero::check;

/// Salience computation must never panic on arbitrary text.
#[test]
fn fuzz_salience_compute() {
    check!().with_type::<String>().for_each(|text| {
        let _signals = hkask_memory::salience::compute_method_signals(text.as_str());
    });
}
