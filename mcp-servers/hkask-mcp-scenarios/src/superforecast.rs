//! Superforecasting computation engine (Tetlock GJP methodology).
//!
//! Four-stage pipeline:
//! 1. Fermi decomposition — break forecast into sub-questions
//! 2. Outside view — base rate calibration from reference class
//! 3. Inside view — case-specific adjustments
//! 4. Bayesian updating — revise probabilities as evidence arrives
//!
//! Plus: event tree computation (conditional probability propagation)
//! and Brier scoring for calibration tracking.

use crate::types::{
    BrainstormProtocol, BrainstormRound, CalibrationBin, CalibrationCurve, CrossValidation,
    DragonflySynthesis, EventTree, EventTreeNode, ForecastOutcome, FramingDocument, PersonaConfig,
    Perspective, PhaseScore, ProjectAssessment, ScenarioError, ScenarioEvent, ScenarioType,
    StakeholderConfig, StoredForecastRecord, SubQuestion, SubQuestionDivergence, TimeHorizon,
    TriageAssessment, UseCase,
};
use std::collections::{HashMap, HashSet};

use hkask_forecast as forecast;

// ── Fermi decomposition ────────────────────────────────────────────────────

/// Fermi decomposition calibration. Delegates to shared hkask-forecast engine.
/// Weighted average of sub-question estimates by confidence.
/// Returns Err if any sub-question has non-finite estimate/confidence.
/// Returns Ok(0.5) if sub_questions is empty (neutral prior).
#[must_use = "calibration result should be used or the error handled"]
pub fn calibrate_from_fermi(sub_questions: &[SubQuestion]) -> Result<f64, ScenarioError> {
    let fqs: Vec<forecast::FermiQuestion> = sub_questions
        .iter()
        .map(|sq| forecast::FermiQuestion::new(sq.question.clone(), sq.estimate, sq.confidence))
        .collect();
    forecast::calibrate_from_fermi(&fqs).map_err(|e| match e {
        forecast::ForecastError::InvalidProbability(v, name) => {
            ScenarioError::InvalidProbability(name, v)
        }
        _ => ScenarioError::InvalidProbability("fermi".into(), 0.0),
    })
}

// ── Outside view (base rate) ───────────────────────────────────────────────

/// Compute the outside-view adjustment. Delegates to shared hkask-forecast engine.
/// Returns (calibrated_probability, confidence).
#[must_use = "adjustment should be bound"]
pub fn outside_view_adjustment(
    base_rate: f64,
    inside_estimate: f64,
    reference_count: u64,
) -> (f64, f64) {
    forecast::outside_view_adjustment(base_rate, inside_estimate, reference_count)
}

// ── Bayesian updating ──────────────────────────────────────────────────────

/// Update a calibrated probability with new evidence using Bayes' theorem.
/// Delegates to shared hkask-forecast engine.
#[must_use = "posterior should be used"]
pub fn bayesian_update(prior: f64, evidence_likelihood: f64, evidence_base_rate: f64) -> f64 {
    forecast::bayesian_update(prior, evidence_likelihood, evidence_base_rate)
}

// ── Brier scoring ──────────────────────────────────────────────────────────

/// Brier score: (probability - outcome)². Delegates to shared engine.
#[must_use = "score should be used or recorded"]
pub fn brier_score(probability: f64, outcome_occurred: bool) -> f64 {
    forecast::brier_score(probability, outcome_occurred)
}

/// Average Brier score across multiple events. Delegates to shared engine.
#[must_use = "multi-score should be used or recorded"]
pub(crate) fn brier_score_multi(
    probabilities: &[f64],
    outcomes: &[bool],
) -> Result<f64, ScenarioError> {
    forecast::brier_score_multi(probabilities, outcomes).map_err(|e| match e {
        forecast::ForecastError::BrierLengthMismatch(a, b) => {
            ScenarioError::BrierLengthMismatch(a, b)
        }
        forecast::ForecastError::BrierNoData => ScenarioError::BrierNoData,
        _ => ScenarioError::BrierNoData,
    })
}

/// Human-readable interpretation of a Brier score. Delegates to shared engine.
/// Human-readable interpretation of a Brier score. Delegates to shared engine.
pub fn brier_interpretation(score: f64) -> &'static str {
    forecast::brier_interpretation(score)
}

// ── Event tree computation ─────────────────────────────────────────────────

/// Compute marginal probabilities for all events in a dependency tree
/// via full joint conditional-table marginalization under parent independence.
///
/// Root events (no parents) use their stored probability.
/// Dependent events marginalize over the full joint truth-assignment space:
///
///   P(E) = Sum_a P(E | a) * Product_i P(p_i)^{a_i} * (1-P(p_i))^{1-a_i}
///
/// where a ranges over the 2^n bitmap of parent truth assignments,
/// and parent probabilities P(p_i) are assumed independent.
///
/// Returns a map of event_id -> resolved marginal probability.
pub(crate) fn compute_marginal_probabilities(
    events: &[ScenarioEvent],
    topo_order: &[String],
) -> HashMap<String, f64> {
    let event_map: HashMap<&str, &ScenarioEvent> =
        events.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut resolved: HashMap<String, f64> = HashMap::new();

    for id in topo_order {
        let event = match event_map.get(id.as_str()) {
            Some(e) => e,
            None => continue,
        };

        if event.depends_on.is_empty() {
            // Root node: use own probability
            resolved.insert(id.clone(), event.probability);
        } else {
            // Full joint marginalization under parent independence.
            // P(E) = Sum_a P(E|a) * Product_i P(p_i)^{a_i} * (1-P(p_i))^{1-a_i}
            let dep = &event.depends_on[0];
            let n_assignments = 1usize << dep.parent_event_ids.len();

            let parent_probs: Vec<f64> = dep
                .parent_event_ids
                .iter()
                .map(|pid| {
                    resolved.get(pid).copied().unwrap_or_else(|| {
                        tracing::warn!(
                            parent_id = %pid,
                            event_id = %id,
                            "Parent not found in resolved map; defaulting to 0.0"
                        );
                        0.0
                    })
                })
                .collect();

            let mut marginal = 0.0;
            for assignment in 0..n_assignments {
                let mut assignment_prob = 1.0;
                for (j, &p_prob) in parent_probs.iter().enumerate() {
                    let bit_set = (assignment >> j) & 1 == 1;
                    assignment_prob *= if bit_set { p_prob } else { 1.0 - p_prob };
                }
                if let Some(&cond) = dep.conditionals.get(assignment) {
                    marginal += cond * assignment_prob;
                }
            }

            resolved.insert(id.clone(), marginal.clamp(0.0, 1.0));
        }
    }

    resolved
}

/// Build a full event tree from a list of events.
/// Topologically sorts based on dependencies, computes marginal probabilities,
/// and produces an EventTree with resolved nodes.
#[must_use = "tree should be used or error inspected"]
pub fn build_event_tree(events: &[ScenarioEvent]) -> Result<EventTree, ScenarioError> {
    if events.is_empty() {
        return Err(ScenarioError::NoEvents);
    }

    // Validate all events
    for event in events {
        event.validate()?;
    }

    // Topological sort by Kahn's algorithm
    let toposort = topological_sort(events)?;

    // Identify root nodes (no dependencies)
    let root_ids: Vec<String> = events
        .iter()
        .filter(|e| e.depends_on.is_empty())
        .map(|e| e.id.clone())
        .collect();

    // Compute marginal probabilities
    let marginals = compute_marginal_probabilities(events, &toposort);

    // Build tree nodes
    let event_map: HashMap<&str, &ScenarioEvent> =
        events.iter().map(|e| (e.id.as_str(), e)).collect();

    let mut nodes: Vec<EventTreeNode> = Vec::new();
    let mut joint_prob = 1.0;

    for id in &toposort {
        let event = event_map
            .get(id.as_str())
            .ok_or_else(|| ScenarioError::EventNotFound(id.clone()))?;
        let marginal = marginals.get(id).copied().unwrap_or(event.probability);
        let joint_factor = if event.depends_on.is_empty() {
            marginal
        } else {
            // All-parents-true conditional: conditionals[last] = P(E | all parents true)
            event.depends_on[0]
                .conditionals
                .last()
                .copied()
                .unwrap_or(0.0)
        };

        // Build path from root to this node
        let paths = build_path(id, events);

        // Variance contribution: |P - 0.5| — how far from coin-flip
        let variance_contribution = (marginal - 0.5).abs() * 2.0; // scale to [0, 1]

        nodes.push(EventTreeNode {
            event: (*event).clone(),
            marginal_probability: marginal,
            paths,
            variance_contribution,
        });

        // For dependent events, the all-events-occur joint factor is
        // P(E | all parents true), drawn from the conditional table.
        joint_prob *= joint_factor;
    }

    let subject = events
        .first()
        .map(|e| e.subject.clone())
        .unwrap_or_default();
    let time_horizon = events
        .first()
        .map(|e| e.time_horizon)
        .unwrap_or(TimeHorizon::Strategic);
    let scenario_type = events
        .first()
        .map(|e| e.scenario_type)
        .unwrap_or(ScenarioType::CompanyAnalysis);

    Ok(EventTree {
        subject,
        time_horizon,
        scenario_type,
        nodes,
        root_ids,
        topo_order: toposort,
        joint_probability: joint_prob,
    })
}

/// Kahn's algorithm for topological sort of events by dependency graph.
fn topological_sort(events: &[ScenarioEvent]) -> Result<Vec<String>, ScenarioError> {
    let event_ids: Vec<String> = events.iter().map(|e| e.id.clone()).collect();
    let id_set: HashSet<&str> = event_ids.iter().map(|s| s.as_str()).collect();

    // Build adjacency list and in-degree map
    let mut in_degree: HashMap<String, u32> = event_ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut adjacency: HashMap<String, Vec<String>> = event_ids
        .iter()
        .map(|id| (id.clone(), Vec::new()))
        .collect();

    for event in events {
        for dep in &event.depends_on {
            for parent_id in &dep.parent_event_ids {
                if !id_set.contains(parent_id.as_str()) {
                    return Err(ScenarioError::UnknownParent(
                        event.id.clone(),
                        parent_id.clone(),
                    ));
                }
                adjacency.get_mut(parent_id).unwrap().push(event.id.clone());
                *in_degree.get_mut(&event.id).unwrap() += 1;
            }
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut sorted: Vec<String> = Vec::new();

    while let Some(node) = queue.pop() {
        sorted.push(node.clone());
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(child.clone());
                    }
                }
            }
        }
    }

    if sorted.len() != events.len() {
        return Err(ScenarioError::CycleDetected);
    }

    Ok(sorted)
}

/// Build all paths from root events to a given event ID.
/// For single-parent events, returns one path. For multi-parent events,
/// returns one path per parent (recursively collected).
fn build_path(target_id: &str, events: &[ScenarioEvent]) -> Vec<Vec<String>> {
    let event_map: HashMap<&str, &ScenarioEvent> =
        events.iter().map(|e| (e.id.as_str(), e)).collect();

    // Recursively collect all paths from an event to root nodes.
    fn collect_paths(
        current: &str,
        event_map: &HashMap<&str, &ScenarioEvent>,
        visited: &mut HashSet<String>,
    ) -> Vec<Vec<String>> {
        // Guard against cycles (should not happen after topological sort)
        if !visited.insert(current.to_string()) {
            return vec![vec![current.to_string()]];
        }

        let event = match event_map.get(current) {
            Some(e) => e,
            None => return vec![vec![current.to_string()]],
        };

        if event.depends_on.is_empty() {
            return vec![vec![current.to_string()]];
        }

        let mut all_paths = Vec::new();
        for dep in &event.depends_on {
            for parent_id in &dep.parent_event_ids {
                let parent_paths = collect_paths(parent_id, event_map, visited);
                for parent_path in parent_paths {
                    let mut full_path = parent_path;
                    full_path.push(current.to_string());
                    all_paths.push(full_path);
                }
            }
        }
        all_paths
    }

    let mut paths = collect_paths(target_id, &event_map, &mut HashSet::new());
    if paths.is_empty() {
        paths.push(vec![target_id.to_string()]);
    }
    paths
}

// ── Sensitivity: which events drive outcome variance ───────────────────────

/// Rank events by their contribution to outcome uncertainty.
/// Uses |P - 0.5| as a proxy — events closer to 0.5 contribute
/// more uncertainty because they're closer to a coin flip.
/// Higher score = more uncertainty.
#[must_use = "ranking result should be used"]
pub fn sensitivity_ranking(tree: &EventTree) -> Vec<(String, f64)> {
    let mut ranked: Vec<(String, f64)> = tree
        .nodes
        .iter()
        .map(|n| (n.event.id.clone(), 1.0 - n.variance_contribution))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked
}

// ── Forecast record helpers ────────────────────────────────────────────────

/// Score a forecast against known outcomes and produce a ForecastOutcome.
/// Also computes per-event update suggestions for closing the feedback loop.
pub fn score_forecast(
    forecast_id: &str,
    events: &[ScenarioEvent],
    outcomes: &[(String, bool)],
    forecast_date: chrono::NaiveDate,
) -> ForecastOutcome {
    let event_map: HashMap<&str, &ScenarioEvent> =
        events.iter().map(|e| (e.id.as_str(), e)).collect();

    let mut probs = Vec::new();
    let mut outs = Vec::new();
    let mut event_outcomes = Vec::new();

    for (event_id, occurred) in outcomes {
        if let Some(event) = event_map.get(event_id.as_str()) {
            probs.push(event.probability);
            outs.push(*occurred);
            event_outcomes.push((event_id.clone(), *occurred));
        }
    }

    let bs = brier_score_multi(&probs, &outs).unwrap_or(f64::NAN);

    ForecastOutcome {
        forecast_id: forecast_id.to_string(),
        subject: events
            .first()
            .map(|e| e.subject.clone())
            .unwrap_or_default(),
        forecast_date,
        outcome_date: chrono::Utc::now().date_naive(),
        event_outcomes,
        brier_score: bs,
        brier_interpretation: brier_interpretation(bs).to_string(),
    }
}

/// Compute per-event Bayesian update suggestions based on forecast error direction.
/// Positive delta means probability should be raised; negative means lowered.
pub fn auto_update_suggestions(
    events: &[ScenarioEvent],
    outcomes: &[(String, bool)],
) -> Vec<serde_json::Value> {
    let event_map: HashMap<&str, &ScenarioEvent> =
        events.iter().map(|e| (e.id.as_str(), e)).collect();

    outcomes
        .iter()
        .filter_map(|(event_id, occurred)| {
            let event = event_map.get(event_id.as_str())?;
            let error = event.probability - if *occurred { 1.0 } else { 0.0 };
            // Suggest a modest correction in the error's direction
            let adjustment = (-error * 0.25).clamp(-0.15, 0.15);
            let suggested = (event.probability + adjustment).clamp(0.01, 0.99);
            Some(serde_json::json!({
                "event_id": event_id,
                "event_name": event.name,
                "forecast_probability": event.probability,
                "outcome": occurred,
                "error": error,
                "suggested_adjustment": adjustment,
                "suggested_probability": suggested,
            }))
        })
        .collect()
}

// ── Brainstorming Protocol ─────────────────────────────────────────────────

/// Generate a multi-round scenario brainstorming protocol.
///
/// Produces a structured 4-round protocol with persona configurations,
/// temperature guidance, and quality gates. The agent (LLM) follows this
/// protocol to collaboratively generate events with the user.
///
/// Round 1 — DIVERGE (high temperature): Generate many candidate events
///   from multiple persona perspectives. Quantity over quality. No filtering.
///
/// Round 2 — GROUND (medium temperature): Ground each candidate in verified
///   facts. Attach base rates, reference classes, source citations.
///   Discard candidates without factual grounding.
///
/// Round 3 — LINK (low temperature): Identify dependencies between events.
///   Build causal chains. What must happen first? What enables what?
///
/// Round 4 — PRUNE (analytical): Evaluate and converge. Eliminate redundant
///   or implausible events. Merge overlapping events. Produce final tree.
pub fn generate_brainstorm_protocol(
    subject: &str,
    time_horizon: &str,
    research_context: &str,
    persona_names: &[String],
) -> BrainstormProtocol {
    let default_personas = vec![
        PersonaConfig {
            name: "Bull".into(),
            lens: "Optimistic — what could go right?".into(),
            prompt: format!(
                "You are an optimist about '{}'. What positive developments could realistically occur by {}? \
                 Focus on: technology breakthroughs, market expansion, regulatory tailwinds, competitive advantages. \
                 Each event must be a specific yes/no question with a deadline. Anchor to facts in the research context.",
                subject, time_horizon
            ),
        },
        PersonaConfig {
            name: "Bear".into(),
            lens: "Pessimistic — what could go wrong?".into(),
            prompt: format!(
                "You are a skeptic about '{}'. What negative developments could realistically occur by {}? \
                 Focus on: competitive threats, regulatory risk, supply chain disruption, market saturation, execution failure. \
                 Each event must be a specific yes/no question with a deadline. Anchor to facts in the research context.",
                subject, time_horizon
            ),
        },
        PersonaConfig {
            name: "Contrarian".into(),
            lens: "What is everyone missing?".into(),
            prompt: format!(
                "You are a contrarian thinker about '{}'. What non-obvious developments could occur by {} that most analysts are ignoring? \
                 Focus on: second-order effects, hidden assumptions, rare but high-impact events, unconventional competitors. \
                 Each event must be a specific yes/no question with a deadline. Challenge consensus views, but stay grounded.",
                subject, time_horizon
            ),
        },
        PersonaConfig {
            name: "Systems Thinker".into(),
            lens: "How do the pieces connect?".into(),
            prompt: format!(
                "You are a systems thinker analyzing '{}'. What feedback loops and cascade effects could unfold by {}? \
                 Focus on: second-order consequences, enabling conditions, bottleneck constraints, network effects. \
                 Each event should illuminate a causal mechanism — not just 'what' but 'what enables what.' \
                 Anchor to structural dynamics visible in the research context.",
                subject, time_horizon
            ),
        },
    ];

    // Use user-provided persona names if given, otherwise defaults
    let personas: Vec<PersonaConfig> = if persona_names.is_empty() {
        default_personas.clone()
    } else {
        let defaults: std::collections::HashMap<&str, PersonaConfig> = default_personas
            .iter()
            .map(|p| (p.name.as_str(), p.clone()))
            .collect();
        persona_names
            .iter()
            .filter_map(|name| {
                defaults.get(name.as_str()).cloned().or_else(|| {
                    Some(PersonaConfig {
                        name: name.clone(),
                        lens: format!("Custom perspective: {}", name),
                        prompt: format!(
                            "You are the '{}' perspective on '{}'. Generate specific yes/no events with deadlines by {}. \
                             Ground each event in the research context.",
                            name, subject, time_horizon
                        ),
                    })
                })
            })
            .collect()
    };

    let rounds = vec![
        BrainstormRound {
            round: 1,
            name: "DIVERGE — Generate Candidate Events".into(),
            mode: "diverge".into(),
            temperature_guidance: "HIGH temperature. Prioritize quantity, novelty, and range. Suspend judgment. No event is too unlikely in this round. Aim for 3-5 events per persona.".into(),
            output_type: "Vec<EventCandidate>".into(),
            instructions: format!(
                "ROUND 1: DIVERGE\n\n\
                 SUBJECT: {}\n\
                 TIME HORIZON: {}\n\n\
                 RESEARCH CONTEXT:\n{}\n\n\
                 INSTRUCTIONS:\n\
                 For each persona below, generate 3-5 candidate events as specific yes/no questions with deadlines.\n\
                 Each event must have:\n\
                 - id: persona-round-number (e.g., 'bull-1-1')\n\
                 - persona: the persona name\n\
                 - name: short descriptive name\n\
                 - question: yes/no framed with specific deadline\n\
                 - deadline_hint: approximate date\n\
                 - steep_category: Society|Technology|Economy|Environment|Politics|Industry\n\
                 - plausibility: 1-5 initial screening score\n\
                 - grounding: reference to specific fact/article in research context\n\
                 - potential_dependencies: [] (leave empty — filled in Round 3)\n\
                 - rationale: why this matters\n\n\
                 PERSONAS TO USE:\n{}",
                subject,
                time_horizon,
                research_context,
                personas
                    .iter()
                    .map(|p| format!(
                        "  {} ({}): {}\n    Prompt: {}",
                        p.name, p.lens, p.name, p.prompt
                    ))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            ),
            quality_gate: Some(
                "Before proceeding to Round 2: (1) At least 12 candidate events generated across all personas. \
                 (2) All five STEEP categories represented. (3) Each event has a specific yes/no question with deadline. \
                 (4) Each event references at least one fact from the research context.".into()
            ),
        },
        BrainstormRound {
            round: 2,
            name: "GROUND — Anchor in Verified Facts".into(),
            mode: "ground".into(),
            temperature_guidance: "MEDIUM temperature. Creative interpretation of facts is allowed, but factual grounding is required. Discard events without supporting evidence.".into(),
            output_type: "Vec<EventCandidate>".into(),
            instructions: "ROUND 2: GROUND\n\n\
                 For each candidate event from Round 1:\n\
                 1. Verify grounding: is there a specific fact, data point, or trend in the research that supports this event?\n\
                 2. Add base rate: search for a reference class. How often do events of this type occur?\n\
                    Format: 'For [reference class], the historical frequency is approximately X% over [time period]'\n\
                 3. Add Fermi sub-scaffolding: what 2-3 sub-questions would decompose this event?\n\
                 4. DISCARD any event without factual grounding. Mark as 'grounding_verified: true/false'.\n\
                 5. Adjust plausibility score based on evidence strength.\n\n\
                 QUALITY GATE: All retained events must have verified grounding. \
                 Minimum 8 events retained. If fewer, return to Round 1 with different personas.".to_string(),
            quality_gate: Some(
                "All retained events have verified grounding (specific fact/article/data point). \
                 At least 8 events retained. Each event has a reference class suggestion. \
                 Fermi sub-questions provided for each event.".into()
            ),
        },
        BrainstormRound {
            round: 3,
            name: "LINK — Build Causal Chains".into(),
            mode: "link".into(),
            temperature_guidance: "LOW temperature. Focus on logical structure, not creativity. Causal links must be defensible.".into(),
            output_type: "Vec<ScenarioEvent>".into(),
            instructions: "ROUND 3: LINK\n\n\
                 For the grounded events from Round 2:\n\
                 1. Identify dependency relationships: which events must happen before others?\n\
                 2. Build causal chains: A → B → C. For each dependency, estimate:\n\
                    - P(B | A occurs) — probability of B if A happens\n\
                    - P(B | A does NOT occur) — probability of B if A doesn't happen\n\
                 3. Check for cycles: if A depends on B and B depends on A, resolve the direction.\n\
                 4. Identify root events (no dependencies) and leaf events (nothing depends on them).\n\
                 5. Convert candidates to full ScenarioEvent objects with:\n\
                    - Formal deadline (YYYY-MM-DD)\n\
                    - Initial probability estimate (0.0-1.0)\n\
                    - depends_on array with conditional probabilities\n\
                    - sub_questions from Round 2\n\
                    - base_rate and reference_class from grounding research\n\n\
                 QUALITY GATE: All events must have at least one dependency or be a root event. \
                 No cycles in the dependency graph. Events form a coherent causal narrative.".to_string(),
            quality_gate: Some(
                "No cycles in dependency graph. Every non-root event has at least one dependency. \
                 Conditional probabilities (P(event|parent), P(event|¬parent)) provided for all dependencies.".into()
            ),
        },
        BrainstormRound {
            round: 4,
            name: "PRUNE — Converge to Final Tree".into(),
            mode: "prune".into(),
            temperature_guidance: "ANALYTICAL. Ruthless pruning. If two events are nearly identical, merge them. If an event has no dependencies and no path to a consequential outcome, remove it.".into(),
            output_type: "Vec<ScenarioEvent>".into(),
            instructions: "ROUND 4: PRUNE\n\n\
                 For the linked events from Round 3:\n\
                 1. Merge overlapping events: if two events describe essentially the same thing, combine them.\n\
                 2. Remove isolated events: if an event has no dependencies AND no events depend on it, \
                    consider whether it belongs in this scenario tree.\n\
                 3. Check for completeness: do the events collectively tell a coherent story? \
                    Are there obvious gaps (no regulatory events? no competitive events?)?\
                 4. Final probability calibration: use Fermi decomposition to estimate initial probability \
                    for each remaining event.\n\
                 5. Produce final output: a JSON array of ScenarioEvent objects ready for scenario_quantify.\n\n\
                 Send this output to scenario_quantify for conditional probability resolution.\n\n\
                 QUALITY GATE: 4-8 events remain. All events are connected (directly or transitively) to at \
                 least one other event or to a consequential outcome. STEEP coverage maintained. \
                 All events have Fermi-calibrated probabilities.".to_string(),
            quality_gate: Some(
                "4-8 events remain in final tree. No isolated events. STEEP coverage maintained. \
                 All events have calibrated probabilities. Dependencies form a causal narrative. \
                 Ready to send to scenario_quantify.".into()
            ),
        },
    ];

    BrainstormProtocol {
        subject: subject.to_string(),
        time_horizon: time_horizon.to_string(),
        research_context: research_context.to_string(),
        personas,
        rounds,
        pipeline: vec![
            "1. scenario_brainstorm → get this protocol".into(),
            "2. [agent follows protocol rounds 1-4]".into(),
            "3. scenario_quantify → resolve conditional probability tree".into(),
            "4. scenario_calibrate → Fermi decomposition per event".into(),
            "5. scenario_synthesize → dragonfly-eye aggregation (if multiple analysts)".into(),
            "6. scenario_assess → evaluate project quality (Chermack)".into(),
        ],
    }
}

// ── Scenario Framing Protocol ──────────────────────────────────────────────

/// Generate a conversational framing protocol for scenario project setup.
///
/// Designed with behavioral psychology principles and improv coaching
/// postures to make framing approachable rather than diagnostic.
///
/// Design principles:
/// - Foot-in-the-door: start easy, build to harder questions
/// - Never explicitly negate (improv Plussing): build on what works
/// - Yes, And: accept the user's answer and extend naturally
/// - Curiosity gap: create intrigue before asking for commitment
/// - Peak-end rule: open warmly, close with clarity
/// - Self-determination: the user is the domain expert; the agent is the method coach
/// - Processing fluency: conversational language, no technical jargon
///
/// The 7 conversational turns replace the formal "7 questions" —
/// each turn is a natural opening, not a numbered test item.
///
/// References:
/// - Chermack (2011), Ch. 5: Project Preparation Phase
/// - Schwartz (1991), Ch. 4: Focal Question
/// - Kahneman (2011): System 1/System 2, loss aversion, peak-end rule
/// - Cialdini (2006): Influence — foot-in-the-door, social proof
/// - Ryan & Deci (2000): Self-determination theory
/// - hKask improv skill: Plussing, Yes And, Yes But postures
/// - hKask kata-starter: coaching posture, 20-minute practice window
pub fn generate_framing_session(subject: &str) -> serde_json::Value {
    serde_json::json!({
        "session_type": "Conversational Scenario Framing",
        "subject": subject,
        "design_principles": {
            "why_conversational": "Framing is where scenario projects usually break down. Formal diagnostic questions create resistance. Conversational turns invite engagement. The goal isn't to extract answers — it's to help the user discover their own frame.",
            "improv_posture": "Plussing by default: accept what the user says, build on what's useful, silently let go of what isn't. Never correct. Never 'no, but.' Always 'yes, and...'",
            "kata_coaching": "The agent is a coach, not an interviewer. The user is the domain expert. The coach helps the expert articulate what they already know but haven't yet made explicit.",
            "behavioral_design": [
                "Foot-in-the-door: Turn 1 is the easiest — 'what's on your mind?' Anyone can answer that.",
                "Curiosity gap: Turns 2-3 build intrigue before asking for commitment.",
                "Peak-end rule: Turn 1 opens warmly; Turn 7 closes with clarity and purpose.",
                "Loss aversion: Turn 4 asks what's OFF the table (easier to identify exclusions).",
                "Social proof: Turn 5 uses 'who else' to normalize multiple perspectives.",
                "Processing fluency: Everyday language throughout. No 'focal question,' no 'epistemic calibration.'",
                "IKEA effect: The user co-creates the frame. They own it because they built it."
            ]
        },

        "conversation_flow": [
            {
                "turn": 1,
                "improv_mode": "Plussing",
                "psychology": "Foot-in-the-door — the easiest question, no wrong answer",
                "opening": "So — tell me a bit about what's on your mind. What situation are you looking at?",
                "why_this_comes_first": "Everyone can answer this. It establishes the user as the domain expert and the agent as the curious listener. No jargon, no pressure, no right answer.",
                "what_to_listen_for": "The subject, the emotional stakes, what makes this situation interesting or urgent. Don't correct or narrow — just let them talk.",
                "agent_posture": "Listen actively. Reflect back what you heard. 'So it sounds like you're looking at [X] and wondering about [Y].' Use Plussing: affirm what's clear, gently surface what's fuzzy without calling it fuzzy.",
                "anti_patterns": [
                    "Jumping to 'so what's your focal question?' — that comes later",
                    "Correcting scope — 'that's too broad' — instead ask 'what part of that feels most uncertain?'",
                    "Solving the problem — the agent's job is to frame, not to answer"
                ],
                "captures": "subject, initial_context, emotional_stakes"
            },
            {
                "turn": 2,
                "improv_mode": "Yes, And",
                "psychology": "Curiosity gap — connect their situation to a decision",
                "opening": "That's really interesting. If you had a clearer picture of how this might play out — what would you actually do differently? What decision is hanging on this?",
                "why_this_comes_second": "Schwartz's rule: if the answer doesn't change any decision, don't spend time on it. But asking 'what is your focal question?' is clinical. This framing connects the situation they just described to the decision it informs. It makes the purpose personal.",
                "what_to_listen_for": "Is there a real decision at stake? If they say 'I just want to understand' — that's fine for landscape exploration, but note it. If they name a specific decision, that's gold — it becomes the focal question.",
                "agent_posture": "Yes, And: 'So the decision is [X], and what makes it hard is [Y].' Don't push for a single sentence focal question yet — that comes after they've explored.",
                "anti_patterns": [
                    "'That's not really a focal question' — never negate. Instead: 'That's a great starting point. Let's see if we can make it even more specific.'",
                    "Accepting 'I just want to understand everything' without probing: 'Totally fair. Is there a particular fork in the road where understanding would change your path?'"
                ],
                "captures": "decision_at_stake, focal_question_draft"
            },
            {
                "turn": 3,
                "improv_mode": "Coaching (kata-style)",
                "psychology": "Temporal anchoring — the kata coach asks 'what is the target condition?'",
                "opening": "Got it. So looking ahead — when do you actually need to make this call? And over what kind of timeframe do the key events play out? Sometimes the decision deadline and the event horizon are different. Like, you might need to decide in three months about things that won't fully play out for three years.",
                "why_this_comes_third": "Now that we have a decision and a situation, we need a temporal boundary. The kata coaching pattern works here: 'what is the target condition?' (when do you need to decide?) followed by 'what is the actual condition now?' (what timeframe are the events on?).",
                "what_to_listen_for": "Distinguish decision deadline from event horizon. If they're different, note both. If the user says 'I don't know' — that's information too. Suggest: tactical (12-18mo), strategic (3-5yr), long-term (7-10yr) as reference points, not as a multiple-choice test.",
                "agent_posture": "Coach, not quizmaster. 'Most people find it helpful to think in terms of tactical (12-18 months), strategic (3-5 years), or long-term (7-10 years). Where does your situation land?' Present options as scaffolding, not as a test.",
                "anti_patterns": [
                    "'You need to pick one of these three time horizons' — don't force categorization",
                    "Overspecifying: 'So exactly 42 months?' — approximate is fine at this stage"
                ],
                "captures": "time_horizon, action_deadline"
            },
            {
                "turn": 4,
                "improv_mode": "Yes, But (constraint focus)",
                "psychology": "Loss aversion — people find it easier to identify what's excluded than what's included",
                "opening": "Helpful. Now let's draw some boundaries — and let's start with what's definitely NOT on the table. What are we explicitly not trying to figure out here? What's somebody else's problem, or a different project, or just not relevant right now?",
                "why_this_comes_fourth": "Loss aversion: people engage more to avoid loss than to seek gain. Asking 'what's out of scope' is easier and more energizing than 'what's in scope.' Once exclusions are clear, the scope naturally emerges. Schwartz: scope-bounded is essential; without boundaries, everything is relevant.",
                "what_to_listen_for": "Explicit boundaries. If the user says 'well, everything is relevant actually' — that's a red flag. Gently probe: 'What's one thing that's NOT relevant?' Even one exclusion is progress.",
                "agent_posture": "Yes, But: 'Okay, so [X], [Y], and [Z] are off the table. Given that, what IS on the table?' The constraint clarifies without contradicting.",
                "anti_patterns": [
                    "Leading with 'what's in scope?' — that's the harder question. Start with exclusions.",
                    "Accepting 'everything' without pushing back — if everything is relevant, nothing is actionable"
                ],
                "captures": "out_of_scope, in_scope"
            },
            {
                "turn": 5,
                "improv_mode": "Plussing (multi-perspective)",
                "psychology": "Social proof + contrarian activation",
                "opening": "Let's think about who else has skin in this game. If this goes wrong — or right — who's going to have a strong opinion about it? And here's a fun one: if it goes wrong, who's the person who's going to say 'I told you so' — and what would they have seen that others missed?",
                "why_this_comes_fifth": "Chermack: stakeholder diversity is the strongest predictor of scenario quality. But 'who are the stakeholders?' is bureaucratic. The 'I told you so' framing activates social dynamics — it makes the question playful and memorable. The contrarian perspective surfaces naturally without having to ask for it explicitly.",
                "what_to_listen_for": "Names, roles, perspectives. The 'I told you so' person is the most valuable — they represent the perspective most likely to be overlooked. Each stakeholder becomes a persona in the brainstorming phase.",
                "agent_posture": "Plussing: 'So we've got [A] who cares about [X], [B] who's watching [Y], and [C] who'd say I told you so about [Z]. That's a great set of lenses. Anyone else?' Build the list collaboratively.",
                "anti_patterns": [
                    "Treating this as a formal stakeholder analysis — keep it conversational",
                    "Forgetting to ask 'who would say I told you so?' — this is the most valuable question in the protocol"
                ],
                "captures": "stakeholders (name, primary_concern, likely_blind_spots, include_as_persona)"
            },
            {
                "turn": 6,
                "improv_mode": "Yes, And (forward-looking)",
                "psychology": "Peak-end rule begins — shift from exploration to commitment",
                "opening": "This is really coming together. So when we're done — when we've built the scenarios and worked through the probabilities — what does 'good enough' look like? What would make you look back and say 'that was worth the time'?",
                "why_this_comes_sixth": "Chermack's core contribution: define success criteria before building scenarios. But 'define assessment criteria' is sterile. 'What does good enough look like?' is human. The peak-end rule says the closing moments matter most — this turn begins the close by shifting from exploration ('what's possible?') to commitment ('what would make this worthwhile?').",
                "what_to_listen_for": "Concrete, observable criteria — not vague aspirations. 'We'd identify risks we hadn't seen before' is better than 'we'd feel more confident.' Also note the use case: are they building a monitoring dashboard? An investment thesis? A strategic decision framework?",
                "agent_posture": "Yes, And: 'So success looks like [X], [Y], and [Z]. AND let me add one more dimension — how will you actually USE the output? Is this going to be something you check quarterly, or something that informs a one-time decision, or...?'",
                "anti_patterns": [
                    "Accepting 'we'll just know if it was useful' — that's not scorable. Ask: 'What would you point to as evidence?'",
                    "Skipping the use case question — the format depends on the use case"
                ],
                "captures": "success_criteria, use_case"
            },
            {
                "turn": 7,
                "improv_mode": "Yes, But (closing with clarity)",
                "psychology": "Peak-end rule closes — provocative but supportive",
                "opening": "Last thing — and this is the one that keeps scenario planners up at night. What are we assuming right now that might turn out to be completely wrong? Not the obvious stuff — the quiet assumptions. The things we're taking for granted that, if they broke, would make this whole exercise irrelevant. And while we're at it — what constraints are we working within? Time, people, information we can't access?",
                "why_this_comes_last": "Chermack: hidden assumptions are the primary source of scenario error. But 'surface your assumptions' is abstract. 'What keeps scenario planners up at night' creates intrigue. Asking about constraints at the end is deliberate — after 6 turns, the user trusts the process enough to be honest about limitations. The peak-end rule: end with a memorable, slightly provocative question that opens up rather than closes down.",
                "what_to_listen_for": "Assumptions they're uncomfortable voicing — those are the most valuable. Constraints they're reluctant to name — those define the real boundary. If they say 'no assumptions, we've thought of everything' — gently note that's the most dangerous assumption of all.",
                "agent_posture": "Supportive but direct: 'This is the hard one, and it's okay if the answers aren't complete. The point isn't to be right about our assumptions — it's to know what they are so we can watch for when they break.'",
                "anti_patterns": [
                    "Rushing through this turn because it's the last one — this is where the most valuable information lives",
                    "Accepting 'no constraints' without a gentle probe — 'Unlimited time and perfect information? I want your job.'"
                ],
                "captures": "surfaced_assumptions, constraints, exploration_prompts"
            }
        ],

        "framing_document_template": {
            "focal_question": "<synthesized from turns 1-2>",
            "decision_at_stake": "<from turn 2 — what changes based on what we learn?>",
            "time_horizon": "<tactical|strategic|long_term — from turn 3>",
            "action_deadline": "<from turn 3 — when decision is needed>",
            "in_scope": ["<from turn 4>"],
            "out_of_scope": ["<from turn 4>"],
            "stakeholders": [{
                "role": "<from turn 5>",
                "primary_concern": "<what does this person care about?>",
                "likely_blind_spots": ["<what might they miss?>"],
                "include_as_persona": true
            }],
            "use_case": "<strategic_decision|investment_thesis|monitoring_dashboard|landscape_exploration|contingency_planning — from turn 6>",
            "success_criteria": ["<from turn 6>"],
            "constraints": ["<from turn 7>"],
            "surfaced_assumptions": ["<from turn 7>"],
            "exploration_prompts": ["<generated from turns 1+5 — what specific questions should personas explore?>"]
        },

        "after_framing": {
            "next_step": "The framing conversation naturally leads into brainstorming. The stakeholders from Turn 5 become personas. The exploration prompts from Turns 2+5 guide the divergent phase. The scope boundaries from Turn 4 keep the tree focused. Flow directly into scenario_brainstorm.",
            "pipeline": [
                "scenario_frame → conversational framing (this tool)",
                "scenario_brainstorm → multi-persona temperature-shifting protocol",
                "scenario_quantify → resolve conditional probability tree",
                "scenario_calibrate → Fermi decomposition + outside view",
                "scenario_synthesize → dragonfly-eye aggregation",
                "scenario_assess → evaluate against Turn 6 success criteria"
            ]
        },

        "agent_guidance": {
            "overall_posture": "Coach, not interviewer. Socratic but warm. The user is the domain expert; you are the method expert. Your job is to help them articulate what they already know but haven't yet made explicit.",
            "improv_rules": [
                "Never explicitly negate. 'That's interesting — let's dig into that' not 'That's too broad.'",
                "Yes, And: accept their answer and extend it naturally.",
                "Plussing: amplify what's clear, gently let go of what's fuzzy without calling it out.",
                "If the conversation is flowing, don't interrupt to ask the next question. Let the turns blend.",
                "If a turn produces a rich answer, stay there. The numbered turns are a scaffold, not a script."
            ],
            "pacing": {
                "target_duration": "15-20 minutes for all 7 turns",
                "if_stuck": "If a turn produces silence, reframe. 'Let me ask it differently...' Don't skip. Don't fill the silence for them.",
                "if_flowing": "If the user is on a roll, let them go. The turns are a guide, not a straitjacket. Capture insights wherever they land.",
                "too_fast": "If all 7 turns are done in 5 minutes, the framing is probably too shallow. Slow down. Ask follow-ups."
            },
            "when_to_redirect": [
                "Turn 2: The user describes a situation with no decision attached. Ask: 'What would you do differently if you knew the answer?'",
                "Turn 4: The user says everything is in scope. Ask: 'What's one thing that's definitely NOT relevant?'",
                "Turn 7: The user says they have no assumptions. Note: 'That's interesting — and it's actually the most common answer. Let me ask it differently: what would surprise you most if it turned out to be wrong?'",
                "Any turn: The user starts solving the problem instead of framing it. Gently: 'We'll get to that. First, let's make sure we're asking the right question.'"
            ],
            "minimalist_principle": "Seven conversational turns. 15-20 minutes. If a question doesn't change the scenario output, it doesn't belong in the conversation. The framing exists to make brainstorming productive — not to produce a document."
        },

        "references": {
            "chermack_2011": "Scenario Planning in Organizations — Phase 1: Project Preparation",
            "schwartz_1991": "The Art of the Long View — Stage 1: Focal Question",
            "kahneman_2011": "Thinking, Fast and Slow — System 1/2, loss aversion, peak-end rule",
            "cialdini_2006": "Influence: The Psychology of Persuasion — foot-in-the-door, social proof",
            "ryan_deci_2000": "Self-Determination Theory — autonomy, competence, relatedness",
            "hkask_improv": "Improv skill — Plussing, Yes And, Yes But postures",
            "hkask_kata": "Kata-Starter skill — coaching posture, 5 Questions Drill pattern"
        }
    })
}
/// Structure a completed framing conversation into a FramingDocument.
/// Takes the subject and a JSON blob of conversation answers, validates them,
/// and produces a typed FramingDocument suitable for feeding into scenario_brainstorm.
pub fn structure_framing_document(
    subject: &str,
    answers: &serde_json::Value,
) -> Result<FramingDocument, ScenarioError> {
    if subject.trim().is_empty() {
        return Err(ScenarioError::InvalidProbability("subject".into(), 0.0));
    }
    let get_str = |key: &str| -> String {
        answers
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let get_list = |key: &str| -> Vec<String> {
        answers
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };

    let time_horizon = match get_str("time_horizon").to_lowercase().as_str() {
        "tactical" | "12-18 months" | "tactical (12-18 months)" => TimeHorizon::Tactical,
        "strategic" | "3-5 years" | "strategic (3-5 years)" => TimeHorizon::Strategic,
        "long-term" | "7-10 years" | "long-term (7-10 years)" => TimeHorizon::LongTerm,
        _ => TimeHorizon::Strategic,
    };

    let use_case = match get_str("use_case").to_lowercase().as_str() {
        "strategic decision" | "strategic_decision" => UseCase::StrategicDecision,
        "investment thesis" | "investment_thesis" => UseCase::InvestmentThesis,
        "monitoring dashboard" | "monitoring_dashboard" => UseCase::MonitoringDashboard,
        "landscape exploration" | "landscape_exploration" => UseCase::LandscapeExploration,
        "contingency planning" | "contingency_planning" => UseCase::ContingencyPlanning,
        _ => UseCase::LandscapeExploration,
    };

    let stakeholders: Vec<StakeholderConfig> = answers
        .get("stakeholders")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|s| StakeholderConfig {
                    role: s
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    primary_concern: s
                        .get("primary_concern")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    likely_blind_spots: s
                        .get("likely_blind_spots")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    include_as_persona: s
                        .get("include_as_persona")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(FramingDocument {
        focal_question: get_str("focal_question"),
        decision_at_stake: get_str("decision_at_stake"),
        time_horizon,
        action_deadline: {
            let d = get_str("action_deadline");
            if d.is_empty() { None } else { Some(d) }
        },
        in_scope: get_list("in_scope"),
        out_of_scope: get_list("out_of_scope"),
        stakeholders,
        use_case,
        success_criteria: get_list("success_criteria"),
        constraints: get_list("constraints"),
        surfaced_assumptions: get_list("surfaced_assumptions"),
        exploration_prompts: get_list("exploration_prompts"),
    })
}

// ── Chermack Project Assessment (P5) ──────────────────────────────────────

/// Assess a scenario project across Chermack's five performance phases.
///
/// Evaluates whether the scenario project was worth doing — not just
/// whether forecasts were accurate. Combines quantitative metrics
/// (Brier scores, disagreement, calibration) with qualitative assessment
/// of preparation, exploration, implementation, and learning.
///
/// Reference: Chermack, T.J. (2011). Scenario Planning in Organizations:
/// How to Create, Use, and Assess Scenarios. Berrett-Koehler.
#[allow(clippy::too_many_arguments)]
pub fn assess_project(
    project_id: &str,
    subject: &str,
    perspective_count: usize,
    disagreement_score: f64,
    event_count: usize,
    events_with_deps: usize,
    calibration_curve: Option<&CalibrationCurve>,
    strategies_generated: usize,
    strategies_implemented: usize,
    learning_events: Vec<String>,
    has_early_warning_indicators: bool,
) -> ProjectAssessment {
    // ── Phase 1: Preparation ──────────────────────────────────────
    // (Chermack, Ch. 5): Scope clarity, stakeholder engagement, resource allocation
    let prep_score = if perspective_count >= 3 {
        0.8
    } else if perspective_count >= 2 {
        0.6
    } else {
        0.3
    };
    let mut prep_strengths = Vec::new();
    let mut prep_gaps = Vec::new();
    if perspective_count >= 3 {
        prep_strengths.push("Multiple perspectives engaged".into());
    } else if perspective_count == 0 {
        prep_gaps.push(
            "No perspectives recorded — project may lack stakeholder engagement (Chermack Phase 1)"
                .into(),
        );
    } else {
        prep_gaps.push(format!("Only {} perspective(s) — consider engaging more diverse viewpoints (Chermack: stakeholder dialogue)", perspective_count));
    }

    // ── Phase 2: Exploration ─────────────────────────────────────
    // (Chermack, Ch. 6): Driving forces identified, trends mapped, uncertainties surfaced
    let exp_score = if event_count >= 5 && disagreement_score > 0.1 {
        0.75
    } else if event_count >= 3 {
        0.5
    } else {
        0.2
    };
    let mut exp_strengths = Vec::new();
    let mut exp_gaps = Vec::new();
    if disagreement_score > 0.2 {
        exp_strengths.push(format!("Significant disagreement ({:.0}%) detected — healthy diversity of views (Chermack: conversation quality)", disagreement_score * 100.0));
    }
    if event_count >= 5 {
        exp_strengths.push(format!(
            "{} events identified — comprehensive force mapping",
            event_count
        ));
    } else {
        exp_gaps.push(format!(
            "Only {} events — consider deeper STEEP force mapping",
            event_count
        ));
    }
    if disagreement_score < 0.05 && event_count > 0 {
        exp_gaps.push("Very low disagreement — potential groupthink. Chermack warns against false consensus in scenario exploration.".into());
    }

    // ── Phase 3: Development ─────────────────────────────────────
    // (Chermack, Ch. 7): Scenario logic, internal consistency, narrative quality
    let dep_ratio = if event_count > 0 {
        events_with_deps as f64 / event_count as f64
    } else {
        0.0
    };
    let dev_score = if dep_ratio > 0.3 && event_count >= 4 {
        0.8
    } else if dep_ratio > 0.1 {
        0.5
    } else {
        0.3
    };
    let mut dev_strengths = Vec::new();
    let mut dev_gaps = Vec::new();
    if dep_ratio > 0.3 {
        dev_strengths.push(format!("{:.0}% of events have conditional dependencies — structured causal reasoning (Chermack: internal consistency)", dep_ratio * 100.0));
    } else {
        dev_gaps.push("Most events lack dependency links. Chermack requires internal consistency: events should form a causal chain, not a list.".into());
    }
    if event_count < 4 {
        dev_gaps.push("Fewer than 4 events — scenarios may lack sufficient structure for meaningful narratives.".into());
    }

    // ── Phase 4: Implementation ──────────────────────────────────
    // (Chermack, Ch. 8): Strategies applied, wind-tunneling, early warning systems
    let impl_score = if strategies_implemented > 0 && has_early_warning_indicators {
        0.85
    } else if strategies_generated > 0 {
        0.5
    } else {
        0.1
    };
    let mut impl_strengths = Vec::new();
    let mut impl_gaps = Vec::new();
    if strategies_implemented > 0 {
        impl_strengths.push(format!(
            "{} strategies implemented — scenario insights drove action (Chermack Phase 4)",
            strategies_implemented
        ));
    }
    if strategies_generated > 0 && strategies_implemented == 0 {
        impl_gaps.push(format!("{} strategies generated but none implemented — the scenario-to-action gap (Chermack's critical Phase 4)", strategies_generated));
    }
    if !has_early_warning_indicators {
        impl_gaps.push("No early warning indicators defined. Chermack: scenarios without tripwires are stories without sensors.".into());
    }

    // ── Phase 5: Project Assessment ──────────────────────────────
    // (Chermack, Ch. 9): Did the project improve decision quality? Learning outcomes?
    let assess_score = if !learning_events.is_empty()
        && calibration_curve.is_some_and(|c| c.resolved_forecasts >= 5)
    {
        0.8
    } else if !learning_events.is_empty() {
        0.5
    } else {
        0.2
    };
    let mut assess_strengths = Vec::new();
    let mut assess_gaps = Vec::new();
    if !learning_events.is_empty() {
        assess_strengths.push(format!("{} learning events recorded — evidence of mental model change (Chermack: organizational learning)", learning_events.len()));
    } else {
        assess_gaps.push("No learning events recorded. Chermack's key metric: did the project change how participants think?".into());
    }
    if let Some(curve) = calibration_curve {
        if curve.resolved_forecasts >= 10 {
            assess_strengths.push(format!(
                "{} resolved forecasts — sufficient data for calibration assessment",
                curve.resolved_forecasts
            ));
        } else if curve.resolved_forecasts > 0 {
            assess_gaps.push(format!(
                "Only {} resolved forecasts — need ≥10 for reliable calibration assessment",
                curve.resolved_forecasts
            ));
        }
    } else {
        assess_gaps.push("No calibration data. Chermack + Tetlock: without outcome tracking, you cannot know if the project improved forecast accuracy.".into());
    }

    // ── Composite ─────────────────────────────────────────────────
    let overall = (prep_score + exp_score + dev_score + impl_score + assess_score) / 5.0;

    let assessment_text = if overall >= 0.7 {
        "Strong scenario project. Preparation was thorough, exploration surfaced diverse views, scenarios are causally structured, insights drove action, and learning is being tracked. Continue deepening the calibration loop."
    } else if overall >= 0.5 {
        "Adequate scenario project with room for improvement. Strengthen the weakest phases (see per-phase gaps below). Focus on closing the implementation gap: scenarios without action are entertainment."
    } else if overall >= 0.3 {
        "Foundational scenario project. Core elements are present but significant gaps remain. Priority: engage more perspectives (Phase 1), add conditional dependencies (Phase 3), and track outcomes (Phase 5)."
    } else {
        "Early-stage scenario project. The scaffolding exists but lacks depth. Start with Phase 1 (preparation): define the focal question clearly and engage multiple perspectives before building scenarios."
    };

    let mut recommendations = Vec::new();
    if prep_score < 0.6 {
        recommendations.push("Phase 1 (Preparation): Engage at least 3 diverse perspectives. Chermack: 'The quality of the conversation determines the quality of the scenarios.'".into());
    }
    if exp_score < 0.6 {
        recommendations.push("Phase 2 (Exploration): Map more driving forces. Use scenario_research to gather external data. Chermack: systematic STEEP analysis prevents blind spots.".into());
    }
    if dev_score < 0.6 {
        recommendations.push("Phase 3 (Development): Link events with conditional dependencies. Scenarios must form causal chains, not lists. Chermack: internal consistency is the quality gate.".into());
    }
    if impl_score < 0.6 {
        recommendations.push("Phase 4 (Implementation): Define early-warning indicators and track which strategies get implemented. Chermack: 'Scenario planning without implementation is intellectual tourism.'".into());
    }
    if assess_score < 0.6 {
        recommendations.push("Phase 5 (Assessment): Record learning events and track calibration. Use scenario_score to resolve forecasts and scenario_calibration to measure improvement over time.".into());
    }

    ProjectAssessment {
        project_id: project_id.to_string(),
        subject: subject.to_string(),
        preparation: PhaseScore {
            phase: "Phase 1: Preparation".into(),
            score: prep_score,
            strengths: prep_strengths,
            gaps: prep_gaps,
        },
        exploration: PhaseScore {
            phase: "Phase 2: Exploration".into(),
            score: exp_score,
            strengths: exp_strengths,
            gaps: exp_gaps,
        },
        development: PhaseScore {
            phase: "Phase 3: Development".into(),
            score: dev_score,
            strengths: dev_strengths,
            gaps: dev_gaps,
        },
        implementation: PhaseScore {
            phase: "Phase 4: Implementation".into(),
            score: impl_score,
            strengths: impl_strengths,
            gaps: impl_gaps,
        },
        project_assessment: PhaseScore {
            phase: "Phase 5: Project Assessment".into(),
            score: assess_score,
            strengths: assess_strengths,
            gaps: assess_gaps,
        },
        overall_score: overall,
        overall_assessment: assessment_text.to_string(),
        learning_evidence: learning_events,
        recommendations,
    }
}
// ── Dragonfly-Eye Synthesis (P1) ──────────────────────────────────────────

/// Synthesize multiple independent perspectives on an event into one
/// aggregated probability with disagreement scoring.
///
/// Uses empirical-Bayes weighting: perspectives with lower historical Brier
/// scores get higher weight. If no historical scores are available, all
/// perspectives are weighted equally.
///
/// Returns an error if fewer than 2 perspectives are provided.
pub fn synthesize_perspectives(
    event_id: &str,
    perspectives: &[Perspective],
) -> Result<DragonflySynthesis, ScenarioError> {
    if perspectives.len() < 2 {
        return Err(ScenarioError::InsufficientPerspectives);
    }

    // Compute weights: inverse-Brier if available, else uniform
    let has_historical = perspectives.iter().any(|p| p.historical_brier.is_some());

    let weights: Vec<f64> = if has_historical {
        let raw: Vec<f64> = perspectives
            .iter()
            .map(|p| {
                let brier = p.historical_brier.unwrap_or(0.25);
                1.0 / (brier + 0.01)
            })
            .collect();
        let total: f64 = raw.iter().sum();
        raw.iter().map(|w| w / total).collect()
    } else {
        let w = 1.0 / perspectives.len() as f64;
        vec![w; perspectives.len()]
    };

    // Weighted average probability
    let aggregated: f64 = perspectives
        .iter()
        .zip(weights.iter())
        .map(|(p, w)| p.probability * w)
        .sum();

    // Disagreement score: normalized standard deviation
    let mean = perspectives.iter().map(|p| p.probability).sum::<f64>() / perspectives.len() as f64;
    let variance = perspectives
        .iter()
        .map(|p| (p.probability - mean).powi(2))
        .sum::<f64>()
        / perspectives.len() as f64;
    let disagreement = (variance / 0.25).sqrt().min(1.0);

    // Identify dissenting perspective
    let (dissent_idx, _) = perspectives
        .iter()
        .enumerate()
        .map(|(i, p)| (i, (p.probability - aggregated).abs()))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, 0.0));

    let dissent_summary = if disagreement > 0.3 {
        perspectives.get(dissent_idx).and_then(|p| {
            p.rationale.as_ref().map(|r| {
                format!(
                    "Dissenting view ({}: {:.0}%): {}",
                    p.source,
                    p.probability * 100.0,
                    r
                )
            })
        })
    } else {
        None
    };

    let quality = if disagreement < 0.1 {
        "high_consensus"
    } else if disagreement < 0.3 {
        "moderate_consensus"
    } else if disagreement < 0.5 {
        "significant_disagreement"
    } else {
        "polarized"
    };

    let perspective_weights: Vec<(String, f64)> = perspectives
        .iter()
        .zip(weights.iter())
        .map(|(p, w)| (p.source.clone(), *w))
        .collect();

    Ok(DragonflySynthesis {
        event_id: event_id.to_string(),
        perspectives: perspectives.to_vec(),
        aggregated_probability: aggregated,
        disagreement_score: disagreement,
        dissent_summary,
        perspective_weights,
        synthesis_quality: quality.to_string(),
    })
}

// ── Calibration Tracking ────────────────────────────────────────────────────

/// Compute a calibration curve from stored forecasts.
pub fn compute_calibration_curve(store: &ForecastStore) -> Result<CalibrationCurve, ScenarioError> {
    let resolved: Vec<&StoredForecastRecord> = store.resolved();

    if resolved.is_empty() {
        return Err(ScenarioError::NoForecastData);
    }

    let mut bins: Vec<(u64, u64, f64)> = vec![(0, 0, 0.0); 10];
    let mut total_brier = 0.0;

    for record in &resolved {
        let occurred = record.outcome.unwrap_or(false);
        let bin_idx = ((record.probability * 10.0) as usize).min(9);
        bins[bin_idx].0 += 1;
        if occurred {
            bins[bin_idx].1 += 1;
        }
        bins[bin_idx].2 += record.probability;
        total_brier += brier_score(record.probability, occurred);
    }

    let n = resolved.len() as f64;
    let overall_brier = total_brier / n;

    let calibration_bins: Vec<CalibrationBin> = bins
        .iter()
        .enumerate()
        .map(|(i, &(count, hits, probability_sum))| {
            let low = i as f64 * 0.1;
            let high = (i + 1) as f64 * 0.1;
            let hit_rate = if count > 0 {
                hits as f64 / count as f64
            } else {
                f64::NAN
            };
            let expected = if count > 0 {
                probability_sum / count as f64
            } else {
                (low + high) / 2.0
            };
            CalibrationBin {
                probability_range: format!("{:.0}–{:.0}%", low * 100.0, high * 100.0),
                forecast_count: count,
                hit_rate,
                expected_rate: expected,
                bias: if count > 0 { expected - hit_rate } else { 0.0 },
            }
        })
        .collect();

    let mut weighted_bias = 0.0;
    let mut bias_weight = 0.0;
    for bin in &calibration_bins {
        if bin.forecast_count >= 5 {
            weighted_bias += bin.bias * bin.forecast_count as f64;
            bias_weight += bin.forecast_count as f64;
        }
    }
    let overconfidence = if bias_weight > 0.0 {
        weighted_bias / bias_weight
    } else {
        0.0
    };

    let interpretation = if overconfidence > 0.10 {
        "systematically_overconfident"
    } else if overconfidence < -0.10 {
        "systematically_underconfident"
    } else if overconfidence.abs() < 0.05 {
        "well_calibrated"
    } else {
        "moderately_calibrated"
    };

    Ok(CalibrationCurve {
        bins: calibration_bins,
        total_forecasts: store.len() as u64,
        resolved_forecasts: resolved.len() as u64,
        overall_brier,
        overconfidence_score: overconfidence,
        interpretation: interpretation.to_string(),
    })
}

// ── Triage (P4) ────────────────────────────────────────────────────────────

/// Triage a forecasting question to determine if it's worth the full pipeline.
#[must_use = "triage result should be used"]
pub fn triage_question(
    question: &str,
    has_deadline: bool,
    has_reference_class: bool,
    has_resolution_criteria: bool,
) -> TriageAssessment {
    let word_count = question.split_whitespace().count();
    let has_specifics = word_count > 5;

    let clarity = if has_deadline && has_specifics {
        0.8
    } else if has_deadline || has_specifics {
        0.5
    } else {
        0.2
    };

    let data_avail = if has_reference_class { 0.8 } else { 0.3 };
    let resolution = if has_resolution_criteria { 0.9 } else { 0.2 };

    let overall = (clarity + data_avail + resolution) / 3.0;

    let (difficulty, recommend, forecastable) = if overall >= 0.7 {
        (
            "clocklike",
            "Well-specified with clear resolution criteria. Simple base-rate extrapolation may suffice — consider whether the full superforecasting pipeline is worth the effort.",
            true,
        )
    } else if overall >= 0.4 {
        (
            "goldilocks",
            "In the Goldilocks zone — difficult enough to reward careful analysis, specific enough to be scored. Run the full pipeline: Fermi decomposition → outside view → Bayesian updating.",
            true,
        )
    } else {
        (
            "cloudlike",
            "Too vague or lacks clear resolution criteria. Refine: add a specific deadline, define what counts as 'yes', and identify a reference class.",
            false,
        )
    };

    TriageAssessment {
        question: question.to_string(),
        is_forecastable: forecastable,
        difficulty: difficulty.to_string(),
        clarity_score: clarity,
        data_availability_score: data_avail,
        resolution_criteria_clarity: resolution,
        recommendation: recommend.to_string(),
    }
}

// ── Persistence ──────────────────────────────────────────────────────────

use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// File-backed persistence using append-only journal + periodic snapshot compaction.
/// Each mutation appends one JSON line to the journal (O(1) write). On load, the
/// snapshot is loaded first, then journal entries are replayed on top (last write wins).
/// After JOURNAL_COMPACT_THRESHOLD entries, the journal is compacted into a full snapshot.
const JOURNAL_COMPACT_THRESHOLD: usize = 100;

#[derive(Debug, Default)]
pub struct ForecastStore {
    pub records: HashMap<String, StoredForecastRecord>,
    pub data_path: Option<PathBuf>,
    journal_path: Option<PathBuf>,
    journal_count: usize,
}

impl ForecastStore {
    /// Create a new store, loading snapshot + journal replay from disk.
    pub fn new(data_path: Option<PathBuf>) -> Self {
        let journal_path = data_path.as_ref().map(|p| {
            let mut jp = p.clone();
            jp.set_extension("json.journal");
            jp
        });
        let mut store = Self {
            records: HashMap::new(),
            data_path,
            journal_path,
            journal_count: 0,
        };
        store.load();
        store
    }

    /// Load: snapshot first, then replay journal on top (last write wins).
    fn load(&mut self) {
        if let Some(ref path) = self.data_path
            && path.exists()
            && let Ok(data) = fs::read_to_string(path)
            && let Ok(records) =
                serde_json::from_str::<HashMap<String, StoredForecastRecord>>(&data)
        {
            self.records = records;
        }
        if let Some(ref jp) = self.journal_path
            && jp.exists()
            && let Ok(data) = fs::read_to_string(jp)
        {
            for line in data.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_str::<serde_json::Value>(trimmed)
                    && let (Some(key), Some(record)) = (
                        entry.get("key").and_then(|v| v.as_str()),
                        entry.get("record"),
                    )
                    && let Ok(rec) = serde_json::from_value::<StoredForecastRecord>(record.clone())
                {
                    self.records.insert(key.to_string(), rec);
                    self.journal_count += 1;
                }
            }
        }
    }

    /// Append a single record entry to the journal (O(1) write per mutation).
    /// Only writes the changed record, not the full dataset.
    fn save_entry(&self, key: &str, record: &StoredForecastRecord) {
        if let (Some(jp), Some(dp)) = (&self.journal_path, &self.data_path) {
            if let Some(parent) = dp.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(jp)
                && let Ok(line) = serde_json::to_string(&serde_json::json!({
                    "key": key,
                    "record": record
                }))
            {
                let _ = writeln!(file, "{}", line);
            }
        }
    }

    /// Insert a record and persist via single-entry journal append.
    pub fn insert(&mut self, key: String, record: StoredForecastRecord) {
        self.save_entry(&key, &record);
        self.records.insert(key, record);
        self.journal_count += 1;
        if self.journal_count >= JOURNAL_COMPACT_THRESHOLD {
            self.compact();
        }
    }

    pub fn get(&self, key: &str) -> Option<&StoredForecastRecord> {
        self.records.get(key)
    }

    /// Get mutable reference. Caller must call persist() after modification.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut StoredForecastRecord> {
        self.records.get_mut(key)
    }

    /// Persist all changes (writes full snapshot, truncates journal).
    pub fn persist(&self) {
        self.compact();
    }

    /// Compact: write full snapshot, truncate journal.
    fn compact(&self) {
        if let Some(ref dp) = self.data_path {
            if let Some(parent) = dp.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(data) = serde_json::to_string_pretty(&self.records) {
                let _ = fs::write(dp, data);
                if let Some(ref jp) = self.journal_path {
                    let _ = fs::write(jp, "");
                }
            }
        }
    }

    /// Force compaction regardless of threshold.
    pub fn force_compact(&self) {
        self.compact();
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns `true` if the forecast store contains no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &StoredForecastRecord> {
        self.records.values()
    }

    pub fn resolved(&self) -> Vec<&StoredForecastRecord> {
        self.records
            .values()
            .filter(|r| r.outcome.is_some())
            .collect()
    }

    pub(crate) fn filtered_by_subject(&self, subject: &str) -> Self {
        Self {
            records: self
                .records
                .iter()
                .filter(|(_, record)| record.subject == subject)
                .map(|(key, record)| (key.clone(), record.clone()))
                .collect(),
            data_path: None,
            journal_path: None,
            journal_count: 0,
        }
    }
}

// ── Cross-Validation ──────────────────────────────────────────────────────

/// Cross-validate two probability estimates for the same event.
///
/// Typically compares an LLM-generated estimate (from the superforecasting
/// skill) against a server-computed estimate (from scenario_calibrate).
///
/// Computes per-sub-question divergence to identify where the estimates
/// differ most. Flags for review when overall divergence exceeds the
/// threshold (default 0.15).
///
/// This closes the learning loop between LLM reasoning and computational
/// verification — the key bridge between the superforecasting skill and
/// the scenarios MCP server.
#[allow(clippy::too_many_arguments)]
#[must_use = "validation result should be inspected"]
pub fn cross_validate(
    event_id: &str,
    source_a: &str,
    estimate_a: f64,
    sub_questions_a: &[SubQuestion],
    source_b: &str,
    estimate_b: f64,
    sub_questions_b: &[SubQuestion],
    threshold: Option<f64>,
) -> CrossValidation {
    let review_threshold = threshold.unwrap_or(0.15);
    let divergence = (estimate_a - estimate_b).abs();
    let requires_review = divergence > review_threshold;

    // Match sub-questions by index (best-effort alignment)
    let max_sq = sub_questions_a.len().max(sub_questions_b.len());
    let mut sq_divergences = Vec::new();
    for i in 0..max_sq {
        let sq_a = sub_questions_a.get(i);
        let sq_b = sub_questions_b.get(i);
        let question = sq_a
            .map(|s| s.question.as_str())
            .or_else(|| sq_b.map(|s| s.question.as_str()))
            .unwrap_or("unknown");
        let est_a = sq_a.map(|s| s.estimate).unwrap_or(0.5);
        let est_b = sq_b.map(|s| s.estimate).unwrap_or(0.5);
        let sq_div = (est_a - est_b).abs();
        sq_divergences.push(SubQuestionDivergence {
            question: question.to_string(),
            estimate_a: est_a,
            estimate_b: est_b,
            divergence: sq_div,
        });
    }

    let recommendation = if !requires_review {
        format!(
            "Estimates are consistent (divergence {:.3} <= threshold {:.3}). No review needed.",
            divergence, review_threshold
        )
    } else {
        let max_sq_div = sq_divergences
            .iter()
            .map(|d| d.divergence)
            .fold(0.0_f64, f64::max);
        let top_sq = sq_divergences
            .iter()
            .max_by(|a, b| {
                a.divergence
                    .partial_cmp(&b.divergence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|d| d.question.as_str())
            .unwrap_or("unknown");
        format!(
            "Estimates diverge ({:.3} > {:.3}). Largest sub-question divergence ({:.3}) on '{}'. Activate grill-me skill.",
            divergence, review_threshold, max_sq_div, top_sq
        )
    };

    let grill_me_questions: Vec<String> = if requires_review {
        let mut questions = vec![format!(
            "What hidden assumptions could explain the {:.1}% divergence between '{}' and '{}' on event '{}'?",
            divergence * 100.0,
            source_a,
            source_b,
            event_id
        )];
        for sq in sq_divergences.iter().take(3) {
            if sq.divergence > 0.05 {
                questions.push(format!(
                    "Sub-question '{}': why does {} estimate {:.0}% while {} estimates {:.0}%?",
                    sq.question,
                    source_a,
                    sq.estimate_a * 100.0,
                    source_b,
                    sq.estimate_b * 100.0
                ));
            }
        }
        questions
    } else {
        Vec::new()
    };

    CrossValidation {
        event_id: event_id.to_string(),
        estimate_a,
        source_a: source_a.to_string(),
        estimate_b,
        source_b: source_b.to_string(),
        divergence,
        requires_review,
        review_threshold,
        sub_question_divergences: sq_divergences,
        recommendation,
        grill_me_questions,
    }
}

// ── Companies Server Bridge ────────────────────────────────────────────────

/// Convert a companies server calibrate_forecast output into ScenarioEvents
/// that can be quantified by the scenarios pipeline.
///
/// The companies server produces Schwartz 2×2 scenario results with
/// intrinsic values per quadrant. This function converts those into
/// binomial events with Fermi sub-questions, ready for scenario_quantify
/// and scenario_calibrate.
///
/// Bridge path: companies.calibrate_forecast → this function → scenario_quantify → scenario_synthesize
pub fn convert_companies_output(
    symbol: &str,
    companies_json: &serde_json::Value,
    time_horizon: TimeHorizon,
) -> Result<Vec<ScenarioEvent>, ScenarioError> {
    let scenarios = companies_json
        .get("scenarios")
        .and_then(|s| s.as_array())
        .ok_or(ScenarioError::NoEvents)?;

    let mut events = Vec::new();
    // Derive deadline from time horizon
    let today = chrono::Utc::now().date_naive();
    let deadline = match time_horizon {
        TimeHorizon::Tactical => today + chrono::TimeDelta::days(540),
        TimeHorizon::Strategic => today + chrono::TimeDelta::days(1460),
        TimeHorizon::LongTerm => today + chrono::TimeDelta::days(2920),
    };

    for (i, scenario) in scenarios.iter().enumerate() {
        let name = scenario
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");
        let intrinsic = scenario
            .get("intrinsic_per_share")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let current_price = companies_json
            .get("current_price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let upside = if current_price > 0.0 {
            (intrinsic - current_price) / current_price
        } else {
            0.0
        };

        let question = format!(
            "Will {} trade within 20% of the {} scenario intrinsic value ({:.2}) by {}",
            symbol,
            name.to_lowercase(),
            intrinsic,
            deadline.format("%Y-%m-%d")
        );

        // Probability from scenario analysis (simplified: equal weight with upside signal)
        let prob = if upside > 0.2 {
            0.65 // strong upside → higher probability
        } else if upside > 0.0 {
            0.55
        } else if upside > -0.2 {
            0.40
        } else {
            0.25 // strong downside → lower probability
        };

        let growth = scenario.get("applied_growth").and_then(|v| v.as_f64());
        let margin = scenario.get("applied_margin").and_then(|v| v.as_f64());

        let mut sub_questions = Vec::new();
        if let Some(g) = growth {
            sub_questions.push(SubQuestion {
                question: format!("Will revenue growth reach {:.0}%?", g * 100.0),
                estimate: if g > 0.1 { 0.6 } else { 0.4 },
                confidence: 0.5,
            });
        }
        if let Some(m) = margin {
            sub_questions.push(SubQuestion {
                question: format!("Will gross margins hold at {:.0}%?", m * 100.0),
                estimate: if m > 0.4 { 0.6 } else { 0.4 },
                confidence: 0.5,
            });
        }

        events.push(ScenarioEvent {
            id: format!("comp-{}-{}", symbol, i),
            name: format!("{} {}", symbol, name),
            question,
            deadline,
            time_horizon,
            scenario_type: ScenarioType::CompanyAnalysis,
            subject: symbol.to_string(),
            probability: prob,
            basis: Some("financial_model".into()),
            depends_on: vec![],
            sub_questions,
            base_rate: None,
            reference_class: Some("Company DCF scenario analysis, 2×2 Schwartz matrix".into()),
            brier_score: None,
            update_count: 0,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CertaintyTier;
    use crate::types::EventDependency;

    fn make_event(id: &str, prob: f64, deps: Vec<EventDependency>) -> ScenarioEvent {
        ScenarioEvent {
            id: id.into(),
            name: format!("Event {}", id),
            question: format!("Will {} occur?", id),
            deadline: chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            time_horizon: TimeHorizon::Strategic,
            scenario_type: ScenarioType::CompanyAnalysis,
            subject: "TEST".into(),
            probability: prob,
            basis: None,
            depends_on: deps,
            sub_questions: vec![],
            base_rate: None,
            reference_class: None,
            brier_score: None,
            update_count: 0,
        }
    }

    #[test]
    fn test_calibrate_from_fermi_simple() {
        let sqs = vec![
            SubQuestion {
                question: "a".into(),
                estimate: 0.8,
                confidence: 0.9,
            },
            SubQuestion {
                question: "b".into(),
                estimate: 0.2,
                confidence: 0.1,
            },
        ];
        let result = calibrate_from_fermi(&sqs).unwrap();
        assert!((result - 0.74).abs() < 0.001);
    }

    #[test]
    fn test_calibrate_empty_returns_neutral() {
        assert_eq!(calibrate_from_fermi(&[]).unwrap(), 0.5);
    }

    #[test]
    fn test_calibrate_nan_rejected() {
        let sqs = vec![SubQuestion {
            question: "nan".into(),
            estimate: f64::NAN,
            confidence: 0.5,
        }];
        let result = calibrate_from_fermi(&sqs);
        assert!(result.is_err());
    }

    #[test]
    fn test_calibrate_inf_rejected() {
        let sqs = vec![SubQuestion {
            question: "inf".into(),
            estimate: f64::INFINITY,
            confidence: 0.5,
        }];
        let result = calibrate_from_fermi(&sqs);
        assert!(result.is_err());
    }

    #[test]
    fn test_outside_view_high_reference_count() {
        let (prob, conf) = outside_view_adjustment(0.7, 0.3, 1000);
        assert!(prob > 0.6);
        assert!(conf > 0.7);
    }

    #[test]
    fn test_outside_view_low_reference_count() {
        let (prob, _conf) = outside_view_adjustment(0.9, 0.5, 1);
        assert!((prob - 0.55).abs() < 0.01);
    }

    #[test]
    fn test_bayesian_update_positive_evidence() {
        let posterior = bayesian_update(0.3, 0.9, 0.3);
        assert!((posterior - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_bayesian_update_with_negative_evidence() {
        let posterior = bayesian_update(0.7, 0.1, 0.4);
        assert!((posterior - 0.175).abs() < 0.01);
    }

    #[test]
    fn test_brier_perfect() {
        assert_eq!(brier_score(1.0, true), 0.0);
        assert_eq!(brier_score(0.0, false), 0.0);
    }

    #[test]
    fn test_brier_worst() {
        assert_eq!(brier_score(0.0, true), 1.0);
        assert_eq!(brier_score(1.0, false), 1.0);
    }

    #[test]
    fn test_brier_mid() {
        assert_eq!(brier_score(0.5, true), 0.25);
        assert_eq!(brier_score(0.5, false), 0.25);
    }

    #[test]
    fn test_brier_multi_ok() {
        let result = brier_score_multi(&[0.8, 0.2], &[true, false]).unwrap();
        // (0.8-1)^2=0.04, (0.2-0)^2=0.04, avg=0.04
        assert!((result - 0.04).abs() < 0.001);
    }

    #[test]
    fn test_brier_multi_mismatch_err() {
        let result = brier_score_multi(&[0.8], &[true, false]);
        assert!(result.is_err());
    }

    #[test]
    fn test_brier_multi_empty_err() {
        let result = brier_score_multi(&[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_brier_interpretation_excellent() {
        assert_eq!(brier_interpretation(0.03), "excellent");
    }

    #[test]
    fn test_brier_interpretation_worse() {
        assert_eq!(brier_interpretation(0.5), "worse_than_climatology");
    }

    #[test]
    fn test_event_tree_no_deps() {
        let events = vec![make_event("A", 0.8, vec![]), make_event("B", 0.6, vec![])];
        let tree = build_event_tree(&events).unwrap();
        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.root_ids.len(), 2);
        assert!((tree.joint_probability - 0.48).abs() < 0.01);
    }

    #[test]
    fn test_event_tree_with_dependency() {
        let dep = vec![EventDependency {
            parent_event_ids: vec!["A".into()],
            conditionals: vec![0.2, 0.9], // [P(E|not A), P(E|A)]
        }];
        let events = vec![make_event("A", 0.5, vec![]), make_event("B", 0.7, dep)];
        let tree = build_event_tree(&events).unwrap();
        assert_eq!(tree.nodes.len(), 2);
        let b_node = tree.nodes.iter().find(|n| n.event.id == "B").unwrap();
        assert!((b_node.marginal_probability - 0.55).abs() < 0.01);
        assert!((tree.joint_probability - 0.45).abs() < 0.01);
    }

    #[test]
    fn test_event_tree_cycle_detection() {
        let dep_a = vec![EventDependency {
            parent_event_ids: vec!["B".into()],
            conditionals: vec![0.3, 0.8],
        }];
        let dep_b = vec![EventDependency {
            parent_event_ids: vec!["A".into()],
            conditionals: vec![0.3, 0.8],
        }];
        let events = vec![make_event("A", 0.5, dep_a), make_event("B", 0.5, dep_b)];
        let result = build_event_tree(&events);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScenarioError::CycleDetected));
    }

    #[test]
    fn test_event_tree_multi_parent_independence() {
        // Two independent root events A and B, with child C depending on both.
        // P(A) = 0.5, P(B) = 0.8
        // Bitmap convention: bit j = parent_event_ids[j] (0=A, 1=B)
        //   conditionals[0b00] = P(C | ¬A, ¬B) = 0.05
        //   conditionals[0b01] = P(C |  A, ¬B) = 0.30
        //   conditionals[0b10] = P(C | ¬A,  B) = 0.40
        //   conditionals[0b11] = P(C |  A,  B) = 0.90
        //
        // Under parent independence:
        //   P(¬A,¬B) = 0.10, P(A,¬B) = 0.10, P(¬A,B) = 0.40, P(A,B) = 0.40
        // P(C) = 0.05*0.10 + 0.30*0.10 + 0.40*0.40 + 0.90*0.40 = 0.555
        // Joint P(all) = P(A) * P(B) * P(C | A=true, B=true) = 0.5 * 0.8 * 0.90 = 0.36
        let dep_c = vec![EventDependency {
            parent_event_ids: vec!["A".into(), "B".into()],
            // bitmap: 00=¬A¬B, 01=A¬B, 10=¬AB, 11=AB
            conditionals: vec![0.05, 0.30, 0.40, 0.90],
        }];
        let events = vec![
            make_event("A", 0.5, vec![]),
            make_event("B", 0.8, vec![]),
            make_event("C", 0.3, dep_c),
        ];
        let tree = build_event_tree(&events).unwrap();
        assert_eq!(tree.nodes.len(), 3);
        assert_eq!(tree.root_ids.len(), 2);

        let c_node = tree.nodes.iter().find(|n| n.event.id == "C").unwrap();
        let expected_marginal = 0.555;
        assert!(
            (c_node.marginal_probability - expected_marginal).abs() < 0.001,
            "P(C) = {} expected {} under independence",
            c_node.marginal_probability,
            expected_marginal
        );

        let expected_joint = 0.36;
        assert!(
            (tree.joint_probability - expected_joint).abs() < 0.001,
            "joint = {} expected {}",
            tree.joint_probability,
            expected_joint
        );
    }

    #[test]
    fn test_sensitivity_ranking() {
        let events = vec![
            make_event("A", 0.5, vec![]),  // max uncertainty (coin flip)
            make_event("B", 0.99, vec![]), // near certainty
        ];
        let tree = build_event_tree(&events).unwrap();
        let ranked = sensitivity_ranking(&tree);
        assert_eq!(ranked[0].0, "A");
        assert_eq!(ranked[1].0, "B");
    }

    #[test]
    fn test_validate_nan_rejected() {
        let event = make_event("A", f64::NAN, vec![]);
        let result = event.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_inf_rejected() {
        let event = make_event("A", f64::INFINITY, vec![]);
        let result = event.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_wrong_conditionals_length_rejected() {
        // 2 parents require 4 conditionals, but we provide only 3
        let dep = EventDependency {
            parent_event_ids: vec!["A".into(), "B".into()],
            conditionals: vec![0.1, 0.3, 0.7], // should be length 4
        };
        let event = make_event("C", 0.5, vec![dep]);
        let result = event.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_calibration_bias_uses_mean_forecast_probability() {
        let mut store = ForecastStore::default();
        for (id, probability, outcome) in [
            ("a", 0.81, false),
            ("b", 0.83, false),
            ("c", 0.85, false),
            ("d", 0.87, false),
            ("e", 0.89, false),
        ] {
            store.insert(
                id.into(),
                StoredForecastRecord {
                    schema_version: 1,
                    forecast_id: "forecast".into(),
                    event_id: id.into(),
                    event_name: id.into(),
                    subject: "test".into(),
                    probability,
                    created_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                    outcome: Some(outcome),
                    resolved_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
                },
            );
        }

        let curve = compute_calibration_curve(&store).unwrap();
        let bin = curve
            .bins
            .iter()
            .find(|bin| bin.forecast_count == 5)
            .unwrap();

        assert!((bin.expected_rate - 0.85).abs() < f64::EPSILON);
        assert!((bin.bias - 0.85).abs() < f64::EPSILON);
        assert!(curve.overconfidence_score > 0.0);
        assert_eq!(curve.interpretation, "systematically_overconfident");
    }

    #[test]
    fn test_auto_update_suggestions_correct_direction() {
        let events = vec![make_event("A", 0.3, vec![])];
        let outcomes = vec![("A".into(), true)]; // event occurred but forecast was 30%
        let suggestions = auto_update_suggestions(&events, &outcomes);
        assert_eq!(suggestions.len(), 1);
        let adj = suggestions[0]["suggested_adjustment"].as_f64().unwrap();
        assert!(adj > 0.0); // should suggest raising probability
    }

    // ── E5: CertaintyTier boundary tests ──────────────────────────────────

    #[test]
    fn test_certainty_tier_exact_boundaries() {
        // Exact boundary values: Proximate ≥ 0.67, Probable in [0.33, 0.67), Possible < 0.33
        assert!(
            matches!(
                CertaintyTier::from_probability(0.67),
                CertaintyTier::Proximate
            ),
            "0.67 should be Proximate boundary (≥ 0.67)"
        );
        assert!(
            matches!(
                CertaintyTier::from_probability(0.33),
                CertaintyTier::Probable
            ),
            "0.33 should be Probable (≥ 0.33 ∧ < 0.67)"
        );
        assert!(
            matches!(
                CertaintyTier::from_probability(0.329),
                CertaintyTier::Possible
            ),
            "0.329 should be Possible (< 0.33)"
        );
        assert!(
            matches!(
                CertaintyTier::from_probability(0.669),
                CertaintyTier::Probable
            ),
            "0.669 should be Probable (< 0.67)"
        );
    }

    #[test]
    fn test_certainty_tier_range() {
        assert_eq!(CertaintyTier::Proximate.range(), "67–100%");
        assert_eq!(CertaintyTier::Probable.range(), "33–66%");
        assert_eq!(CertaintyTier::Possible.range(), "0–32%");
    }

    #[test]
    fn test_certainty_tier_edges() {
        assert!(
            matches!(
                CertaintyTier::from_probability(0.0),
                CertaintyTier::Possible
            ),
            "0.0 should be Possible"
        );
        assert!(
            matches!(
                CertaintyTier::from_probability(1.0),
                CertaintyTier::Proximate
            ),
            "1.0 should be Proximate"
        );
        assert!(
            matches!(
                CertaintyTier::from_probability(0.5),
                CertaintyTier::Probable
            ),
            "0.5 should be Probable (middle of range)"
        );
    }

    // ── Persistence round-trip tests ──────────────────────────────────────

    #[test]
    fn test_journal_insert_and_reload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("forecasts.json");

        // Phase 1: Insert records
        {
            let mut store = ForecastStore::new(Some(path.clone()));
            store.insert(
                "fcst-1:evt-A".into(),
                StoredForecastRecord {
                    schema_version: 1,
                    forecast_id: "fcst-1".into(),
                    event_id: "evt-A".into(),
                    event_name: "Event A".into(),
                    subject: "TEST".into(),
                    probability: 0.75,
                    created_at: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    outcome: None,
                    resolved_at: None,
                },
            );
            store.insert(
                "fcst-1:evt-B".into(),
                StoredForecastRecord {
                    schema_version: 1,
                    forecast_id: "fcst-1".into(),
                    event_id: "evt-B".into(),
                    event_name: "Event B".into(),
                    subject: "TEST".into(),
                    probability: 0.30,
                    created_at: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    outcome: Some(true),
                    resolved_at: Some(chrono::NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
                },
            );
            store.force_compact(); // ensure snapshot is written
        }

        // Phase 2: Reload from disk
        {
            let store = ForecastStore::new(Some(path));
            assert_eq!(store.len(), 2, "both records should survive restart");

            let a = store.get("fcst-1:evt-A").expect("evt-A should exist");
            assert_eq!(a.probability, 0.75);
            assert!(a.outcome.is_none());

            let b = store.get("fcst-1:evt-B").expect("evt-B should exist");
            assert_eq!(b.probability, 0.30);
            assert_eq!(b.outcome, Some(true));
            assert_eq!(store.resolved().len(), 1);
        }
    }
}
