//! Plussing (Catmull) — Extract agreeable components, silently discard remainder.
//!
//! Plussing is the constructive default posture: find what you can agree with,
//! build on it, and silently omit what you cannot. Criticism is deletion-by-omission
//! — never explicit negation.
//!
//! The extraction uses heuristic semantic agreeableness detection. In production,
//! this delegates to `hkask-inference` for LLM-based agreeableness scoring.

use crate::protocol::Contribution;

/// A component of a contribution that was identified as agreeable.
///
/// Carries the extracted text and a confidence score (0.0–1.0).
#[derive(Debug, Clone, PartialEq)]
pub struct AgreeableComponent {
    /// The extracted agreeable text fragment.
    pub text: String,
    /// Confidence that this component is genuinely agreeable (0.0–1.0).
    pub confidence: f64,
}

/// Output of the Plussing process: selected seeds + constructive build.
#[derive(Debug, Clone, PartialEq)]
pub struct PlussedResponse {
    /// The agreeable components extracted from the contribution.
    pub selected_seeds: Vec<AgreeableComponent>,
    /// The constructive response built on the selected seeds.
    pub build: String,
}

/// Process a contribution through the Plussing filter.
///
/// 1. Extract agreeable components from the contribution.
/// 2. Silently discard the remainder (no negation).
/// 3. Build a constructive response on the selected seeds.
///
/// In the current implementation, extraction uses heuristic keyword-based
/// agreeableness detection. The production path will delegate to
/// `hkask-inference` for semantic agreeableness scoring.
pub fn process(contribution: &Contribution) -> PlussedResponse {
    let seeds = extract_agreeable(contribution);
    let build = build_on(&seeds, contribution);
    PlussedResponse {
        selected_seeds: seeds,
        build,
    }
}

/// Extract agreeable components from a contribution.
///
/// Uses heuristic detection of constructive/positive language patterns.
/// Returns components sorted by confidence (highest first).
pub fn extract_agreeable(contribution: &Contribution) -> Vec<AgreeableComponent> {
    let content = &contribution.content;

    // Heuristic: split on sentence boundaries and score each sentence.
    let sentences: Vec<&str> = content
        .split(['.', '!', '?'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let mut components: Vec<AgreeableComponent> = sentences
        .iter()
        .filter_map(|sentence| {
            let confidence = agreeableness_score(sentence);
            if confidence > 0.0 {
                Some(AgreeableComponent {
                    text: sentence.to_string(),
                    confidence,
                })
            } else {
                None // Silently discard — this is the core Plussing constraint
            }
        })
        .collect();

    // Sort by confidence descending — build on the strongest seeds first.
    components.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    components
}

/// Heuristic agreeableness score for a sentence.
///
/// Returns 0.0–1.0 based on presence of constructive language markers.
/// Uses word-boundary matching to avoid false positives (e.g., "and" in "handling").
/// This is a placeholder for LLM-based semantic scoring via `hkask-inference`.
fn agreeableness_score(sentence: &str) -> f64 {
    let lower = sentence.to_lowercase();
    // Split into words for boundary-aware matching.
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Constructive/agreeable markers — sentences that build, suggest, or affirm.
    // Multi-word markers are checked against the full lowercased string.
    let constructive_markers = [
        "yes",
        "agree",
        "good",
        "great",
        "interesting",
        "let's",
        "consider",
        "explore",
        "try",
        "extend",
        "improve",
        "add",
        "also",
        "worth",
        "useful",
        "helpful",
        "promising",
    ];

    // Multi-word constructive phrases (checked against full string).
    let constructive_phrases = [
        "we could",
        "what if",
        "how about",
        "maybe we",
        "i like",
        "that works",
        "build on",
    ];

    // Negative markers — sentences that reject, dismiss, or attack.
    let negative_markers = [
        "no",
        "wrong",
        "bad",
        "terrible",
        "stupid",
        "never",
        "can't",
        "won't",
        "don't",
        "doesn't",
        "impossible",
        "useless",
        "waste",
        "fail",
        "broken",
        "hate",
        "ridiculous",
        "absurd",
    ];

    // Count single-word constructive markers (word-boundary match).
    let constructive_count: usize = constructive_markers
        .iter()
        .filter(|m| words.contains(m))
        .count();

    // Count multi-word constructive phrases (substring match on full string).
    let phrase_count: usize = constructive_phrases
        .iter()
        .filter(|p| lower.contains(*p))
        .count();

    let total_constructive = constructive_count + phrase_count;

    // Count negative markers (word-boundary match).
    let negative_count: usize = negative_markers
        .iter()
        .filter(|m| words.contains(m))
        .count();

    if total_constructive == 0 && negative_count == 0 {
        // Neutral sentence — include with low confidence.
        return 0.3;
    }

    if negative_count > total_constructive {
        // Dominantly negative — silently discard (return 0.0).
        return 0.0;
    }

    // Constructive-leaning: score proportional to constructive markers.
    let total = total_constructive + negative_count;
    if total == 0 {
        return 0.3;
    }
    (total_constructive as f64 / total as f64).clamp(0.0, 1.0)
}

/// Build a constructive response on the selected agreeable seeds.
///
/// The build acknowledges the agreeable components and extends them
/// constructively. Never references discarded components.
pub fn build_on(seeds: &[AgreeableComponent], _contribution: &Contribution) -> String {
    if seeds.is_empty() {
        // Edge case: nothing agreeable found. Still respond constructively
        // without referencing the disagreeable content.
        return "Let's explore this from a different angle. What aspect would you like to focus on?"
            .to_string();
    }

    let seed_summary: Vec<String> = seeds
        .iter()
        .take(3) // Build on top 3 seeds max — avoid overwhelming.
        .map(|s| s.text.clone())
        .collect();

    format!(
        "Building on your points about '{}' — let's extend that further. What if we also considered the implications for the broader system?",
        seed_summary.join("', '")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::id::WebID;

    fn make_contribution(content: &str) -> Contribution {
        Contribution {
            source: WebID::new(),
            content: content.to_string(),
            turn_index: 0,
        }
    }

    #[test]
    fn extracts_agreeable_from_constructive_input() {
        let c = make_contribution(
            "I think we should refactor the auth module. It's well-designed but could be simpler.",
        );
        let seeds = extract_agreeable(&c);
        // Both sentences are constructive — should extract at least one.
        assert!(!seeds.is_empty(), "Should find agreeable components");
        // All extracted components should have confidence > 0.
        for seed in &seeds {
            assert!(seed.confidence > 0.0, "Seed confidence must be positive");
        }
    }

    #[test]
    fn silently_discards_negative_content() {
        let c = make_contribution(
            "This code is terrible and broken. The design is wrong. But we could improve error handling.",
        );
        let seeds = extract_agreeable(&c);
        // The negative sentences should be discarded.
        // The constructive sentence should be extracted.
        let has_negative = seeds.iter().any(|s| {
            s.text.contains("terrible") || s.text.contains("wrong") || s.text.contains("broken")
        });
        assert!(!has_negative, "Negative content must be silently discarded");
        // The constructive sentence should survive.
        let has_constructive = seeds.iter().any(|s| s.text.contains("improve"));
        assert!(has_constructive, "Constructive content must be extracted");
    }

    #[test]
    fn build_never_references_discarded() {
        let c = make_contribution("This is wrong. But we could try a different approach.");
        let seeds = extract_agreeable(&c);
        let build = build_on(&seeds, &c);
        // Build must not contain the discarded negative content.
        assert!(
            !build.contains("wrong"),
            "Build must not reference discarded 'wrong'"
        );
    }

    #[test]
    fn handles_fully_disagreeable_contribution() {
        let c = make_contribution("This is terrible. Absolutely wrong. Completely broken.");
        let seeds = extract_agreeable(&c);
        // All sentences are negative — should be empty.
        assert!(seeds.is_empty(), "All-negative input should yield no seeds");
        let build = build_on(&seeds, &c);
        // Build should still be constructive, not empty or negative.
        assert!(!build.is_empty(), "Build must not be empty");
        assert!(
            !build.contains("terrible") && !build.contains("wrong"),
            "Build must not echo negative content"
        );
    }

    #[test]
    fn handles_empty_contribution() {
        let c = make_contribution("");
        let seeds = extract_agreeable(&c);
        assert!(seeds.is_empty(), "Empty input should yield no seeds");
        let build = build_on(&seeds, &c);
        assert!(!build.is_empty(), "Build must not be empty for empty input");
    }

    #[test]
    fn confidence_in_valid_range() {
        let c = make_contribution("Great idea! Let's explore that. I agree completely.");
        let seeds = extract_agreeable(&c);
        for seed in &seeds {
            assert!(
                (0.0..=1.0).contains(&seed.confidence),
                "Confidence {} out of range",
                seed.confidence
            );
        }
    }
}
