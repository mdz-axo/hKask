//! Fusion orchestration engine — provider-agnostic multi-model deliberation modes.
//!
//! Each fusion mode defines how the judge interacts with the panel:
//! - BestOfN: Judge picks the single best response.
//! - Synthesis: Judge composes a unified response from all panelists.
//! - Critique: 2-round: draft → panel critique → revised final.
//! - Deliberation: Multi-round with convergence check.
//! - PlanImplement: 2-phase: strategy plan → implementation plan.
//!
//! Skills anchor the judge's reasoning with hKask's pragmatic methodology.

use crate::config::{FusionConfig, FusionMode, FusionSkill};
use crate::inference_router::InferenceRouter;
use hkask_ports::{ChatToolDefinition, InferenceError, InferenceResult};
use hkask_types::template::LLMParameters;
use tracing::info;

// ── Skill Anchor Prompts ─────────────────────────────────────────────────────

/// The compact methodology prompt injected for each anchored skill.
fn skill_prompt(skill: &FusionSkill) -> &'static str {
    match skill {
        FusionSkill::PragmaticSemantics => {
            "Pragmatic Semantics: Classify every claim by certainty level (IS vs OUGHT, \
             declarative vs probabilistic vs subjunctive). Surface unstated assumptions. \
             Flag conflation of fact and preference. Trace provenance of key claims."
        }
        FusionSkill::PragmaticCybernetics => {
            "Pragmatic Cybernetics: Identify feedback loops, measure variety, assess \
             homeostasis. Every system change must have an observable feedback mechanism. \
             Prefer closed-loop over open-loop interventions. Map control channels."
        }
        FusionSkill::PragmaticLaziness => {
            "Pragmatic Laziness: Find the path of least action. Before adding anything, \
             ask: can this be deleted? Can a simpler mechanism achieve the same result? \
             Compose from existing primitives rather than creating new ones."
        }
        FusionSkill::CodingGuidelines => {
            "Coding Guidelines (Karpathy): (1) Think before coding — surface assumptions, \
             present alternatives. (2) Simplicity first — minimum code, no speculative features. \
             (3) Surgical changes — touch only what you must, match existing style. \
             (4) Goal-driven — define verifiable success criteria, loop until verified."
        }
        FusionSkill::DeepModule => {
            "Deep Module (Ousterhout): Apply the deletion test — can this module's callers \
             be deleted without losing complexity? Interface minimalism — ≤7 public items. \
             Dependency direction — depend on what's stable, not what's convenient."
        }
        FusionSkill::Essentialist => {
            "Essentialist: Apply the 3-gate challenge loop: (1) Exist — does this artifact \
             earn its existence? (2) Surface — is its interface minimal? (3) Contract — \
             are its behavioral contracts explicit and verified?"
        }
        FusionSkill::Superforecasting => {
            "Superforecasting (Tetlock): Use Fermi decomposition for estimates. Apply \
             outside/inside views. Update probabilities with Bayesian evidence. Express \
             uncertainty as calibrated probability ranges, not binary predictions."
        }
        FusionSkill::MCDA => {
            "Multi-Criteria Decision Analysis: Identify criteria, weight and score \
             alternatives, check for compensation masking. Perform sensitivity analysis. \
             Prefer robust options that perform well across weight ranges."
        }
        FusionSkill::TestDrivenDevelopment => {
            "TDD: Red-Green-Refactor. Write the contract first (pre:/post:), then a \
             property-based test verifying it (RED), implement minimally (GREEN), \
             refactor while contracts hold. Vertical tracer-bullet: one thin slice end-to-end."
        }
    }
}

/// Build the skill anchor section of the judge's system prompt.
fn build_skill_anchor(skills: &[FusionSkill]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let mut anchor = String::from(
        "\n\n## Reasoning Framework\n\
         You are anchored on the following methodologies. Apply them in your analysis:\n\n",
    );
    for skill in skills {
        anchor.push_str(&format!("- {}\n", skill_prompt(skill)));
    }
    anchor
}

// ── Panel Dispatch ───────────────────────────────────────────────────────────

/// Result from a single panel model.
struct PanelResponse {
    model_name: String,
    text: String,
}

/// Dispatch to all panel models in parallel.
async fn dispatch_panel(
    router: &InferenceRouter,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    panel: &[String],
) -> Vec<PanelResponse> {
    use futures_util::future::join_all;

    let futures: Vec<_> = panel
        .iter()
        .map(|model_name| async move {
            let (provider, model) = match router.resolve(model_name) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        target: "cns.inference",
                        panel_model = %model_name,
                        error = %e,
                        "Panel model resolution failed"
                    );
                    return None;
                }
            };
            match router
                .dispatch_generate(provider, model, prompt, params, tools)
                .await
            {
                Ok(result) => Some(PanelResponse {
                    model_name: model_name.clone(),
                    text: result.text,
                }),
                Err(e) => {
                    tracing::warn!(
                        target: "cns.inference",
                        panel_model = %model_name,
                        error = %e,
                        "Panel model generation failed"
                    );
                    None
                }
            }
        })
        .collect();

    join_all(futures).await.into_iter().flatten().collect()
}

/// Format panel responses for judge consumption.
fn format_panel_responses(responses: &[PanelResponse]) -> String {
    let mut sections = String::new();
    for (i, resp) in responses.iter().enumerate() {
        sections.push_str(&format!(
            "\n### Panelist {}: {}\n{}\n",
            i + 1,
            resp.model_name,
            resp.text
        ));
    }
    sections
}

/// Call the judge model with a given prompt.
async fn call_judge(
    router: &InferenceRouter,
    judge_model: &str,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
) -> Result<InferenceResult, InferenceError> {
    let (judge_provider, judge_stripped) = router.resolve(judge_model)?;
    let judge_params = LLMParameters {
        bypass_fusion: true,
        ..params.clone()
    };
    router
        .dispatch_generate(judge_provider, judge_stripped, prompt, &judge_params, tools)
        .await
}

// ── Mode Implementations ─────────────────────────────────────────────────────

/// Best-of-N: Judge evaluates all panel responses and picks the single best.
async fn mode_best_of_n(
    router: &InferenceRouter,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    fusion: &FusionConfig,
) -> Result<InferenceResult, InferenceError> {
    let responses = dispatch_panel(router, prompt, params, tools, &fusion.panel).await;
    if responses.is_empty() {
        return Err(InferenceError::Generation("All panel models failed".into()));
    }

    let skill_anchor = build_skill_anchor(&fusion.skills);
    let judge_prompt = format!(
        "You are a best-of-N judge. Below are responses from {n} models to the same prompt. \
         Evaluate each response and select the single best one. Output ONLY the chosen \
         response verbatim — no commentary, no synthesis, no justification.{skills}\n\n\
         ## Original Prompt\n{prompt}\n\n## Candidate Responses{candidates}",
        n = responses.len(),
        skills = skill_anchor,
        prompt = prompt,
        candidates = format_panel_responses(&responses),
    );

    call_judge(router, &fusion.judge, &judge_prompt, params, tools).await
}

/// Synthesis: Judge composes a unified response from all panelists.
async fn mode_synthesis(
    router: &InferenceRouter,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    fusion: &FusionConfig,
) -> Result<InferenceResult, InferenceError> {
    let responses = dispatch_panel(router, prompt, params, tools, &fusion.panel).await;
    if responses.is_empty() {
        return Err(InferenceError::Generation(
            "All panel models failed — cannot synthesize".into(),
        ));
    }

    let skill_anchor = build_skill_anchor(&fusion.skills);
    let judge_prompt = format!(
        "You are a synthesis judge. Below are responses from a panel of models to the \
         same prompt. Synthesize the best answer, incorporating the strongest elements \
         from each response. Resolve any contradictions explicitly. Be concise and \
         accurate.{skills}\n\n\
         ## Original Prompt\n{prompt}\n\n## Panel Responses{candidates}",
        skills = skill_anchor,
        prompt = prompt,
        candidates = format_panel_responses(&responses),
    );

    call_judge(router, &fusion.judge, &judge_prompt, params, tools).await
}

/// Critique: 2-round — draft synthesis, panel critiques draft, judge revises.
async fn mode_critique(
    router: &InferenceRouter,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    fusion: &FusionConfig,
) -> Result<InferenceResult, InferenceError> {
    let skill_anchor = build_skill_anchor(&fusion.skills);

    // Round 1: Initial synthesis
    let r1_responses = dispatch_panel(router, prompt, params, tools, &fusion.panel).await;
    if r1_responses.is_empty() {
        return Err(InferenceError::Generation(
            "All panel models failed in round 1".into(),
        ));
    }

    let r1_judge_prompt = format!(
        "You are a synthesis judge (Round 1). Below are responses from a panel of models. \
         Produce an initial draft synthesis incorporating the strongest elements.{skills}\n\n\
         ## Original Prompt\n{prompt}\n\n## Panel Responses{candidates}\n\n\
         ## Instructions\nProduce your draft synthesis now.",
        skills = skill_anchor,
        prompt = prompt,
        candidates = format_panel_responses(&r1_responses),
    );
    let draft = call_judge(router, &fusion.judge, &r1_judge_prompt, params, tools).await?;
    let draft_text = &draft.text;

    info!(
        target: "cns.inference",
        fusion_mode = "critique",
        round = 1,
        draft_len = draft_text.len(),
        "Critique round 1 complete"
    );

    // Round 2: Panel critiques the draft
    let critique_prompt = format!(
        "You are a panelist reviewing a draft synthesis. Identify weaknesses, gaps, \
         contradictions, or improvements in the draft below. Be specific and constructive.\n\n\
         ## Original Prompt\n{prompt}\n\n## Draft Synthesis\n{draft_text}\n\n\
         ## Instructions\nProvide your critique. Focus on what the draft gets wrong, \
         misses, or could improve.",
    );
    let critiques = dispatch_panel(router, &critique_prompt, params, tools, &fusion.panel).await;

    // Round 2: Judge revises based on critiques
    let critique_sections = format_panel_responses(&critiques);
    let r2_judge_prompt = format!(
        "You are a synthesis judge (Round 2 — Final). You produced a draft synthesis. \
         The panel has reviewed it and provided critiques. Revise your synthesis, \
         incorporating the valid critiques and improving weaknesses.{skills}\n\n\
         ## Original Prompt\n{prompt}\n\n## Your Draft\n{draft_text}\n\n\
         ## Panel Critiques{critique_sections}\n\n\
         ## Instructions\nProduce your final revised synthesis.",
        skills = skill_anchor,
    );
    call_judge(router, &fusion.judge, &r2_judge_prompt, params, tools).await
}

/// Deliberation: Multi-round with convergence.
async fn mode_deliberation(
    router: &InferenceRouter,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    fusion: &FusionConfig,
) -> Result<InferenceResult, InferenceError> {
    let skill_anchor = build_skill_anchor(&fusion.skills);
    let max_rounds = fusion.max_rounds as usize;

    // Round 1: Initial panel responses
    let mut prior_responses = dispatch_panel(router, prompt, params, tools, &fusion.panel).await;
    if prior_responses.is_empty() {
        return Err(InferenceError::Generation(
            "All panel models failed in round 1".into(),
        ));
    }

    let mut prior_text = format_panel_responses(&prior_responses);

    for round in 1..=max_rounds {
        let judge_prompt = format!(
            "You are a deliberation judge (Round {round}/{max_rounds}). Below are the \
             latest responses from the panel. If the responses have converged on a \
             consistent answer, synthesize a final response. If there are still \
             significant disagreements or gaps, formulate a follow-up question to \
             resolve them.{skills}\n\n\
             ## Original Prompt\n{prompt}\n\n## Current Round Responses{prior_text}\n\n\
             ## Instructions\nIf converged: produce final synthesis.\n\
             If not converged: output ONLY a follow-up question for the panel, \
             prefixed with 'FOLLOW_UP: '.",
            round = round,
            max_rounds = max_rounds,
            skills = skill_anchor,
        );

        let judge_result = call_judge(router, &fusion.judge, &judge_prompt, params, tools).await?;

        if judge_result.text.starts_with("FOLLOW_UP:") {
            let follow_up = judge_result
                .text
                .strip_prefix("FOLLOW_UP:")
                .unwrap_or(&judge_result.text)
                .trim();
            info!(
                target: "cns.inference",
                fusion_mode = "deliberation",
                round = round,
                "Judge requested follow-up"
            );
            prior_responses = dispatch_panel(router, follow_up, params, tools, &fusion.panel).await;
            prior_text = format_panel_responses(&prior_responses);
        } else {
            info!(
                target: "cns.inference",
                fusion_mode = "deliberation",
                round = round,
                "Deliberation converged"
            );
            return Ok(judge_result);
        }
    }

    // Max rounds reached — force final synthesis
    let final_prompt = format!(
        "You are a deliberation judge (Final). Maximum rounds reached. Synthesize a \
         final response from the last round of panel discussion.{skills}\n\n\
         ## Original Prompt\n{prompt}\n\n## Final Round Responses{prior_text}\n\n\
         ## Instructions\nProduce the final synthesis now.",
        skills = skill_anchor,
    );
    call_judge(router, &fusion.judge, &final_prompt, params, tools).await
}

/// Plan-Implement: 2-phase — Phase 1: strategy plan, Phase 2: implementation plan.
async fn mode_plan_implement(
    router: &InferenceRouter,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    fusion: &FusionConfig,
) -> Result<InferenceResult, InferenceError> {
    let skill_anchor = build_skill_anchor(&fusion.skills);

    // ── Phase 1: Strategy Plan ──────────────────────────────────────────────
    let phase1_plan_prompt = format!(
        "You are a strategy panelist. Given the task below, propose a high-level \
         strategy or approach. Focus on architecture, key decisions, tradeoffs, and \
         the overall plan — NOT implementation details.\n\n\
         ## Task\n{prompt}\n\n\
         ## Instructions\nPropose a strategy. Be specific about approach, not code."
    );

    let phase1_responses =
        dispatch_panel(router, &phase1_plan_prompt, params, tools, &fusion.panel).await;
    if phase1_responses.is_empty() {
        return Err(InferenceError::Generation(
            "All panel models failed in strategy phase".into(),
        ));
    }

    let p1_judge_prompt = format!(
        "You are a strategy synthesis judge (Phase 1: Plan). Below are strategy \
         proposals from the panel. Synthesize a unified strategy plan incorporating \
         the best approaches. Resolve contradictions. This is the STRATEGY only — \
         no implementation details.{skills}\n\n\
         ## Original Task\n{prompt}\n\n## Strategy Proposals{candidates}\n\n\
         ## Instructions\nProduce the unified strategy plan.",
        skills = skill_anchor,
        candidates = format_panel_responses(&phase1_responses),
    );
    let strategy = call_judge(router, &fusion.judge, &p1_judge_prompt, params, tools).await?;
    let strategy_text = &strategy.text;

    info!(
        target: "cns.inference",
        fusion_mode = "pi",
        phase = 1,
        strategy_len = strategy_text.len(),
        "P-I Phase 1 complete — strategy synthesized"
    );

    // ── Phase 2: Implementation Plan ────────────────────────────────────────
    let phase2_impl_prompt = format!(
        "You are an implementation panelist. Below is a unified strategy plan. \
         Given this strategy, propose concrete implementation steps, file changes, \
         code structure, tests, and sequencing.\n\n\
         ## Original Task\n{prompt}\n\n## Strategy Plan\n{strategy_text}\n\n\
         ## Instructions\nPropose implementation details. Be specific about files, \
         functions, tests, and the order of work.",
    );

    let phase2_responses =
        dispatch_panel(router, &phase2_impl_prompt, params, tools, &fusion.panel).await;

    let p2_candidates = if phase2_responses.is_empty() {
        "No panelists provided implementation details.".to_string()
    } else {
        format_panel_responses(&phase2_responses)
    };

    let p2_judge_prompt = format!(
        "You are an implementation synthesis judge (Phase 2: Implement). Below is \
         the strategy plan and the panel's implementation proposals. Synthesize a \
         unified implementation plan with concrete steps, file changes, code \
         structure, tests, and sequencing.{skills}\n\n\
         ## Original Task\n{prompt}\n\n## Strategy Plan\n{strategy_text}\n\n\
         ## Implementation Proposals{p2_candidates}\n\n\
         ## Instructions\nProduce the unified implementation plan. Be specific. \
         Include: files to create/modify, key functions/types, test strategy, \
         and execution order.",
        skills = skill_anchor,
    );
    call_judge(router, &fusion.judge, &p2_judge_prompt, params, tools).await
}

// ── Public Entry Point ───────────────────────────────────────────────────────

/// Orchestrate provider-agnostic fusion deliberation.
///
/// Dispatches to the panel in parallel, then routes to the configured
/// fusion mode for judge behavior.
///
/// expect: "Fusion orchestrates multi-model deliberation provider-agnostically"
/// \[P9\] Motivating: Homeostatic Self-Regulation — hKask-side fusion orchestration
/// pre:  fusion.panel is non-empty, fusion.judge is valid
/// post: returns judge output per the configured mode
#[must_use = "result must be used"]
pub async fn orchestrate(
    router: &InferenceRouter,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    fusion: &FusionConfig,
) -> Result<InferenceResult, InferenceError> {
    info!(
        target: "cns.inference",
        fusion_mode = %fusion.mode.as_str(),
        fusion_judge = %fusion.judge,
        panel_count = fusion.panel.len(),
        skills = fusion.skills.len(),
        "Fusion orchestration starting"
    );

    match fusion.mode {
        FusionMode::BestOfN => mode_best_of_n(router, prompt, params, tools, fusion).await,
        FusionMode::Synthesis => mode_synthesis(router, prompt, params, tools, fusion).await,
        FusionMode::Critique => mode_critique(router, prompt, params, tools, fusion).await,
        FusionMode::Deliberation => mode_deliberation(router, prompt, params, tools, fusion).await,
        FusionMode::PlanImplement => {
            mode_plan_implement(router, prompt, params, tools, fusion).await
        }
    }
}
