//! Improv cascade — recursive composition of improv modes.
//!
//! An `ImprovCascade` composes multiple improv modes into a sequential pipeline,
//! bounded by the matryoshka limit (7). Each step's output feeds into the next
//! step as input. Cascades can nest: a step can itself be a `Cascade` mode,
//! enabling recursive composition within the depth bound.
//!
//! This mirrors the `BundleManifest` cascade system: steps execute sequentially,
//! and total applications are bounded by `MATRYOSHKA_LIMIT`.

use crate::ConversationContext;
use crate::modes::ImprovMode;
use crate::protocol::{Contribution, ImprovResponse};
use thiserror::Error;

/// The matryoshka limit — maximum total mode applications in any composition.
///
/// Mirrors the `BundleManifest` cascade depth limit of 7 steps.
pub const MATRYOSHKA_LIMIT: u8 = 7;

/// Errors that can occur during improv cascade construction or execution.
#[derive(Debug, Error)]
pub enum ImprovError {
    #[error("Cascade depth {depth} exceeds matryoshka limit ({limit})")]
    MatryoshkaExceeded { depth: usize, limit: u8 },

    #[error("Recursion depth {depth} exceeded at runtime (limit: {limit})")]
    RecursionExceeded { depth: u8, limit: u8 },

    #[error("Cascade must have at least 1 step")]
    EmptyCascade,
}

/// A cascade of improv mode applications — recursive composition.
///
/// Cascades execute sequentially: the output of step N becomes the input
/// to step N+1. Total applications (including nested cascades) are bounded
/// by `MATRYOSHKA_LIMIT` (7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImprovCascade {
    /// Modes in execution order. A mode can itself be a `Cascade` for nesting.
    pub modes: Vec<ImprovMode>,
}

impl ImprovCascade {
    /// Create a new cascade from a sequence of modes.
    ///
    /// Validates that total mode applications (including nested cascades)
    /// does not exceed `MATRYOSHKA_LIMIT` (7).
    pub fn new(modes: Vec<ImprovMode>) -> Result<Self, ImprovError> {
        if modes.is_empty() {
            return Err(ImprovError::EmptyCascade);
        }

        let total_apps = Self::count_applications(&modes);
        if total_apps > MATRYOSHKA_LIMIT as usize {
            return Err(ImprovError::MatryoshkaExceeded {
                depth: total_apps,
                limit: MATRYOSHKA_LIMIT,
            });
        }

        Ok(Self { modes })
    }

    /// Execute the cascade on an initial contribution.
    ///
    /// Each mode is applied to the output of the previous mode. For nested
    /// `Cascade` modes, the inner cascade executes fully before the outer
    /// continues. Recursion depth is tracked via the context.
    pub fn execute(
        &self,
        initial: &Contribution,
        context: &ConversationContext,
    ) -> Result<ImprovResponse, ImprovError> {
        if context.recursion_depth >= MATRYOSHKA_LIMIT {
            return Err(ImprovError::RecursionExceeded {
                depth: context.recursion_depth,
                limit: MATRYOSHKA_LIMIT,
            });
        }

        let mut current_response: Option<ImprovResponse> = None;

        for mode in &self.modes {
            let input = match &current_response {
                None => initial.clone(),
                Some(prev) => Contribution {
                    source: initial.source,
                    content: prev.content_text(),
                    turn_index: initial.turn_index,
                },
            };

            let step_context = match mode {
                ImprovMode::Cascade(_) => context.descend(),
                _ => context.clone(),
            };

            current_response = Some(mode.respond(&input, &step_context));
        }

        // Safe: cascade has at least 1 mode (enforced at construction).
        Ok(current_response.unwrap())
    }

    /// Number of modes in this cascade (not counting nested).
    pub fn step_count(&self) -> usize {
        self.modes.len()
    }

    /// Total number of mode applications across all nesting levels.
    pub fn total_applications(&self) -> usize {
        Self::count_applications(&self.modes)
    }

    /// Count total applications for a slice of modes, recursing into nested cascades.
    fn count_applications(modes: &[ImprovMode]) -> usize {
        modes
            .iter()
            .map(|m| match m {
                ImprovMode::Cascade(inner) => 1 + inner.total_applications(),
                _ => 1,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::riffing::RiffReturn;
    use hkask_types::id::WebID;

    fn make_contribution(content: &str) -> Contribution {
        Contribution {
            source: WebID::new(),
            content: content.to_string(),
            turn_index: 0,
        }
    }

    fn make_context() -> ConversationContext {
        ConversationContext::new(WebID::new())
    }

    // REQ: Cascade composes modes sequentially — output of step N feeds step N+1
    #[test]
    fn cascade_composes_modes_sequentially() {
        let cascade =
            ImprovCascade::new(vec![ImprovMode::Plussing, ImprovMode::YesAnd]).expect("valid");

        assert_eq!(cascade.step_count(), 2);

        let contrib = make_contribution("We should improve error handling and add caching");
        let ctx = make_context();
        let result = cascade.execute(&contrib, &ctx).expect("execution ok");

        match result {
            ImprovResponse::Extended {
                accepted_base,
                extension: _,
            } => {
                assert!(
                    accepted_base.contains("Building on"),
                    "YesAnd should accept the Plussed output, got: {}",
                    accepted_base
                );
            }
            other => panic!("Expected Extended after Plussing→YesAnd, got {:?}", other),
        }
    }

    // REQ: Cascade enforces matryoshka limit (max 7 total applications)
    #[test]
    fn enforces_matryoshka_limit() {
        // 8 simple modes — exceeds limit.
        let result = ImprovCascade::new(vec![ImprovMode::Plussing; 8]);
        assert!(matches!(
            result.unwrap_err(),
            ImprovError::MatryoshkaExceeded { depth: 8, limit: 7 }
        ));

        // 7 simple modes — at the limit, valid.
        assert!(ImprovCascade::new(vec![ImprovMode::Plussing; 7]).is_ok());
    }

    // REQ: Cascade rejects empty mode list
    #[test]
    fn rejects_empty_cascade() {
        assert!(matches!(
            ImprovCascade::new(vec![]).unwrap_err(),
            ImprovError::EmptyCascade
        ));
    }

    // REQ: Cascade with nested Cascade tracks total applications correctly
    #[test]
    fn tracks_nested_applications() {
        let inner =
            ImprovCascade::new(vec![ImprovMode::Plussing, ImprovMode::YesAnd]).expect("inner");

        let outer = ImprovCascade::new(vec![
            ImprovMode::Riffing {
                return_policy: RiffReturn::ReturnToGroup,
            },
            ImprovMode::Cascade(inner),
        ])
        .expect("outer");

        assert_eq!(outer.step_count(), 2);
        // Riffing (1) + Cascade step (1) + inner Plussing (1) + inner YesAnd (1) = 4
        assert_eq!(outer.total_applications(), 4);
    }

    // REQ: Cascade execution with nested cascade descends context
    #[test]
    fn nested_cascade_descends_context() {
        let inner =
            ImprovCascade::new(vec![ImprovMode::Plussing, ImprovMode::YesAnd]).expect("inner");

        let outer = ImprovCascade::new(vec![ImprovMode::Plussing, ImprovMode::Cascade(inner)])
            .expect("outer");

        let contrib = make_contribution("Let's refactor the auth module and add tests");
        let ctx = make_context();
        let result = outer.execute(&contrib, &ctx).expect("execution ok");

        assert!(matches!(result, ImprovResponse::Extended { .. }));
    }

    // REQ: Deeply nested cascade exceeding matryoshka limit is rejected
    #[test]
    fn rejects_deep_nesting() {
        // Build a cascade of 7 total applications (at the limit).
        let mut current = ImprovCascade::new(vec![ImprovMode::Plussing]).expect("depth 1");
        // Nest 6 more times: each nesting adds 1 application (the wrapper).
        for _ in 0..6 {
            current = ImprovCascade::new(vec![ImprovMode::Cascade(current)]).expect("within limit");
        }
        assert_eq!(current.total_applications(), 7);

        // One more nesting → 8 applications → rejected.
        let result = ImprovCascade::new(vec![ImprovMode::Cascade(current)]);
        assert!(matches!(
            result.unwrap_err(),
            ImprovError::MatryoshkaExceeded { depth: 8, limit: 7 }
        ));
    }

    // REQ: Cascade execution with exceeded recursion depth at runtime is caught
    #[test]
    fn catches_runtime_recursion_exceeded() {
        let cascade = ImprovCascade::new(vec![ImprovMode::Plussing]).expect("valid");
        let contrib = make_contribution("test");
        let mut ctx = make_context();
        ctx.recursion_depth = MATRYOSHKA_LIMIT;

        assert!(matches!(
            cascade.execute(&contrib, &ctx).unwrap_err(),
            ImprovError::RecursionExceeded { depth: 7, limit: 7 }
        ));
    }
}
