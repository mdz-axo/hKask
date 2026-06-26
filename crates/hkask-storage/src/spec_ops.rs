//! Spec operations — pure business logic for spec capture, decomposition,
//! writing-quality assessment, graph query, and coherence analysis.
//!
//! All functions are pure — no I/O, no async, no side effects.
//! Storage access, CNS spanning, and inference are handled by callers.

use crate::spec_types::{GoalSpec, Spec, SpecCategory};

// ── Shared output types ──────────────────────────────────────────────────

/// A dependency edge between two sub-goals: `from` must complete before `to`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
}

/// Heuristic writing quality score (Hopper, Lovelace, Schriver, Gentle).
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct HeuristicWritingQuality {
    pub hopper: bool,
    pub lovelace: bool,
    pub schriver: bool,
    pub gentle: bool,
}

impl HeuristicWritingQuality {
    pub fn passes(&self) -> usize {
        [self.hopper, self.lovelace, self.schriver, self.gentle]
            .iter()
            .filter(|&&p| p)
            .count()
    }

    pub fn meets_publication_standard(&self) -> bool {
        self.passes() >= 3
    }
}

/// A node in the spec graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub category: String,
}

/// An edge in the spec graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

/// A path through the spec graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphPath {
    pub nodes: Vec<String>,
    pub length: usize,
}

/// Result of querying the spec graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphQueryResult {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub paths: Vec<GraphPath>,
}

/// Result of a coherence check on a spec collection.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoherenceCheck {
    pub coherence_score: f64,
    pub violations: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Writing quality computation from embedding dimension scores.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmbeddingQualityResult {
    pub dimensions_passing: usize,
    pub meets_standard: bool,
    pub weakest_dimension: Option<String>,
    pub rewrite_prompt: Option<String>,
}

// ── OCAP boundary extraction ─────────────────────────────────────────────

/// Extract OCAP boundary hints from context keywords.
///
/// This is pure string matching — no storage access.
///
pub fn extract_ocap_boundaries(context: Option<&str>) -> Vec<String> {
    let ctx = match context {
        Some(c) => c.to_lowercase(),
        None => return vec![],
    };
    let mut boundaries = Vec::new();
    if ctx.contains("curation") || ctx.contains("curat") {
        boundaries.push("curation".to_string());
    }
    if ctx.contains("cybernetics") || ctx.contains("cns") {
        boundaries.push("cybernetics".to_string());
    }
    if ctx.contains("spec_curate") || ctx.contains("spec curate") {
        boundaries.push("spec_curate".to_string());
    }
    boundaries
}

// ── Goal decomposition ───────────────────────────────────────────────────

/// Decompose a description into sub-goals by splitting on sentence boundaries.
///
/// Returns sub-goal texts and sequential dependency edges (each sub-goal
/// depends on the previous one).
///
pub fn decompose_description(description: &str) -> (Vec<String>, Vec<DependencyEdge>) {
    let sub_goals: Vec<String> = description
        .split(['.', '\n'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let mut dependencies = Vec::new();
    for i in 1..sub_goals.len() {
        dependencies.push(DependencyEdge {
            from: sub_goals[i - 1].clone(),
            to: sub_goals[i].clone(),
        });
    }

    (sub_goals, dependencies)
}

// ── Writing quality (heuristic) ──────────────────────────────────────────

/// Assess a spec's writing quality using a structural heuristic.
///
/// Four dimensions: Hopper (zero-context accessibility), Lovelace (testable
/// precision), Schriver (30-second findability), Gentle (agent-correctness).
///
/// This is the fast, no-embedding check. For embedding-based comparison,
/// callers must supply their own embedding infrastructure.
///
pub fn assess_writing_quality_heuristic(spec: &Spec) -> HeuristicWritingQuality {
    let has_description = !spec.name.is_empty();
    let has_goals = !spec.goals.is_empty();
    let has_criteria = spec.goals.iter().any(|g| !g.criteria.is_empty());
    let has_verbs = !spec.declared_verbs.is_empty();

    HeuristicWritingQuality {
        hopper: has_goals && has_criteria,
        lovelace: has_criteria,
        schriver: has_description && has_goals,
        gentle: has_description && has_verbs,
    }
}

// ── Embedding quality computation ────────────────────────────────────────

/// Compute pass/fail and weakest-dimension from embedding-based dimension scores.
///
/// Takes (dimension_name, cosine_distance) pairs and determines which pass
/// (≤ 0.4 threshold), finds the weakest dimension (highest distance, excluding
/// composite), and builds a rewrite prompt for it.
///
pub fn compute_embedding_quality(
    scores: &[(String, f64)],
    spec_name: &str,
    goal_texts: &[String],
    criteria_texts: &[String],
) -> EmbeddingQualityResult {
    if scores.is_empty() {
        return EmbeddingQualityResult {
            dimensions_passing: 0,
            meets_standard: false,
            weakest_dimension: None,
            rewrite_prompt: None,
        };
    }

    let passing = scores.iter().filter(|(_, d)| *d <= 0.4).count();

    let weakest = scores
        .iter()
        .filter(|(dim, _)| dim != "composite")
        .max_by(|(_, a), (_, b)| a.total_cmp(b));

    let weakest_dim = weakest.map(|(dim, _)| dim.clone());
    let rewrite_prompt = weakest.and_then(|(dim, dist)| {
        if *dist > 0.4 {
            Some(format!(
                "Rewrite this specification to improve its {} dimension (current cosine distance: {:.2}, threshold: 0.40).\n\n=== SPECIFICATION TO REWRITE ===\n\nName: {}\nGoals: {}\nCriteria: {}",
                dim,
                dist,
                spec_name,
                goal_texts.join("; "),
                criteria_texts.join("; "),
            ))
        } else {
            None
        }
    });

    EmbeddingQualityResult {
        dimensions_passing: passing,
        meets_standard: passing >= 3,
        weakest_dimension: weakest_dim,
        rewrite_prompt,
    }
}

// ── Replica rewrite prompt building ──────────────────────────────────────

/// Get dimension-specific rewrite guidance for the Gentle Lovelace persona.
///
/// Returns a guidance string explaining what to optimize for the given dimension.
///
pub fn dimension_guidance(dimension: &str) -> &'static str {
    match dimension.to_lowercase().as_str() {
        "gentle" => {
            "Rewrite this text to maximize agent-correctness. Docs ARE code — ensure every statement is actionable and unambiguous. Remove any stale references or outdated information."
        }
        "schriver" => {
            "Rewrite this text for maximum findability. Use scannable headings, descriptive hyperlinks, and front-load key concepts. A reader must find their answer within 30 seconds."
        }
        "hopper" => {
            "Rewrite this text for maximum accessibility. Make it comprehensible on first reading with zero prior context. Use plain language, active voice, and short sentences."
        }
        "lovelace" => {
            "Rewrite this text for maximum precision. Make every specification independently verifiable — a reader must be able to write a test from this text alone."
        }
        _ => {
            "Rewrite this text for all four dimensions of documentation excellence: agent-correctness (Gentle), findability (Schriver), accessibility (Hopper), and precision (Lovelace)."
        }
    }
}

/// Build a full replica rewrite prompt with dimension guidance and passage text.
///
pub fn build_rewrite_prompt(dimension: &str, passage: &str) -> String {
    let guidance = dimension_guidance(dimension);
    format!("{guidance}\n\n=== TEXT TO REWRITE ===\n\n{passage}")
}

/// Build the centroid reference string for a given dimension.
///
pub fn build_centroid_ref(dimension: &str) -> String {
    if dimension.to_lowercase() == "composite" {
        "style:gentle-lovelace:centroid".to_string()
    } else {
        format!(
            "style:gentle-lovelace:{}-centroid",
            dimension.to_lowercase()
        )
    }
}

// ── Spec graph query ─────────────────────────────────────────────────────

/// Query the spec graph: filter specs matching a search term and build
/// category-based adjacency edges and simple paths.
///
/// Matches against spec name, goal texts, and category string.
///
pub fn query_spec_graph(specs: &[Spec], query: &str, max_depth: u8) -> GraphQueryResult {
    let query_lower = query.to_lowercase();

    // Match specs where name, goals, or category contain the query
    let nodes: Vec<GraphNode> = specs
        .iter()
        .filter(|s| {
            s.name.to_lowercase().contains(&query_lower)
                || s.goals
                    .iter()
                    .any(|g| g.text.to_lowercase().contains(&query_lower))
                || s.category.as_str().contains(&query_lower)
        })
        .map(|s| GraphNode {
            id: s.id.to_string(),
            label: s.name.clone(),
            category: s.category.as_str().to_string(),
        })
        .collect();

    // Build edges between specs in the same category (composition adjacency)
    let mut edges = Vec::new();
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            if nodes[i].category == nodes[j].category {
                edges.push(GraphEdge {
                    from: nodes[i].id.clone(),
                    to: nodes[j].id.clone(),
                    relation: "same-category".to_string(),
                });
            }
        }
    }

    // Build simple paths (direct category-linked chains up to max_depth)
    let mut paths = Vec::new();
    for node in &nodes {
        let linked: Vec<String> = edges
            .iter()
            .filter(|e| e.from == node.id || e.to == node.id)
            .flat_map(|e| vec![e.from.clone(), e.to.clone()])
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(max_depth as usize)
            .collect();
        if !linked.is_empty() {
            paths.push(GraphPath {
                nodes: linked,
                length: 1,
            });
        }
    }

    GraphQueryResult {
        nodes,
        edges,
        paths,
    }
}

// ── Collection coherence ─────────────────────────────────────────────────

/// Compute collection-wide coherence for a set of specs.
///
/// Checks Jaccard-based coherence via Spec::collection_coherence, then
/// supplements with category coverage and completeness checks.
///
pub fn compute_collection_coherence(specs: &[Spec], threshold: f64) -> CoherenceCheck {
    let coherence = Spec::collection_coherence(specs);
    let mut violations = Vec::new();
    let mut suggestions = Vec::new();

    if coherence < threshold {
        violations.push(format!(
            "Collection coherence {:.2} below threshold {:.2}",
            coherence, threshold
        ));
    }

    let categories_covered: std::collections::HashSet<String> = specs
        .iter()
        .map(|s| s.category.as_str().to_string())
        .collect();
    for cat in SpecCategory::all() {
        if !categories_covered.contains(cat.as_str()) {
            suggestions.push(format!("Missing category: {}", cat.as_str()));
        }
    }

    for spec in specs {
        if !spec.is_complete() {
            suggestions.push(format!("Incomplete spec: {} ({})", spec.id, spec.name));
        }
    }

    CoherenceCheck {
        coherence_score: coherence,
        violations,
        suggestions,
    }
}

// ── Spec goal decomposition (structured) ──────────────────────────────────

/// Decompose all decomposable goals in a spec into sub-goals.
///
/// Mutates the spec in-place. Skips goals that already have sub-goals
/// or are at max depth (7).
///
pub fn decompose_spec_goals(spec: &mut Spec) {
    for goal in &mut spec.goals {
        if !goal.can_have_subgoals() || !goal.sub_goals.is_empty() {
            continue;
        }
        let (sub_texts, _deps) = decompose_description(&goal.text);
        if sub_texts.len() <= 1 {
            continue;
        }
        for text in &sub_texts {
            let mut child = GoalSpec::new(text);
            child.depth = goal.depth + 1;
            goal.sub_goals.push(child);
        }
    }
}

/// Collect all sub-goal texts and sequential dependencies from a spec.
///
/// Returns (sub_goals, dependencies) where dependencies are sequential
/// (each sub-goal depends on the previous).
///
pub fn collect_subgoals_and_deps(spec: &Spec) -> (Vec<String>, Vec<DependencyEdge>) {
    let mut all_subs = Vec::new();
    let mut all_deps = Vec::new();

    for goal in &spec.goals {
        for sub in &goal.sub_goals {
            all_subs.push(sub.text.clone());
        }
    }

    if all_subs.len() > 1 {
        for i in 1..all_subs.len() {
            all_deps.push(DependencyEdge {
                from: all_subs[i - 1].clone(),
                to: all_subs[i].clone(),
            });
        }
    }

    (all_subs, all_deps)
}

// ── Spec document text builder ────────────────────────────────────────────

/// Build a canonical document text from a spec for embedding or display.
///
pub fn build_spec_document_text(spec: &Spec) -> String {
    format!(
        "{}: Goals: {}. Criteria: {}.",
        spec.name,
        spec.goals
            .iter()
            .map(|g| g.text.as_str())
            .collect::<Vec<_>>()
            .join("; "),
        spec.goals
            .iter()
            .flat_map(|g| &g.criteria)
            .map(|c| c.description.as_str())
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// Collect goal texts and criteria texts from a spec as separate Vecs.
///
pub fn collect_goal_and_criteria_texts(spec: &Spec) -> (Vec<String>, Vec<String>) {
    let goals: Vec<String> = spec.goals.iter().map(|g| g.text.clone()).collect();
    let criteria: Vec<String> = spec
        .goals
        .iter()
        .flat_map(|g| &g.criteria)
        .map(|c| c.description.clone())
        .collect();
    (goals, criteria)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_types::{DomainAnchor, SpecId};

    fn make_test_spec(name: &str, category: SpecCategory) -> Spec {
        Spec::new(name, category, DomainAnchor::Hkask)
    }

    fn make_spec_with_goals(name: &str, category: SpecCategory, goal_texts: &[&str]) -> Spec {
        let mut spec = Spec::new(name, category, DomainAnchor::Hkask);
        for text in goal_texts {
            let goal = GoalSpec::new(text);
            spec = spec.with_goal(goal);
        }
        spec
    }

    // ── decompose_description ───────────────────────────────────────────

    #[test]
    fn decompose_description_single_sentence() {
        let (subs, deps) = decompose_description("Hello world.");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0], "Hello world");
        assert!(deps.is_empty());
    }

    #[test]
    fn decompose_description_multiple_sentences() {
        let (subs, deps) = decompose_description("First step. Second step. Third step.");
        assert_eq!(subs.len(), 3);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].from, "First step");
        assert_eq!(deps[0].to, "Second step");
    }

    #[test]
    fn decompose_description_empty() {
        let (subs, deps) = decompose_description("");
        assert!(subs.is_empty());
        assert!(deps.is_empty());
    }

    #[test]
    fn decompose_description_newline_separated() {
        let (subs, deps) = decompose_description("A\nB\nC");
        assert_eq!(subs.len(), 3);
        assert_eq!(deps.len(), 2);
    }

    // ── assess_writing_quality_heuristic ─────────────────────────────────

    #[test]
    fn heuristic_empty_spec_fails_all() {
        let spec = make_test_spec("", SpecCategory::Domain);
        let q = assess_writing_quality_heuristic(&spec);
        assert!(!q.hopper);
        assert!(!q.lovelace);
        assert!(!q.schriver);
        assert!(!q.gentle);
        assert_eq!(q.passes(), 0);
        assert!(!q.meets_publication_standard());
    }

    #[test]
    fn heuristic_named_spec_with_goals_passes_some() {
        let spec =
            make_spec_with_goals("Test Spec", SpecCategory::Domain, &["Goal one", "Goal two"]);
        let q = assess_writing_quality_heuristic(&spec);
        // has_description=true, has_goals=true, has_criteria=false, has_verbs=false
        assert!(!q.hopper); // needs criteria
        assert!(!q.lovelace); // needs criteria
        assert!(q.schriver); // has_description && has_goals
        assert!(!q.gentle); // needs verbs
        assert_eq!(q.passes(), 1);
    }

    // ── extract_ocap_boundaries ──────────────────────────────────────────

    #[test]
    fn extract_ocap_none_context() {
        assert!(extract_ocap_boundaries(None).is_empty());
    }

    #[test]
    fn extract_ocap_curation_context() {
        let boundaries = extract_ocap_boundaries(Some("curation workflow"));
        assert!(boundaries.contains(&"curation".to_string()));
    }

    #[test]
    fn extract_ocap_cybernetics_context() {
        let boundaries = extract_ocap_boundaries(Some("cns monitoring"));
        assert!(boundaries.contains(&"cybernetics".to_string()));
    }

    #[test]
    fn extract_ocap_irrelevant_context() {
        let boundaries = extract_ocap_boundaries(Some("hello world"));
        assert!(boundaries.is_empty());
    }

    // ── dimension_guidance ───────────────────────────────────────────────

    #[test]
    fn guidance_gentle_is_agent_focused() {
        let g = dimension_guidance("gentle");
        assert!(g.contains("agent-correctness"));
    }

    #[test]
    fn guidance_unknown_is_composite() {
        let g = dimension_guidance("unknown");
        assert!(g.contains("all four dimensions"));
    }

    // ── build_centroid_ref ───────────────────────────────────────────────

    #[test]
    fn centroid_ref_composite() {
        assert_eq!(
            build_centroid_ref("composite"),
            "style:gentle-lovelace:centroid"
        );
    }

    #[test]
    fn centroid_ref_gentle() {
        assert_eq!(
            build_centroid_ref("Gentle"),
            "style:gentle-lovelace:gentle-centroid"
        );
    }

    // ── query_spec_graph ─────────────────────────────────────────────────

    #[test]
    fn graph_query_matches_name() {
        let specs = vec![make_test_spec("Authentication", SpecCategory::Trust)];
        let result = query_spec_graph(&specs, "auth", 3);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].label, "Authentication");
    }

    #[test]
    fn graph_query_no_match() {
        let specs = vec![make_test_spec("Auth", SpecCategory::Domain)];
        let result = query_spec_graph(&specs, "zzz", 3);
        assert!(result.nodes.is_empty());
    }

    #[test]
    fn graph_query_same_category_edges() {
        let specs = vec![
            make_test_spec("Auth", SpecCategory::Trust),
            make_test_spec("RBAC", SpecCategory::Trust),
        ];
        let result = query_spec_graph(&specs, "trus", 3);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);
        assert_eq!(result.edges[0].relation, "same-category");
    }

    // ── compute_collection_coherence ─────────────────────────────────────

    #[test]
    fn coherence_empty_collection() {
        let result = compute_collection_coherence(&[], 0.7);
        assert_eq!(result.coherence_score, 0.0);
        assert!(!result.violations.is_empty());
    }

    // ── build_spec_document_text ─────────────────────────────────────────

    #[test]
    fn document_text_includes_name() {
        let spec = make_test_spec("TestSpec", SpecCategory::Domain);
        let text = build_spec_document_text(&spec);
        assert!(text.contains("TestSpec"));
    }

    // ── build_rewrite_prompt ─────────────────────────────────────────────

    #[test]
    fn rewrite_prompt_contains_passage() {
        let prompt = build_rewrite_prompt("gentle", "some text here");
        assert!(prompt.contains("some text here"));
        assert!(prompt.contains("agent-correctness"));
    }

    // ── collect_goal_and_criteria_texts ──────────────────────────────────

    #[test]
    fn collect_texts_from_spec_with_goals() {
        let mut spec = make_test_spec("S", SpecCategory::Domain);
        let mut goal = GoalSpec::new("G1");
        goal = goal.with_criterion("C1");
        goal = goal.with_criterion("C2");
        spec = spec.with_goal(goal);
        let (goals, criteria) = collect_goal_and_criteria_texts(&spec);
        assert_eq!(goals, vec!["G1"]);
        assert_eq!(criteria, vec!["C1", "C2"]);
    }

    // ── decompose_spec_goals ─────────────────────────────────────────────

    #[test]
    fn decompose_goal_creates_sub_goals() {
        let mut spec = make_spec_with_goals("S", SpecCategory::Domain, &["First. Second. Third."]);
        decompose_spec_goals(&mut spec);
        assert_eq!(spec.goals[0].sub_goals.len(), 3);
        assert_eq!(spec.goals[0].sub_goals[0].text, "First");
    }

    #[test]
    fn decompose_single_sentence_no_sub_goals() {
        let mut spec = make_spec_with_goals("S", SpecCategory::Domain, &["Only one sentence"]);
        decompose_spec_goals(&mut spec);
        assert!(spec.goals[0].sub_goals.is_empty());
    }
}
