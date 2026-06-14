//! Integration tests for Plussing mode.
//!
//! REQ: filters without negation — verifies the core Plussing constraint
//! that negative content is silently discarded, never explicitly negated.

use hkask_improv::plussing::{extract_agreeable, process};
use hkask_improv::protocol::Contribution;
use hkask_types::id::WebID;

fn make_contribution(content: &str) -> Contribution {
    Contribution {
        source: WebID::new(),
        content: content.to_string(),
        turn_index: 0,
    }
}

// REQ: Plussing extraction accuracy — constructive content is identified
#[test]
fn extraction_identifies_constructive_content() {
    let c = make_contribution(
        "Great idea! Let's explore that further. I think we could also add caching. \
         This would improve performance significantly.",
    );
    let seeds = extract_agreeable(&c);
    // All sentences are constructive — should extract multiple seeds.
    assert!(
        seeds.len() >= 2,
        "Should find multiple agreeable components"
    );
    // Seeds should be sorted by confidence (highest first).
    for i in 1..seeds.len() {
        assert!(
            seeds[i - 1].confidence >= seeds[i].confidence,
            "Seeds should be sorted by confidence descending"
        );
    }
}

// REQ: Plussing never produces negation in output
#[test]
fn output_never_contains_negation() {
    let c = make_contribution(
        "This approach is wrong. The design is terrible. But we could try a different pattern.",
    );
    let response = process(&c);
    // The build must not contain any negative language.
    let negative_words = ["wrong", "terrible", "bad", "no", "never", "can't", "don't"];
    for word in &negative_words {
        assert!(
            !response.build.to_lowercase().contains(word),
            "Build contains negative word '{}': {}",
            word,
            response.build
        );
    }
    // The selected seeds must not contain negative content.
    for seed in &response.selected_seeds {
        for word in &negative_words {
            assert!(
                !seed.text.to_lowercase().contains(word),
                "Seed contains negative word '{}': {}",
                word,
                seed.text
            );
        }
    }
}

// REQ: Plussing handles mixed constructive/negative input
#[test]
fn handles_mixed_input() {
    let c = make_contribution(
        "The error handling is broken. We should refactor it. \
         The tests are useless. But the API design is good. \
         Let's keep the API and improve error handling.",
    );
    let seeds = extract_agreeable(&c);
    // Constructive sentences should be extracted.
    let has_refactor = seeds.iter().any(|s| s.text.contains("refactor"));
    let has_api_good = seeds.iter().any(|s| s.text.contains("API design is good"));
    let has_improve = seeds.iter().any(|s| s.text.contains("improve"));
    assert!(
        has_refactor || has_api_good || has_improve,
        "Should extract constructive content"
    );
    // Negative sentences should be absent.
    let has_broken = seeds.iter().any(|s| s.text.contains("broken"));
    let has_useless = seeds.iter().any(|s| s.text.contains("useless"));
    assert!(!has_broken, "Should discard 'broken'");
    assert!(!has_useless, "Should discard 'useless'");
}

// REQ: Plussing build is always constructive, even with empty seeds
#[test]
fn build_always_constructive() {
    // Fully negative input.
    let c = make_contribution("This is terrible. Completely wrong. Absolutely broken.");
    let response = process(&c);
    assert!(response.selected_seeds.is_empty());
    assert!(!response.build.is_empty(), "Build must not be empty");
    assert!(
        !response.build.contains("terrible")
            && !response.build.contains("wrong")
            && !response.build.contains("broken"),
        "Build must not echo negative content"
    );

    // Empty input.
    let c = make_contribution("");
    let response = process(&c);
    assert!(response.selected_seeds.is_empty());
    assert!(
        !response.build.is_empty(),
        "Build must not be empty for empty input"
    );
}
