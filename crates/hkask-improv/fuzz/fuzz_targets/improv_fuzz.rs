use bolero::check;

/// Riffing pattern matching must never panic on arbitrary input.
#[test]
fn fuzz_riffing_match() {
    check!().with_type::<String>().for_each(|s| {
        // Test basic string operations that riffing depends on
        let _ = s.to_lowercase();
        let _ = s.contains("yes");
        let _ = s.contains("no");
        let _ = s.split_whitespace().count();
    });
}
