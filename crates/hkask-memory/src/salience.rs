//! Salience scoring and method signal extraction for style corpora.
//!
//! Computes two things from raw passage text (zero LLM cost):
//! 1. **Method signals** — cheap stylometric metrics (parataxis ratio,
//!    adjective density, dialogue ratio, etc.) that constitute the "how"
//!    dimension of the 5W1H framework.
//! 2. **Salience score** — weighted graph degree centrality combining entity
//!    tag counts, method coverage, category diversity, and positional
//!    significance into a single 0.0–1.0+ score.
//!
//! Used by `EmbedService` at embed time (budget gating) and by the style
//! synthesizer at query time (salience-parameterized retrieval).

// ── Method Signals ────────────────────────────────────────────────────────

/// Cheaply-computed stylometric signals for a passage.
///
/// All fields are derived from simple text analysis — no model inference.
/// These constitute the "how" (methods/techniques) dimension of the 5W1H
/// metadata layer.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct MethodSignals {
    /// Ratio of coordinating conjunctions (and, but, or) to total
    /// conjunctions. High = paratactic (Hemingway). Low = hypotactic (Wilde).
    pub parataxis_ratio: f32,

    /// Approximate adjective count per 100 words. Uses suffix heuristics
    /// (-y, -ous, -ful, -less, -ive, -able, -al, -ent, -ic, -ish).
    pub adjective_density: f32,

    /// Words ending in -ly per 100 words (filtered for common false
    /// positives like "only", "early", "family").
    pub adverb_density: f32,

    /// Ratio of "was/were `<verb>ed`" patterns to total verbs.
    pub passive_voice_ratio: f32,

    /// Words inside double-quote characters divided by total words.
    pub dialogue_ratio: f32,

    /// Standard deviation of sentence lengths within the passage.
    pub sentence_length_variance: f32,

    /// Hedge words ("perhaps", "maybe", "seemed", "almost", "rather",
    /// "quite") per 100 words. Indicates qualification/uncertainty.
    pub hedge_density: f32,

    /// Intensifiers ("very", "really", "absolutely", "extremely",
    /// "utterly", "completely") per 100 words.
    pub intensifier_density: f32,

    /// Tangible/concrete nouns ÷ abstract nouns (rough suffix heuristic).
    /// High = sensory, concrete. Low = abstract, conceptual.
    pub concrete_noun_ratio: f32,

    /// Sensory words (sight, sound, touch, taste, smell) per 100 words.
    pub sensory_word_ratio: f32,

    /// Total word count of the passage.
    pub word_count: usize,

    /// Number of sentences in the passage.
    pub sentence_count: usize,

    /// Average sentence length (words/sentences).
    pub avg_sentence_length: f32,
}

/// Compute method signals from raw passage text.
///
/// All signals are cheap substring/character operations. No allocations
/// beyond what's needed for word splitting.
pub fn compute_method_signals(text: &str) -> MethodSignals {
    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len();
    if word_count == 0 {
        return MethodSignals::default();
    }

    // Sentence boundaries
    let sentence_count = text
        .chars()
        .filter(|c| matches!(c, '.' | '!' | '?'))
        .count()
        .max(1);
    let sentence_lengths = sentence_lengths(text);
    let avg_sentence_length = word_count as f32 / sentence_count as f32;
    let sentence_length_variance = if sentence_lengths.len() > 1 {
        let mean = sentence_lengths.iter().sum::<usize>() as f32 / sentence_lengths.len() as f32;
        let variance = sentence_lengths
            .iter()
            .map(|&l| (l as f32 - mean).powi(2))
            .sum::<f32>()
            / sentence_lengths.len() as f32;
        variance.sqrt()
    } else {
        0.0
    };

    // ── Signal extraction ─────────────────────────────────────────

    let per_100 = |count: usize| -> f32 { (count as f32 / word_count as f32) * 100.0 };

    // Parataxis ratio: coordinating ÷ (coordinating + subordinating)
    let coord_count = words
        .iter()
        .filter(|w| matches!(w.to_lowercase().as_str(), "and" | "but" | "or" | "nor"))
        .count();
    let subord_count = words
        .iter()
        .filter(|w| {
            matches!(
                w.to_lowercase().as_str(),
                "because"
                    | "although"
                    | "though"
                    | "since"
                    | "while"
                    | "unless"
                    | "until"
                    | "whereas"
                    | "after"
                    | "before"
                    | "if"
                    | "when"
            )
        })
        .count();
    let total_conj = coord_count + subord_count;
    let parataxis_ratio = if total_conj > 0 {
        coord_count as f32 / total_conj as f32
    } else {
        0.5 // neutral
    };

    // Adjective density: suffix-heuristic
    let adj_suffixes = [
        "y", "ous", "ful", "less", "ive", "able", "ible", "al", "ent", "ic", "ish",
    ];
    let adj_count = words
        .iter()
        .filter(|w| {
            let lower = w.to_lowercase();
            // Exclude very short words and common non-adjective -y words
            if lower.len() < 3 {
                return false;
            }
            adj_suffixes
                .iter()
                .any(|suf| lower.ends_with(suf) && lower.len() > suf.len() + 1)
        })
        .count();
    let adjective_density = per_100(adj_count);

    // Adverb density: -ly suffix, minus false positives
    let ly_false_positives = [
        "only",
        "early",
        "family",
        "july",
        "jelly",
        "belly",
        "holy",
        "reply",
        "apply",
        "supply",
        "multiply",
        "butterfly",
        "barfly",
    ];
    let adv_count = words
        .iter()
        .filter(|w| {
            let lower = w.to_lowercase();
            lower.ends_with("ly")
                && lower.len() > 3
                && !ly_false_positives.contains(&lower.as_str())
        })
        .count();
    let adverb_density = per_100(adv_count);

    // Passive voice: count "was *ed", "were *ed" patterns
    let passive_count = words
        .windows(2)
        .filter(|pair| {
            let first = pair[0].to_lowercase();
            let second = pair[1].to_lowercase();
            (first == "was" || first == "were") && (second.ends_with("ed") && second.len() > 4)
        })
        .count();
    let passive_voice_ratio = if word_count > 0 {
        passive_count as f32 / (word_count / 10) as f32
    } else {
        0.0
    };

    // Dialogue ratio: words inside double quotes
    let in_quotes = {
        let mut inside = false;
        let mut quote_words = 0usize;
        for word in &words {
            let has_open = word.contains('"') || word.contains('\u{201c}');
            let has_close = word.contains('"') || word.contains('\u{201d}');
            if inside {
                quote_words += 1;
            }
            if has_open && !has_close {
                inside = true;
                quote_words += 1;
            } else if has_close && !has_open {
                inside = false;
            } else if has_open && has_close {
                // Single-word quote
                quote_words += 1;
            }
        }
        quote_words
    };
    let dialogue_ratio = if word_count > 0 {
        in_quotes as f32 / word_count as f32
    } else {
        0.0
    };

    // Hedge density
    let hedge_words = [
        "perhaps",
        "maybe",
        "seemed",
        "almost",
        "rather",
        "quite",
        "fairly",
        "somewhat",
        "apparently",
        "presumably",
    ];
    let hedge_count = words
        .iter()
        .filter(|w| hedge_words.contains(&w.to_lowercase().as_str()))
        .count();
    let hedge_density = per_100(hedge_count);

    // Intensifier density
    let intensifiers = [
        "very",
        "really",
        "absolutely",
        "extremely",
        "utterly",
        "completely",
        "totally",
        "entirely",
        "thoroughly",
    ];
    let intensifier_count = words
        .iter()
        .filter(|w| intensifiers.contains(&w.to_lowercase().as_str()))
        .count();
    let intensifier_density = per_100(intensifier_count);

    // Concrete noun ratio: rough heuristic based on common concrete suffixes
    // vs abstract suffixes (-tion, -ness, -ity, -ism, -ment, -ance, -ence)
    let abstract_suffixes = [
        "tion", "ness", "ity", "ism", "ment", "ance", "ence", "hood", "ship",
    ];
    let abstract_count = words
        .iter()
        .filter(|w| {
            let lower = w.to_lowercase();
            lower.len() > 4 && abstract_suffixes.iter().any(|s| lower.ends_with(s))
        })
        .count();
    let concrete_count = words.len().saturating_sub(abstract_count);
    let concrete_noun_ratio = if !words.is_empty() {
        concrete_count as f32 / words.len() as f32
    } else {
        0.7 // default: mostly concrete
    };

    // Sensory word density
    let sensory_words = [
        // Sight
        "bright",
        "dark",
        "red",
        "blue",
        "green",
        "white",
        "black",
        "pale",
        "shadow",
        "light",
        "color",
        "gleam",
        "glitter",
        "shine",
        "dim",
        "glow",
        // Sound
        "loud",
        "quiet",
        "silent",
        "noise",
        "sound",
        "echo",
        "whisper",
        "roar",
        "crash",
        // Touch
        "cold",
        "hot",
        "warm",
        "cool",
        "rough",
        "smooth",
        "hard",
        "soft",
        "wet",
        "dry",
        "sharp",
        "heavy",
        "light",
        // Taste
        "sweet",
        "bitter",
        "sour",
        "salty",
        // Smell
        "scent",
        "smell",
        "odor",
        "fragrance",
        "stink",
    ];
    let sensory_count = words
        .iter()
        .filter(|w| sensory_words.contains(&w.to_lowercase().as_str()))
        .count();
    let sensory_word_ratio = per_100(sensory_count);

    MethodSignals {
        parataxis_ratio: parataxis_ratio.clamp(0.0, 1.0),
        adjective_density,
        adverb_density,
        passive_voice_ratio: passive_voice_ratio.clamp(0.0, 1.0),
        dialogue_ratio,
        sentence_length_variance,
        hedge_density,
        intensifier_density,
        concrete_noun_ratio: concrete_noun_ratio.clamp(0.0, 1.0),
        sensory_word_ratio,
        word_count,
        sentence_count,
        avg_sentence_length,
    }
}

/// Compute sentence lengths by splitting on `. ! ?` boundaries.
fn sentence_lengths(text: &str) -> Vec<usize> {
    let mut lengths = Vec::new();
    let mut current = 0usize;
    for word in text.split_whitespace() {
        current += 1;
        let last_char = word.chars().last();
        if matches!(last_char, Some('.') | Some('!') | Some('?')) && current > 0 {
            lengths.push(current);
            current = 0;
        }
    }
    if current > 0 {
        lengths.push(current);
    }
    lengths
}

// ── Declared Method Matching ──────────────────────────────────────────────

/// A declared method with signal thresholds for matching.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeclaredMethod {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub signal: MethodThresholds,
}

/// Thresholds for matching a declared method to a passage's signals.
///
/// Each field is optional — only the fields present constrain the match.
/// A passage matches if ALL present constraints are satisfied.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct MethodThresholds {
    #[serde(default)]
    pub parataxis_ratio_min: Option<f32>,
    #[serde(default)]
    pub parataxis_ratio_max: Option<f32>,
    #[serde(default)]
    pub adjective_density_max: Option<f32>,
    #[serde(default)]
    pub adverb_density_max: Option<f32>,
    #[serde(default)]
    pub passive_voice_ratio_max: Option<f32>,
    #[serde(default)]
    pub dialogue_ratio_min: Option<f32>,
    #[serde(default)]
    pub sentence_length_variance_max: Option<f32>,
    #[serde(default)]
    pub hedge_density_max: Option<f32>,
    #[serde(default)]
    pub intensifier_density_max: Option<f32>,
    #[serde(default)]
    pub concrete_noun_ratio_min: Option<f32>,
    #[serde(default)]
    pub sensory_word_ratio_min: Option<f32>,
    #[serde(default)]
    pub avg_sentence_length_min: Option<f32>,
    #[serde(default)]
    pub avg_sentence_length_max: Option<f32>,
}

impl DeclaredMethod {
    /// Check whether a passage's signals match this method's thresholds.
    pub fn matches(&self, signals: &MethodSignals) -> bool {
        let t = &self.signal;
        check_min(t.parataxis_ratio_min, signals.parataxis_ratio)
            && check_max(t.parataxis_ratio_max, signals.parataxis_ratio)
            && check_max(t.adjective_density_max, signals.adjective_density)
            && check_max(t.adverb_density_max, signals.adverb_density)
            && check_max(t.passive_voice_ratio_max, signals.passive_voice_ratio)
            && check_min(t.dialogue_ratio_min, signals.dialogue_ratio)
            && check_max(
                t.sentence_length_variance_max,
                signals.sentence_length_variance,
            )
            && check_max(t.hedge_density_max, signals.hedge_density)
            && check_max(t.intensifier_density_max, signals.intensifier_density)
            && check_min(t.concrete_noun_ratio_min, signals.concrete_noun_ratio)
            && check_min(t.sensory_word_ratio_min, signals.sensory_word_ratio)
            && check_min(t.avg_sentence_length_min, signals.avg_sentence_length)
            && check_max(t.avg_sentence_length_max, signals.avg_sentence_length)
    }
}

fn check_min(min: Option<f32>, value: f32) -> bool {
    min.is_none_or(|m| value >= m)
}

fn check_max(max: Option<f32>, value: f32) -> bool {
    max.is_none_or(|m| value <= m)
}

// ── Entity Tagging ────────────────────────────────────────────────────────

/// Tags extracted from a passage by string-matching against declared entities.
#[derive(Debug, Clone, Default)]
pub struct EntityTags {
    pub characters: Vec<String>,
    pub places: Vec<String>,
    pub events: Vec<String>,
    pub concepts: Vec<String>,
    pub methods: Vec<String>,
}

/// Tag a passage by matching declared entity names against the text.
///
/// Uses simple case-insensitive substring matching. Returns distinct
/// tags only (no duplicates within a category).
pub fn tag_entities(
    text: &str,
    characters: &[String],
    places: &[String],
    events: &[String],
    concepts: &[String],
) -> EntityTags {
    let lower = text.to_lowercase();
    EntityTags {
        characters: filter_matches(&lower, characters),
        places: filter_matches(&lower, places),
        events: filter_matches(&lower, events),
        concepts: filter_matches(&lower, concepts),
        methods: Vec::new(), // filled separately by method matching
    }
}

fn filter_matches(lower_text: &str, candidates: &[String]) -> Vec<String> {
    candidates
        .iter()
        .filter(|c| lower_text.contains(&c.to_lowercase()))
        .cloned()
        .collect()
}

impl EntityTags {
    /// All entity and method names as a single iterator for graph construction.
    pub fn all_tags(&self) -> impl Iterator<Item = &str> {
        self.characters
            .iter()
            .map(String::as_str)
            .chain(self.places.iter().map(String::as_str))
            .chain(self.events.iter().map(String::as_str))
            .chain(self.concepts.iter().map(String::as_str))
            .chain(self.methods.iter().map(String::as_str))
    }

    /// Number of distinct tags across all categories.
    pub fn tag_count(&self) -> usize {
        self.characters.len()
            + self.places.len()
            + self.events.len()
            + self.concepts.len()
            + self.methods.len()
    }
}

// ── Salience Score ────────────────────────────────────────────────────────

/// Compute salience scores for all tagged passages using graph centrality.
///
/// Builds a bipartite graph (passages ↔ entities/methods), then computes
/// one-hop and two-hop connectedness for each passage.
///
/// - **one_hop(p)**: fraction of passages that share ≥1 tag with p.
/// - **two_hop(p)**: fraction of passages reachable from p in ≤2 hops
///   (one-hop set ∪ their neighbors). Always ≥ one_hop(p).
///
/// Salience = (one_hop + two_hop/2) / 2
///
/// This naturally biases toward direct connections: one-hop has full
/// weight, two-hop is halved. No configuration needed.
///
/// Returns a vector of salience scores in the same order as the input.
/// Foundational rules (passages with zero tags) get salience 0.0.
pub fn compute_salience_batch(all_tags: &[EntityTags]) -> Vec<f32> {
    let n = all_tags.len();
    if n == 0 {
        return Vec::new();
    }

    // Build inverted index: entity_name → set of passage indices
    let mut entity_to_passages: std::collections::HashMap<&str, Vec<usize>> =
        std::collections::HashMap::new();

    for (i, tags) in all_tags.iter().enumerate() {
        for tag in tags.all_tags() {
            entity_to_passages.entry(tag).or_default().push(i);
        }
    }

    // For each passage, compute its neighbor set (union of all entity co-occurrences)
    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, tags) in all_tags.iter().enumerate() {
        let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for tag in tags.all_tags() {
            if let Some(passages) = entity_to_passages.get(tag) {
                for &p in passages {
                    if p != i {
                        seen.insert(p);
                    }
                }
            }
        }
        neighbors[i] = seen.into_iter().collect();
    }

    // One-hop: fraction of passages directly connected
    let n_f = n as f32;
    let one_hop: Vec<f32> = neighbors.iter().map(|nb| nb.len() as f32 / n_f).collect();

    // Two-hop: fraction of passages reachable in ≤2 hops (includes one-hop set)
    let two_hop: Vec<f32> = neighbors
        .iter()
        .enumerate()
        .map(|(i, nb)| {
            let mut reachable: std::collections::HashSet<usize> =
                std::collections::HashSet::from_iter(nb.iter().copied());
            for &j in nb {
                for &k in &neighbors[j] {
                    if k != i {
                        reachable.insert(k);
                    }
                }
            }
            reachable.len() as f32 / n_f
        })
        .collect();

    // Salience = (one_hop + two_hop/2) / 2 — biases toward direct connections
    one_hop
        .iter()
        .zip(two_hop.iter())
        .map(|(&oh, &th)| (oh + th / 2.0) / 2.0)
        .collect()
}

// ── Budget ────────────────────────────────────────────────────────────────

/// Triple budget configuration for gating metadata storage.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum BudgetConfig {
    /// Budget derived from passage count: `triples_per_100_pages`.
    PerPage {
        #[serde(default = "default_budget_per_100_pages")]
        per_100_pages: usize,
    },
    /// Absolute hard cap.
    Absolute { max_triples: usize },
}

fn default_budget_per_100_pages() -> usize {
    3750
}

impl Default for BudgetConfig {
    fn default() -> Self {
        BudgetConfig::PerPage {
            per_100_pages: default_budget_per_100_pages(),
        }
    }
}

impl BudgetConfig {
    /// Compute the absolute triple budget from the config and passage count.
    ///
    /// For `PerPage`: budget = (passage_count / 250) × per_100_pages.
    /// The constant 250 assumes ~250 passages ≈ 100 pages.
    pub fn resolve(&self, passage_count: usize) -> usize {
        match self {
            BudgetConfig::PerPage { per_100_pages } => {
                let pages_equivalent = (passage_count as f32 / 250.0).max(1.0);
                (pages_equivalent * *per_100_pages as f32).ceil() as usize
            }
            BudgetConfig::Absolute { max_triples } => *max_triples,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_signals_hemingway_like() {
        let text = "He drank the wine. It was good. He walked out into the rain.";
        let signals = compute_method_signals(text);
        assert!(signals.parataxis_ratio > 0.0);
        assert!(signals.adverb_density < 5.0); // no -ly words
        assert!(signals.sentence_count >= 3);
        assert!(signals.avg_sentence_length < 10.0);
    }

    #[test]
    fn method_signals_wilde_like() {
        let text = "The utterly magnificent and beautifully ornate chandelier \
                    glittered brilliantly in the perfectly silent ballroom, \
                    casting remarkably delicate shadows upon the exquisitely \
                    dressed guests who seemed almost entirely entranced.";
        let signals = compute_method_signals(text);
        assert!(signals.adjective_density > 5.0);
        assert!(signals.adverb_density > 5.0);
        assert!(signals.hedge_density > 0.0);
        assert!(signals.intensifier_density > 0.0);
    }

    #[test]
    fn declared_method_matches() {
        let method = DeclaredMethod {
            name: "low_adjective".into(),
            description: String::new(),
            signal: MethodThresholds {
                adjective_density_max: Some(5.0),
                ..Default::default()
            },
        };
        let hemingway_signals =
            compute_method_signals("He drank the wine. It was good. He walked out.");
        assert!(method.matches(&hemingway_signals));

        let wilde_signals = compute_method_signals(
            "The magnificent and beautifully ornate chandelier glittered brilliantly.",
        );
        assert!(!method.matches(&wilde_signals));
    }

    #[test]
    fn salience_zero_for_empty_tags() {
        let tags = vec![EntityTags::default()];
        let scores = compute_salience_batch(&tags);
        assert_eq!(scores.len(), 1);
        assert!((scores[0] - 0.0).abs() < 0.01);
    }

    #[test]
    fn salience_increases_with_shared_entities() {
        // Three passages: two share "Jake", one isolated
        let tags = vec![
            EntityTags {
                characters: vec!["Jake".into()],
                ..Default::default()
            },
            EntityTags {
                characters: vec!["Jake".into(), "Brett".into()],
                ..Default::default()
            },
            EntityTags {
                concepts: vec!["rain".into()],
                ..Default::default()
            },
        ];
        let scores = compute_salience_batch(&tags);
        assert!(scores[0] > 0.0, "passage 0 shares Jake with passage 1");
        assert!(scores[1] > 0.0, "passage 1 shares Jake with passage 0");
        assert!((scores[2] - 0.0).abs() < 0.01, "passage 2 isolated");
    }

    #[test]
    fn two_hop_always_gte_one_hop() {
        // By definition, passages reachable in ≤2 hops includes those
        // reachable in 1 hop.
        let tags = [
            EntityTags {
                characters: vec!["A".into()],
                ..Default::default()
            },
            EntityTags {
                characters: vec!["A".into(), "B".into()],
                ..Default::default()
            },
            EntityTags {
                characters: vec!["B".into()],
                ..Default::default()
            },
            EntityTags::default(),
        ];
        // Compute raw hop fractions to verify invariant
        let n = tags.len();
        let mut entity_to_passages: std::collections::HashMap<&str, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, t) in tags.iter().enumerate() {
            for tag in t.all_tags() {
                entity_to_passages.entry(tag).or_default().push(i);
            }
        }
        for (i, t) in tags.iter().enumerate() {
            let mut one_hop_set: std::collections::HashSet<usize> =
                std::collections::HashSet::new();
            for tag in t.all_tags() {
                if let Some(ps) = entity_to_passages.get(tag) {
                    for &p in ps {
                        if p != i {
                            one_hop_set.insert(p);
                        }
                    }
                }
            }
            let mut two_hop_set = one_hop_set.clone();
            for &j in &one_hop_set {
                for tag in tags[j].all_tags() {
                    if let Some(ps) = entity_to_passages.get(tag) {
                        for &p in ps {
                            if p != i {
                                two_hop_set.insert(p);
                            }
                        }
                    }
                }
            }
            let one = one_hop_set.len() as f32 / n as f32;
            let two = two_hop_set.len() as f32 / n as f32;
            assert!(two >= one, "passage {i}: two_hop={two} < one_hop={one}");
        }
    }

    #[test]
    fn two_hop_amplifies_connected_neighbors() {
        // A: protagonist only. B: protagonist + rare_place (bridge).
        // C: rare_place only. D: isolated.
        let tags = vec![
            EntityTags {
                characters: vec!["protagonist".into()],
                ..Default::default()
            },
            EntityTags {
                characters: vec!["protagonist".into()],
                places: vec!["rare_place".into()],
                ..Default::default()
            },
            EntityTags {
                places: vec!["rare_place".into()],
                ..Default::default()
            },
            EntityTags::default(),
        ];
        let scores = compute_salience_batch(&tags);
        assert!(
            scores[1] > scores[0],
            "B bridges protagonist and rare_place"
        );
        assert!(scores[2] > 0.0, "C connects to B via rare_place");
        assert!((scores[3] - 0.0).abs() < 0.01, "D is isolated");
    }

    #[test]
    fn methods_participate_in_graph() {
        let tags = vec![
            EntityTags {
                methods: vec!["iceberg_theory".into()],
                ..Default::default()
            },
            EntityTags {
                methods: vec!["iceberg_theory".into()],
                ..Default::default()
            },
        ];
        let scores = compute_salience_batch(&tags);
        assert!(scores[0] > 0.0);
        assert!(scores[1] > 0.0);
    }

    #[test]
    fn budget_per_page_resolve() {
        let budget = BudgetConfig::PerPage {
            per_100_pages: 3750,
        };
        // 250 passages ≈ 100 pages
        assert_eq!(budget.resolve(250), 3750);
        // 500 passages ≈ 200 pages
        assert_eq!(budget.resolve(500), 7500);
        // Tiny corpus: minimum 1 page-equivalent
        assert!(budget.resolve(10) >= 3750);
    }

    #[test]
    fn budget_absolute() {
        let budget = BudgetConfig::Absolute { max_triples: 10000 };
        assert_eq!(budget.resolve(5000), 10000);
    }

    #[test]
    fn entity_tagging_case_insensitive() {
        let text = "Jake Barnes walked through Paris in the rain.";
        let tags = tag_entities(text, &["Jake Barnes".into()], &["Paris".into()], &[], &[]);
        assert_eq!(tags.characters, vec!["Jake Barnes"]);
        assert_eq!(tags.places, vec!["Paris"]);
    }

    #[test]
    fn dialogue_ratio_detection() {
        let text = "\"I'm not drunk,\" he said. \"You are,\" she replied. The rain fell.";
        let signals = compute_method_signals(text);
        assert!(signals.dialogue_ratio > 0.3);
    }
}
