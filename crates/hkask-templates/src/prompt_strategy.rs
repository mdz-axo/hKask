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
use hkask_rsolidity::contract;

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
    ///
    /// \[P3\] Motivating: Generative Space — constructs prompt strategy from user input
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
    ///
    /// \[P3\] Motivating: Generative Space — frames prompt for a strategy step
    pub fn frame(&self, input: &str) -> String {
        match self {
            PromptStrategy::Answer => format!("Answer concisely: {}", input),
            PromptStrategy::Instruct => format!("Provide step-by-step instructions: {}", input),
            PromptStrategy::Assist => format!("Respond helpfully: {}", input),
        }
    }

    /// Strategy name for tagging/logging.
    ///
    /// \[P3\] Motivating: Generative Space — names the selected strategy
    pub fn name(&self) -> &'static str {
        match self {
            PromptStrategy::Answer => "answer",
            PromptStrategy::Instruct => "instruct",
            PromptStrategy::Assist => "assist",
        }
    }
}
