//! Prompt Decomposition — deterministic sentence analysis for inference variety sensing
//!
//! Analyzes the full prompt (system prompt + user input + semantic context) to produce
//! structured variety signals. No model calls — purely deterministic string analysis.
//!
//! The decomposition extracts three CNS variety domains:
//! - `cns.inference.prompt_depth` — shallow/medium/deep based on clause density
//! - `cns.inference.prompt_structure` — question/imperative/declarative/conditional
//! - `cns.inference.prompt_domain` — unique lemmatized topic keywords
//!
//! Originally in `hkask-cns::prompt_decomposition` — relocated to `hkask-agents`
//! because this is inference variety sensing (Loop 1), not cybernetic regulation (Loop 6).
//! The CNS consumes the output; it does not produce it.

use serde::{Deserialize, Serialize};

/// Per-sentence decomposition result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceDecomposition {
    /// Extracted subject noun phrases (lemmatized to root form)
    pub subjects: Vec<String>,
    /// Main verbs (lemmatized to root form)
    pub verbs: Vec<String>,
    /// Predicate/object phrases
    pub predicates: Vec<String>,
    /// Conditional clauses (if/when/unless/although/provided)
    pub conditionals: Vec<String>,
    /// Whether the sentence is a question
    pub is_question: bool,
    /// Whether the sentence is imperative (command)
    pub is_imperative: bool,
}

/// Aggregated analysis of the full prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAnalysis {
    /// Per-sentence decomposition results
    pub sentences: Vec<SentenceDecomposition>,
    /// Total sentence count
    pub sentence_count: usize,
    /// Average clauses per sentence (predicates + conditionals)
    pub clause_density: f64,
    /// Depth bucket: "shallow" (<2 clauses/sentence), "medium" (2-4), "deep" (>4)
    pub depth_bucket: &'static str,
    /// All unique topic keywords (lemmatized subjects across all sentences)
    pub topic_keywords: Vec<String>,
    /// Number of distinct verb roots
    pub verb_diversity: usize,
    /// Count of conditional clauses across all sentences
    pub conditional_count: usize,
    /// Count of questions across all sentences
    pub question_count: usize,
    /// Count of imperatives across all sentences
    pub imperative_count: usize,
}

/// Lemmatize a word to its root form using a lookup table of common English inflections.
///
/// This covers the most frequent English inflection patterns. Words not in the
/// table are returned lowercased but otherwise unchanged.
fn lemmatize(word: &str) -> String {
    let lower = word.to_lowercase();
    let irregular: &[(&str, &str)] = &[
        ("was", "be"),
        ("were", "be"),
        ("am", "be"),
        ("is", "be"),
        ("are", "be"),
        ("been", "be"),
        ("being", "be"),
        ("had", "have"),
        ("has", "have"),
        ("having", "have"),
        ("did", "do"),
        ("does", "do"),
        ("doing", "do"),
        ("done", "do"),
        ("went", "go"),
        ("gone", "go"),
        ("goes", "go"),
        ("going", "go"),
        ("came", "come"),
        ("comes", "come"),
        ("coming", "come"),
        ("made", "make"),
        ("makes", "make"),
        ("making", "make"),
        ("took", "take"),
        ("takes", "take"),
        ("taken", "take"),
        ("taking", "take"),
        ("gave", "give"),
        ("gives", "give"),
        ("given", "give"),
        ("giving", "give"),
        ("said", "say"),
        ("says", "say"),
        ("saying", "say"),
        ("got", "get"),
        ("gets", "get"),
        ("getting", "get"),
        ("gotten", "get"),
        ("found", "find"),
        ("finds", "find"),
        ("finding", "find"),
        ("knew", "know"),
        ("knows", "know"),
        ("known", "know"),
        ("knowing", "know"),
        ("thought", "think"),
        ("thinks", "think"),
        ("thinking", "think"),
        ("saw", "see"),
        ("sees", "see"),
        ("seen", "see"),
        ("seeing", "see"),
        ("ran", "run"),
        ("runs", "run"),
        ("running", "run"),
        ("wrote", "write"),
        ("writes", "write"),
        ("written", "write"),
        ("writing", "write"),
        ("told", "tell"),
        ("tells", "tell"),
        ("telling", "tell"),
        ("left", "leave"),
        ("leaves", "leave"),
        ("leaving", "leave"),
        ("felt", "feel"),
        ("feels", "feel"),
        ("feeling", "feel"),
        ("kept", "keep"),
        ("keeps", "keep"),
        ("keeping", "keep"),
        ("let", "let"),
        ("lets", "let"),
        ("letting", "let"),
        ("began", "begin"),
        ("begins", "begin"),
        ("beginning", "begin"),
        ("held", "hold"),
        ("holds", "hold"),
        ("holding", "hold"),
        ("stood", "stand"),
        ("stands", "stand"),
        ("standing", "stand"),
        ("understood", "understand"),
        ("understands", "understand"),
        ("lost", "lose"),
        ("loses", "lose"),
        ("losing", "lose"),
        ("paid", "pay"),
        ("pays", "pay"),
        ("paying", "pay"),
        ("met", "meet"),
        ("meets", "meet"),
        ("meeting", "meet"),
        ("learned", "learn"),
        ("learns", "learn"),
        ("learning", "learn"),
        ("showed", "show"),
        ("shows", "show"),
        ("shown", "show"),
        ("showing", "show"),
        ("heard", "hear"),
        ("hears", "hear"),
        ("hearing", "hear"),
        ("turned", "turn"),
        ("turns", "turn"),
        ("turning", "turn"),
        ("started", "start"),
        ("starts", "start"),
        ("starting", "start"),
        ("needed", "need"),
        ("needs", "need"),
        ("needing", "need"),
        ("used", "use"),
        ("uses", "use"),
        ("using", "use"),
        ("worked", "work"),
        ("works", "work"),
        ("working", "work"),
        ("called", "call"),
        ("calls", "call"),
        ("calling", "call"),
        ("tried", "try"),
        ("tries", "try"),
        ("trying", "try"),
        ("asked", "ask"),
        ("asks", "ask"),
        ("asking", "ask"),
        ("looked", "look"),
        ("looks", "look"),
        ("looking", "look"),
        ("wanted", "want"),
        ("wants", "want"),
        ("wanting", "want"),
        ("helped", "help"),
        ("helps", "help"),
        ("helping", "help"),
        ("talked", "talk"),
        ("talks", "talk"),
        ("talking", "talk"),
        ("played", "play"),
        ("plays", "play"),
        ("playing", "play"),
        ("moved", "move"),
        ("moves", "move"),
        ("moving", "move"),
        ("lived", "live"),
        ("lives", "live"),
        ("living", "live"),
        ("believed", "believe"),
        ("believes", "believe"),
        ("believing", "believe"),
        ("brought", "bring"),
        ("brings", "bring"),
        ("bringing", "bring"),
        ("happened", "happen"),
        ("happens", "happen"),
        ("happening", "happen"),
        ("provided", "provide"),
        ("provides", "provide"),
        ("providing", "provide"),
        ("considered", "consider"),
        ("considers", "consider"),
        ("considering", "consider"),
        ("created", "create"),
        ("creates", "create"),
        ("creating", "create"),
        ("required", "require"),
        ("requires", "require"),
        ("requiring", "require"),
        ("included", "include"),
        ("includes", "include"),
        ("including", "include"),
        ("followed", "follow"),
        ("follows", "follow"),
        ("following", "follow"),
        ("allowed", "allow"),
        ("allows", "allow"),
        ("allowing", "allow"),
        ("led", "lead"),
        ("leads", "lead"),
        ("leading", "lead"),
        ("set", "set"),
        ("sets", "set"),
        ("setting", "set"),
        ("put", "put"),
        ("puts", "put"),
        ("putting", "put"),
        ("added", "add"),
        ("adds", "add"),
        ("adding", "add"),
        ("stayed", "stay"),
        ("stays", "stay"),
        ("staying", "stay"),
        ("changed", "change"),
        ("changes", "change"),
        ("changing", "change"),
        ("received", "receive"),
        ("receives", "receive"),
        ("receiving", "receive"),
        ("returned", "return"),
        ("returns", "return"),
        ("returning", "return"),
    ];

    for (form, root) in irregular {
        if lower == *form {
            return root.to_string();
        }
    }
    if lower.ends_with("ies") && lower.len() > 4 {
        return format!("{}y", &lower[..lower.len() - 3]);
    }
    if lower.ends_with("es") && lower.len() > 3 {
        let stem = &lower[..lower.len() - 2];
        if lower.ends_with("sses")
            || lower.ends_with("shes")
            || lower.ends_with("ches")
            || lower.ends_with("xes")
            || lower.ends_with("zes")
        {
            return stem.to_string();
        }
        return stem.to_string();
    }
    if lower.ends_with("s") && !lower.ends_with("ss") && lower.len() > 3 {
        return lower[..lower.len() - 1].to_string();
    }
    if lower.ends_with("ing") && lower.len() > 5 {
        let stem = &lower[..lower.len() - 3];
        if stem.len() >= 2 {
            let chars: Vec<char> = stem.chars().collect();
            if chars.len() >= 2
                && chars[chars.len() - 1] == chars[chars.len() - 2]
                && "bdfglmnprst".contains(chars[chars.len() - 1])
            {
                return stem[..stem.len() - 1].to_string();
            }
        }
        return stem.to_string();
    }
    if lower.ends_with("ed") && lower.len() > 4 {
        let stem = &lower[..lower.len() - 2];
        if stem.ends_with("at") || stem.ends_with("it") || stem.ends_with("ut") {
            return format!("{}e", stem);
        }
        // Double consonant: mapped → map
        if stem.len() >= 2 {
            let chars: Vec<char> = stem.chars().collect();
            if chars.len() >= 2 && chars[chars.len() - 1] == chars[chars.len() - 2] {
                return stem[..stem.len() - 1].to_string();
            }
        }
        return stem.to_string();
    }
    if lower.ends_with("ly") && lower.len() > 4 {
        // quickly → quick, carefully → careful
        return lower[..lower.len() - 2].to_string();
    }
    if lower.ends_with("er") && lower.len() > 4 {
        // Only strip -er for common agent nouns (runner, builder, etc.)
        // Don't strip for words like "computer", "water", "number"
        let stem = &lower[..lower.len() - 2];
        // Agent nouns typically end in -er after a verb stem
        // Skip if the stem would be < 3 chars (e.g., "her" → "h")
        if stem.len() >= 3 {
            // Check if the stem is a known verb stem — conservative list
            // Only include stems that produce valid agent nouns with -er
            let agent_verbs = [
                "run", "build", "teach", "lead", "read", "speak", "work", "write", "paint", "sing",
                "drive", "swim", "cook", "hunt", "manag", "provid", "creat", "develop",
            ];
            if agent_verbs.contains(&stem) {
                return stem.to_string();
            }
        }
        // For other -er words, keep the original
        return lower;
    }
    if lower.ends_with("tion") && lower.len() > 5 {
        // computation → comput (approximate — good enough for topic clustering)
        return lower[..lower.len() - 4].to_string();
    }

    lower
}

/// Common English stop words — excluded from topic keywords
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
    "from", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do", "does",
    "did", "will", "would", "could", "should", "may", "might", "shall", "can", "it", "its", "this",
    "that", "these", "those", "i", "you", "he", "she", "we", "they", "me", "him", "her", "us",
    "them", "my", "your", "his", "our", "their", "what", "which", "who", "whom", "whose", "where",
    "when", "why", "how", "all", "each", "every", "both", "few", "more", "most", "other", "some",
    "such", "no", "not", "only", "same", "so", "than", "too", "very", "just", "also", "then",
    "there", "here", "if", "about", "up", "out", "into", "over", "after", "before", "between",
    "under", "again", "further", "once", "please", "yes", "ok", "okay", "thanks", "thank",
];

/// Check if a word is a stop word
fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(&word.to_lowercase().as_str())
}

/// Conditional clause starters
const CONDITIONAL_STARTERS: &[&str] = &[
    "if ",
    "if\t",
    "when ",
    "when\t",
    "unless ",
    "unless\t",
    "although ",
    "although\t",
    "provided that ",
    "assuming ",
    "given that ",
    "in case ",
    "in the event ",
    "even if ",
    "whether or not ",
];

/// Extract conditional clauses from a sentence
fn extract_conditionals(sentence: &str) -> Vec<String> {
    let lower = sentence.to_lowercase();
    let mut conditionals = Vec::new();

    for starter in CONDITIONAL_STARTERS {
        if let Some(pos) = lower.find(starter) {
            let rest = &sentence[pos + starter.len()..];
            let end_pos = rest.find([',', ';', '\n']).unwrap_or(rest.len());
            let clause = rest[..end_pos].trim();
            if !clause.is_empty() {
                conditionals.push(format!("{}{}", starter.trim(), clause));
            }
        }
    }

    conditionals
}

/// Common imperative starters (verb forms that indicate commands)
const IMPERATIVE_STARTERS: &[&str] = &[
    "tell",
    "show",
    "explain",
    "describe",
    "write",
    "create",
    "make",
    "give",
    "list",
    "find",
    "search",
    "look",
    "check",
    "run",
    "execute",
    "compute",
    "calculate",
    "analyze",
    "compare",
    "summarize",
    "translate",
    "convert",
    "fix",
    "debug",
    "refactor",
    "implement",
    "add",
    "remove",
    "delete",
    "update",
    "help",
    "please",
    "let",
    "do",
    "try",
    "use",
];

/// Decompose a single sentence into structured components
fn decompose_sentence(sentence: &str) -> SentenceDecomposition {
    let trimmed = sentence.trim();

    // Detect question
    let is_question = trimmed.ends_with('?');

    // Detect imperative: starts with a verb (common command patterns)
    let first_word = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    let first_lemma = lemmatize(&first_word);
    let is_imperative = !is_question
        && (IMPERATIVE_STARTERS.contains(&first_word.as_str())
            || IMPERATIVE_STARTERS.contains(&first_lemma.as_str()));

    // Extract conditional clauses
    let conditionals = extract_conditionals(trimmed);

    // Tokenize into words
    let words: Vec<&str> = trimmed
        .split([' ', '\t', '\n'])
        .filter(|w| !w.is_empty())
        .collect();

    // Simple subject-verb-predicate extraction:
    // - Before first verb: likely subjects
    // - First verb encountered: main verb
    // - After verb: predicates/objects
    let mut subjects = Vec::new();
    let mut verbs = Vec::new();
    let mut predicates = Vec::new();

    let mut found_verb = false;
    for word in &words {
        let clean = word.trim_matches(|c: char| {
            c == ','
                || c == '.'
                || c == ';'
                || c == ':'
                || c == '!'
                || c == '?'
                || c == '"'
                || c == '\''
                || c == '('
                || c == ')'
        });

        if clean.is_empty() || is_stop_word(clean) {
            continue;
        }

        let lemma = lemmatize(clean);

        let is_likely_verb = clean.ends_with("ing")
            || clean.ends_with("ed")
            || (clean.ends_with("s") && clean.len() > 3 && !clean.ends_with("ss"))
            || clean.ends_with("es")
            || IMPERATIVE_STARTERS.contains(&lemma.as_str())
            || [
                "be", "have", "do", "will", "would", "could", "should", "may", "might", "shall",
                "can", "must",
            ]
            .contains(&lemma.as_str());

        if is_likely_verb && !found_verb {
            verbs.push(lemma);
            found_verb = true;
        } else if !found_verb {
            subjects.push(lemma);
        } else {
            predicates.push(lemma);
        }
    }

    SentenceDecomposition {
        subjects,
        verbs,
        predicates,
        conditionals,
        is_question,
        is_imperative,
    }
}

/// Split text into sentences
fn split_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;

    for (i, c) in text.char_indices() {
        if c == '.' || c == '!' || c == '?' {
            let remaining = &text[i + 1..];
            if remaining.starts_with(' ') || remaining.starts_with('\n') || remaining.is_empty() {
                let sentence = text[start..i + 1].trim();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
                start = i + 1;
                while start < text.len() && text.as_bytes().get(start) == Some(&b' ')
                    || text.as_bytes().get(start) == Some(&b'\n')
                {
                    start += 1;
                }
            }
        } else if c == '\n' {
            let sentence = text[start..i].trim();
            if !sentence.is_empty() {
                sentences.push(sentence);
            }
            start = i + 1;
        }
    }

    // Last fragment
    let sentence = text[start..].trim();
    if !sentence.is_empty() {
        sentences.push(sentence);
    }

    sentences
}

/// Decompose a full prompt into structured variety signals.
///
/// Input: the full prompt (system prompt + user input + any semantic context)
/// Output: `PromptAnalysis` with depth/structure/domain buckets for CNS tracking.
///
/// REQ: P9-agt-prompt-classify
/// [P9] Motivating: Homeostatic Self-Regulation — classify prompt to guide loop action
/// pre:  `prompt` is a valid UTF-8 string (may be empty).
/// post: Returns a `PromptAnalysis` with sentence decompositions, clause
///       density, depth bucket (shallow/medium/deep), topic keywords,
///       verb diversity, and counts of conditionals/questions/imperatives.
///       An empty prompt yields zero counts and density 0.0.
pub fn decompose_prompt(prompt: &str) -> PromptAnalysis {
    let sentences_raw = split_sentences(prompt);
    let sentence_decompositions: Vec<SentenceDecomposition> = sentences_raw
        .iter()
        .map(|s| decompose_sentence(s))
        .collect();

    let sentence_count = sentence_decompositions.len();
    let question_count = sentence_decompositions
        .iter()
        .filter(|s| s.is_question)
        .count();
    let imperative_count = sentence_decompositions
        .iter()
        .filter(|s| s.is_imperative)
        .count();

    let total_predicates: usize = sentence_decompositions
        .iter()
        .map(|s| s.predicates.len())
        .sum();
    let conditional_count: usize = sentence_decompositions
        .iter()
        .map(|s| s.conditionals.len())
        .sum();

    let clause_density = if sentence_count > 0 {
        (total_predicates + conditional_count) as f64 / sentence_count as f64
    } else {
        0.0
    };

    let depth_bucket = if clause_density < 2.0 {
        "shallow"
    } else if clause_density <= 4.0 {
        "medium"
    } else {
        "deep"
    };

    // Collect all unique topic keywords (lemmatized subjects)
    let mut topic_set = std::collections::BTreeSet::new();
    let mut verb_set = std::collections::BTreeSet::new();
    for sd in &sentence_decompositions {
        for subject in &sd.subjects {
            topic_set.insert(subject.clone());
        }
        for verb in &sd.verbs {
            verb_set.insert(verb.clone());
        }
    }

    PromptAnalysis {
        sentences: sentence_decompositions,
        sentence_count,
        clause_density,
        depth_bucket,
        topic_keywords: topic_set.into_iter().collect(),
        verb_diversity: verb_set.len(),
        conditional_count,
        question_count,
        imperative_count,
    }
}
