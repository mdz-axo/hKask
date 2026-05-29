pub fn assert_variety_absorbed_at_least(absorbed: i64, min_expected: i64) {
    assert!(
        absorbed >= min_expected,
        "expected absorbed variety >= {}, got {}",
        min_expected,
        absorbed
    );
}

pub fn assert_variety_deficit_below(deficit: i64, max_allowed: i64) {
    assert!(
        deficit < max_allowed,
        "expected deficit < {}, got {}",
        max_allowed,
        deficit
    );
}

pub fn assert_algedonic_triggered_when_deficit_above(
    deficit: i64,
    threshold: i64,
    algedonic_triggered: bool,
) {
    if deficit > threshold {
        assert!(
            algedonic_triggered,
            "expected algedonic trigger when deficit {} exceeds threshold {}",
            deficit, threshold
        );
    }
}
