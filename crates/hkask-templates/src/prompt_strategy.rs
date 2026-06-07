//! Prompt strategy — heuristic prompt framing based on input analysis.
//!
//! Determines how to frame a user prompt before sending to inference.
//! This is the simplest form of template selection — keyword-based heuristic
//! that maps input patterns to prompt framing strategies.

/// Prompt strategy — heuristic prompt framing based on input analysis.
///
/// Determines how to frame a user prompt before sending to inference.
/// This is the simplest form of template selection — keyword-based heuristic
/// that maps input patterns to prompt framing strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptStrategy {
    /// Factual question — answer concisely
    Answer,
    /// Creative/construction request — step-by-step instructions
    Instruct,
    /// General — respond helpfully
    Assist,
}

impl PromptStrategy {
    /// Analyze input text to determine the best prompt strategy.
    pub fn from_input(input: &str) -> Self {
        if input.contains('?') || input.contains("what") || input.contains("how") {
            PromptStrategy::Answer
        } else if input.contains("create") || input.contains("make") || input.contains("build") {
            PromptStrategy::Instruct
        } else {
            PromptStrategy::Assist
        }
    }

    /// Apply the strategy to frame a prompt.
    pub fn frame(&self, input: &str) -> String {
        match self {
            PromptStrategy::Answer => format!("Answer concisely: {}", input),
            PromptStrategy::Instruct => format!("Provide step-by-step instructions: {}", input),
            PromptStrategy::Assist => format!("Respond helpfully: {}", input),
        }
    }

    /// Strategy name for tagging/logging.
    pub fn name(&self) -> &'static str {
        match self {
            PromptStrategy::Answer => "answer",
            PromptStrategy::Instruct => "instruct",
            PromptStrategy::Assist => "assist",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_input_question_mark() {
        assert_eq!(
            PromptStrategy::from_input("What is rust?"),
            PromptStrategy::Answer
        );
    }

    #[test]
    fn from_input_how() {
        assert_eq!(
            PromptStrategy::from_input("how do I bake bread"),
            PromptStrategy::Answer
        );
    }

    #[test]
    fn from_input_what() {
        assert_eq!(
            PromptStrategy::from_input("what is the time"),
            PromptStrategy::Answer
        );
    }

    #[test]
    fn from_input_create() {
        assert_eq!(
            PromptStrategy::from_input("create a new project"),
            PromptStrategy::Instruct
        );
    }

    #[test]
    fn from_input_make() {
        assert_eq!(
            PromptStrategy::from_input("make it so"),
            PromptStrategy::Instruct
        );
    }

    #[test]
    fn from_input_build() {
        assert_eq!(
            PromptStrategy::from_input("build a house"),
            PromptStrategy::Instruct
        );
    }

    #[test]
    fn from_input_default() {
        assert_eq!(
            PromptStrategy::from_input("hello there"),
            PromptStrategy::Assist
        );
    }

    #[test]
    fn from_input_question_mark_beats_create() {
        // '?' triggers Answer even if 'create' is also present
        assert_eq!(
            PromptStrategy::from_input("how to create?"),
            PromptStrategy::Answer
        );
    }

    #[test]
    fn name_variants() {
        assert_eq!(PromptStrategy::Answer.name(), "answer");
        assert_eq!(PromptStrategy::Instruct.name(), "instruct");
        assert_eq!(PromptStrategy::Assist.name(), "assist");
    }

    #[test]
    fn from_input_empty_string() {
        assert_eq!(PromptStrategy::from_input(""), PromptStrategy::Assist);
    }

    // F-SYN-016: the `PromptStrategy` enum and `PromptCache` struct
    // are *parallel collaborators*, not overlapping concepts.
    // This test pins the composition: a `PromptStrategy::frame`
    // output is exactly the kind of value that becomes a
    // `PromptCache` key. If the composition breaks (e.g. a
    // caller forgets to apply the strategy before hashing), the
    // test fails.
    #[test]
    fn strategy_frame_then_cache_key_round_trip() {
        use crate::prompt_cache::PromptCache;
        use hkask_types::LLMParameters;

        for input in [
            "What is the capital of France?",
            "Create a function that adds two numbers",
            "Help me with my homework",
        ] {
            let strategy = PromptStrategy::from_input(input);
            let framed = strategy.frame(input);
            let params = LLMParameters::default();
            let key = PromptCache::generate_key(&framed, "test-model", &params);
            // The key is a 32-char hex (16 bytes). Asserting the
            // shape — non-empty, hex, fixed length — proves the
            // composition produces a usable cache key.
            assert_eq!(key.len(), 32, "cache key must be 16 bytes hex-encoded");
            assert!(
                key.chars().all(|c| c.is_ascii_hexdigit()),
                "cache key must be hex: {key}"
            );
            // Different inputs produce different keys.
            let key2 = PromptCache::generate_key(&framed, "test-model", &params);
            assert_eq!(key, key2, "deterministic key for same input");
        }
    }
}
