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
}
