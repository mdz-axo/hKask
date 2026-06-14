//! Improv cascade — recursive composition of improv modes.
//!
//! An `ImprovCascade` composes multiple improv modes into a sequential pipeline,
//! bounded by the matryoshka limit (7). Each step's output feeds into the next
//! step as input. Cascades can nest: a step can itself be a `Cascade` mode,
//! enabling recursive composition within the depth bound.
//!
//! This mirrors the `BundleManifest` cascade system: steps have ordinals,
//! execution is sequential, and depth is bounded by `MATRYOSHKA_LIMIT`.

use crate::ConversationContext;
use crate::modes::ImprovMode;
use crate::protocol::{Contribution, ImprovResponse};
use thiserror::Error;

/// The matryoshka limit — maximum recursion depth for improv composition.
///
/// Mirrors the `BundleManifest` cascade depth limit of 7 steps.
/// Any cascade exceeding this depth is rejected at construction time.
pub const MATRYOSHKA_LIMIT: u8 = 7;

/// Errors that can occur during improv cascade construction or execution.
#[derive(Debug, Error)]
pub enum ImprovError {
    /// Cascade depth exceeds the matryoshka limit.
    #[error("Cascade depth {depth} exceeds matryoshka limit ({limit})")]
    MatryoshkaExceeded { depth: usize, limit: u8 },

    /// Recursion depth exceeded during execution (defensive check).
    #[error("Recursion depth {depth} exceeded at runtime (limit: {limit})")]
    RecursionExceeded { depth: u8, limit: u8 },

    /// Cascade has no steps — empty composition is invalid.
    #[error("Cascade must have at least 1 step")]
    EmptyCascade,
}

/// A single step in an improv cascade.
///
/// Each step applies one improv mode. Steps execute in ordinal order.
/// A step's mode can itself be a `Cascade`, enabling recursive nesting.
#[derive(Debug, Clone)]
pub struct ImprovCascadeStep {
    /// Position in the cascade sequence (1-based).
    pub ordinal: u32,
    /// The improv mode to apply at this step.
    pub mode: ImprovMode,
}

/// A cascade of improv mode applications — recursive composition.
///
/// Cascades execute sequentially: the output of step N becomes the input
/// to step N+1. The total depth (including nested cascades) is bounded
/// by `MATRYOSHKA_LIMIT` (7).
///
/// # Recursive nesting
///
/// A cascade step can itself be a `Cascade` mode. When executed, the inner
/// cascade runs to completion before the outer cascade continues. The
/// recursion depth is tracked and enforced at each level.
#[derive(Debug, Clone)]
pub struct ImprovCascade {
    /// Steps in ordinal order.
    pub steps: Vec<ImprovCascadeStep>,
    /// Total recursion depth of this cascade (1 + max nested depth).
    pub total_depth: u8,
}

impl ImprovCascade {
    /// Create a new cascade from a sequence of modes.
    ///
    /// Validates that the total number of mode applications (including nested
    /// cascades) does not exceed `MATRYOSHKA_LIMIT` (7). Steps are assigned
    /// ordinals automatically (1-based).
    ///
    /// # Errors
    /// - `EmptyCascade` if `modes` is empty
    /// - `MatryoshkaExceeded` if total applications > 7
    pub fn new(modes: Vec<ImprovMode>) -> Result<Self, ImprovError> {
        if modes.is_empty() {
            return Err(ImprovError::EmptyCascade);
        }

        // Count total mode applications across all nesting levels.
        // Cascade steps count as 1 application + their inner applications.
        let total_apps: usize = modes
            .iter()
            .map(|m| match m {
                ImprovMode::Cascade(inner) => 1 + inner.total_applications(),
                _ => 1,
            })
            .sum();

        if total_apps > MATRYOSHKA_LIMIT as usize {
            return Err(ImprovError::MatryoshkaExceeded {
                depth: total_apps,
                limit: MATRYOSHKA_LIMIT,
            });
        }

        // Compute total nesting depth: 1 + max depth of any nested cascade.
        let max_nested_depth: u8 = modes
            .iter()
            .map(|m| match m {
                ImprovMode::Cascade(inner) => inner.total_depth,
                _ => 0,
            })
            .max()
            .unwrap_or(0);

        let total_depth = 1u8.saturating_add(max_nested_depth);

        let steps: Vec<ImprovCascadeStep> = modes
            .into_iter()
            .enumerate()
            .map(|(i, mode)| ImprovCascadeStep {
                ordinal: (i as u32) + 1,
                mode,
            })
            .collect();

        Ok(Self { steps, total_depth })
    }

    /// Execute the cascade on an initial contribution.
    ///
    /// Each step's mode is applied to the output of the previous step.
    /// For nested `Cascade` modes, the inner cascade executes fully
    /// before the outer cascade continues. Recursion depth is tracked
    /// via the context.
    ///
    /// Returns the final response after all steps complete.
    pub fn execute(
        &self,
        initial: &Contribution,
        context: &ConversationContext,
    ) -> Result<ImprovResponse, ImprovError> {
        // Defensive: check that we haven't exceeded the limit at runtime.
        if context.recursion_depth >= MATRYOSHKA_LIMIT {
            return Err(ImprovError::RecursionExceeded {
                depth: context.recursion_depth,
                limit: MATRYOSHKA_LIMIT,
            });
        }

        let mut current_response: Option<ImprovResponse> = None;

        for step in &self.steps {
            // Determine input: first step uses initial contribution,
            // subsequent steps use the previous step's output text.
            let input = match &current_response {
                None => initial.clone(),
                Some(prev) => {
                    let content = prev.content_text();
                    Contribution {
                        source: initial.source,
                        content,
                        turn_index: initial.turn_index,
                    }
                }
            };

            // Apply the mode. For nested cascades, descend the context.
            let step_context = match &step.mode {
                ImprovMode::Cascade(_) => context.descend(),
                _ => context.clone(),
            };

            let response = step.mode.respond(&input, &step_context);
            current_response = Some(response);
        }

        // Unwrap safe: cascade has at least 1 step (enforced at construction).
        Ok(current_response.unwrap())
    }

    /// Number of steps in this cascade (not counting nested steps).
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Total number of mode applications across all nesting levels.
    /// Cascade steps count as 1 application + their inner applications.
    pub fn total_applications(&self) -> usize {
        self.steps
            .iter()
            .map(|s| match &s.mode {
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
        let cascade = ImprovCascade::new(vec![ImprovMode::Plussing, ImprovMode::YesAnd])
            .expect("2-step cascade should be valid");

        assert_eq!(cascade.step_count(), 2);
        assert_eq!(cascade.total_depth, 1); // No nested cascades.

        let contrib = make_contribution("We should improve error handling and add caching");
        let ctx = make_context();
        let result = cascade
            .execute(&contrib, &ctx)
            .expect("execution should succeed");

        // After Plussing → YesAnd, the final response should be Extended
        // (YesAnd is the last step).
        match result {
            ImprovResponse::Extended {
                accepted_base,
                extension: _,
            } => {
                // The accepted_base should be the Plussed build, not the original.
                assert!(
                    accepted_base.contains("Building on"),
                    "YesAnd should accept the Plussed output, got: {}",
                    accepted_base
                );
            }
            other => panic!(
                "Expected Extended after Plussing→YesAnd cascade, got {:?}",
                other
            ),
        }
    }

    // REQ: Cascade enforces matryoshka limit (max 7 total applications)
    #[test]
    fn enforces_matryoshka_limit() {
        // 8 simple modes — total applications = 8, exceeds limit of 7.
        let result = ImprovCascade::new(vec![
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
        ]);
        assert!(result.is_err());
        match result.unwrap_err() {
            ImprovError::MatryoshkaExceeded { depth, limit } => {
                assert_eq!(depth, 8);
                assert_eq!(limit, 7);
            }
            other => panic!("Expected MatryoshkaExceeded, got {:?}", other),
        }

        // 7 simple modes — exactly at the limit, should be valid.
        let result = ImprovCascade::new(vec![
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
            ImprovMode::Plussing,
        ]);
        assert!(result.is_ok(), "7 steps should be at the limit and valid");
    }

    // REQ: Cascade rejects empty mode list
    #[test]
    fn rejects_empty_cascade() {
        let result = ImprovCascade::new(vec![]);
        assert!(matches!(result.unwrap_err(), ImprovError::EmptyCascade));
    }

    // REQ: Cascade with nested Cascade tracks total depth correctly
    #[test]
    fn tracks_nested_depth() {
        // Inner cascade: Plussing → YesAnd (depth 1)
        let inner = ImprovCascade::new(vec![ImprovMode::Plussing, ImprovMode::YesAnd])
            .expect("inner should be valid");

        // Outer cascade: Riffing → inner Cascade (depth = 1 + inner.total_depth = 2)
        let outer = ImprovCascade::new(vec![
            ImprovMode::Riffing {
                return_policy: RiffReturn::ReturnToGroup,
            },
            ImprovMode::Cascade(inner),
        ])
        .expect("outer with nested should be valid");

        assert_eq!(outer.total_depth, 2);
        assert_eq!(outer.step_count(), 2);
        // Riffing (1) + Cascade step (1) + inner Plussing (1) + inner YesAnd (1) = 4
        assert_eq!(outer.total_applications(), 4);
    }

    // REQ: Cascade execution with nested cascade descends context
    #[test]
    fn nested_cascade_descends_context() {
        let inner = ImprovCascade::new(vec![ImprovMode::Plussing, ImprovMode::YesAnd])
            .expect("inner valid");

        let outer = ImprovCascade::new(vec![ImprovMode::Plussing, ImprovMode::Cascade(inner)])
            .expect("outer valid");

        let contrib = make_contribution("Let's refactor the auth module and add tests");
        let ctx = make_context();
        let result = outer
            .execute(&contrib, &ctx)
            .expect("execution should succeed");

        // Final step of inner cascade is YesAnd → should produce Extended.
        assert!(matches!(result, ImprovResponse::Extended { .. }));
    }

    // REQ: Deeply nested cascade exceeding matryoshka limit is rejected
    #[test]
    fn rejects_deep_nesting() {
        // Build a cascade of depth 7 (at the limit).
        let mut current = ImprovCascade::new(vec![ImprovMode::Plussing]).expect("depth 1");
        // Nest 6 more times to reach depth 7.
        for _ in 0..6 {
            current = ImprovCascade::new(vec![ImprovMode::Cascade(current)]).expect("within limit");
        }
        assert_eq!(current.total_depth, 7);

        // One more nesting should exceed the limit.
        let result = ImprovCascade::new(vec![ImprovMode::Cascade(current)]);
        assert!(result.is_err());
        match result.unwrap_err() {
            ImprovError::MatryoshkaExceeded { depth, limit } => {
                assert_eq!(depth, 8);
                assert_eq!(limit, 7);
            }
            other => panic!("Expected MatryoshkaExceeded, got {:?}", other),
        }
    }

    // REQ: Cascade execution with exceeded recursion depth at runtime is caught
    #[test]
    fn catches_runtime_recursion_exceeded() {
        let cascade = ImprovCascade::new(vec![ImprovMode::Plussing]).expect("valid");
        let contrib = make_contribution("test");
        let mut ctx = make_context();
        ctx.recursion_depth = MATRYOSHKA_LIMIT; // Already at limit.

        let result = cascade.execute(&contrib, &ctx);
        assert!(result.is_err());
        match result.unwrap_err() {
            ImprovError::RecursionExceeded { depth, limit } => {
                assert_eq!(depth, 7);
                assert_eq!(limit, 7);
            }
            other => panic!("Expected RecursionExceeded, got {:?}", other),
        }
    }
}
