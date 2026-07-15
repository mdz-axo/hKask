//! Corpus pipeline types — shared between MCP server (hkask-mcp-docproc)
//! and pipeline tools.
//!
//! Single source of truth for the TaggedChunk type that flows through the
//! pipeline: tag → dedup → consolidate → build-prompts → ingest-qa.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dublin Core + PKO metadata attached to consolidated chunks.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ChunkOntology {
    /// Dublin Core type (always "bibo:Document" for consolidated chunks).
    pub dc_type: String,
    /// Dublin Core subject — the concepts as ontology terms.
    pub dc_subject: Vec<String>,
    /// Dublin Core source — the original source file.
    pub dc_source: String,
    /// PKO provenance — wasExtractedFrom the original chunk refs.
    pub pko_extracted_from: Vec<String>,
}

/// A chunk annotated with multi-dimensional ontology tags.
///
/// This is the canonical type that flows through the entire corpus pipeline.
/// The MCP server (hkask-mcp-docproc) uses this struct — no local duplicates.
///
/// Design: open-world ontology tagging.
/// - 5W1H dimensions and Dublin Core are structural (every chunk has them)
/// - Domain-specific ontologies (FIBO, GOLEM, OMC, PKO, etc.) are stored in
///   `ontology_tags` — a flexible map keyed by namespace. Adding a new
///   ontology doesn't require changing this struct.
/// - `concepts` is a convenience cache = union of all ontology_tags values.
///
/// Pipeline flow:
///   tag-chunks → writes TaggedChunk to JSONL
///   dedup-chunks → reads TaggedChunk, writes subset
///   consolidate-chunks → reads TaggedChunk, writes merged TaggedChunk
///   build-prompts → reads TaggedChunk, generates QA prompts
///   ingest-qa → reads TaggedChunk metadata for sidecar annotations
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TaggedChunk {
    /// Unique entity reference (e.g., "corpus:researcher:Damodaran-ROIC_pdf_txt:119").
    pub entity_ref: String,
    /// Source file name (e.g., "Damodaran-ROIC.pdf.txt").
    pub source: String,
    /// The chunk text content.
    pub text: String,
    /// Word count from the original chunking phase.
    #[serde(default)]
    #[allow(dead_code)]
    pub word_count: usize,

    // ── Structural tags (always present) ─────────────────────────────────
    /// 5W1H interrogatory dimensions (Who/What/When/Where/Why/How). Multiple allowed.
    /// Universal ground — every chunk answers at least one interrogatory.
    #[serde(default)]
    pub dimensions: Vec<String>,

    /// Dublin Core BIBO type (e.g., "bibo:Book", "bibo:Article").
    #[serde(default)]
    pub dc_type: String,

    /// Dublin Core subject keywords — general topic classification.
    #[serde(default)]
    pub dc_subject: Vec<String>,

    /// Expertise level: "practitioner", "analyst", or "researcher".
    #[serde(default)]
    pub expertise_level: String,

    // ── Flexible ontology tags (open-world) ──────────────────────────────
    /// Domain-specific ontology concepts, keyed by namespace.
    /// Examples:
    ///   {"fibo": ["competitive advantage", "ROIC"], "golem": ["metaphor"], "omc": ["scene"], "pko": ["analysis"]}
    ///
    /// Adding a new ontology is just a new key — no struct change needed.
    /// The tagging LLM determines which ontologies are relevant per passage.
    #[serde(default)]
    pub ontology_tags: HashMap<String, Vec<String>>,

    /// Union of all ontology_tags values — convenience cache for downstream
    /// consumers that need the flat concept list without caring which ontology
    /// each concept came from.
    #[serde(default)]
    pub concepts: Vec<String>,

    // ── Computed scores ──────────────────────────────────────────────────
    /// Graph-centrality salience score [0.0, 1.0].
    #[serde(default)]
    pub salience: f32,

    // ── Consolidation provenance ─────────────────────────────────────────
    /// Original chunk refs this chunk was consolidated from (pko:wasExtractedFrom).
    /// Empty for pass-through (singleton) chunks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consolidated_from: Vec<String>,

    /// Dublin Core + PKO metadata for consolidated chunks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ontology: Option<ChunkOntology>,
}

impl TaggedChunk {
    /// Get concepts from a specific ontology namespace.
    /// Returns empty slice if the namespace isn't present.
    pub fn ontology_concepts(&self, namespace: &str) -> &[String] {
        self.ontology_tags
            .get(namespace)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Which ontology namespaces are present in this chunk's tags?
    pub fn ontology_namespaces(&self) -> impl Iterator<Item = &str> {
        self.ontology_tags.keys().map(|k| k.as_str())
    }

    /// Does this chunk have tags from the given ontology namespace?
    pub fn has_ontology(&self, namespace: &str) -> bool {
        self.ontology_tags.contains_key(namespace)
    }
}
