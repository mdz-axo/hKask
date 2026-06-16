#!/usr/bin/env python3
"""Realign hkask-memory REQ contract IDs to P3-mem-* format."""

import re
from pathlib import Path

ROOT = Path("crates/hkask-memory/src")

OLD_TO_NEW = {
    "MEM-001": "P3-mem-recall-eav-hash",
    "MEM-002": "P3-mem-recall-dedup-triples",
    "MEM-003": "P3-mem-consolidation-bridge-new",
    "MEM-004": "P3-mem-consolidation-bridge-consolidate",
    "MEM-005": "P3-mem-consolidation-candidate-count",
    "MEM-006": "P3-mem-episodic-loop-new",
    "MEM-007": "P3-mem-episodic-loop-with-consolidation",
    "MEM-008": "P3-mem-episodic-loop-storage-budget",
    "MEM-009": "P3-mem-ranking-rrf-score",
    "MEM-010": "P3-mem-ranking-parse-age",
    "MEM-011": "P3-mem-ranking-normalize-date-bucket",
    "MEM-012": "P3-mem-consolidation-service-new",
    "MEM-013": "P3-mem-consolidation-service-consolidate",
    "MEM-014": "P3-mem-consolidation-service-candidate-count",
    "MEM-015": "P3-mem-consolidation-service-low-confidence-count",
    "MEM-016": "P3-mem-consolidation-service-triple-count",
    "MEM-017": "P3-mem-semantic-loop-new",
    "MEM-018": "P3-mem-semantic-loop-with-budget",
    "MEM-019": "P3-mem-semantic-loop-with-budget-threshold",
    "MEM-020": "P3-mem-semantic-loop-storage-budget",
    "MEM-021": "P3-mem-semantic-loop-low-confidence-threshold",
    "MEM-022": "P3-mem-episodic-memory-new",
    "MEM-023": "P3-mem-episodic-store",
    "MEM-024": "P3-mem-episodic-query-deduped",
    "MEM-025": "P3-mem-episodic-storage-usage",
    "MEM-026": "P3-mem-episodic-storage-budget",
    "MEM-027": "P3-mem-episodic-candidate-count",
    "MEM-028": "P3-mem-salience-method-signals",
    "MEM-029": "P3-mem-salience-declared-method-matches",
    "MEM-030": "P3-mem-salience-tag-entities",
    "MEM-031": "P3-mem-salience-all-tags",
    "MEM-032": "P3-mem-salience-tag-count",
    "MEM-033": "P3-mem-salience-compute-batch",
    "MEM-034": "P3-mem-salience-budget-resolve",
    "MEM-035": "P3-mem-semantic-memory-new",
    "MEM-036": "P3-mem-semantic-query-deduped",
    "MEM-037": "P3-mem-semantic-store",
    "MEM-038": "P3-mem-semantic-triple-count",
    "MEM-039": "P3-mem-semantic-triple-count-entity",
    "MEM-040": "P3-mem-semantic-query-attribute",
    "MEM-041": "P3-mem-semantic-store-embedding",
    "MEM-042": "P3-mem-semantic-search-similar",
    "MEM-043": "P3-mem-semantic-embedding-count",
    "MEM-044": "P3-mem-semantic-embedding-store",
    "MEM-045": "P3-mem-semantic-compute-centroid",
    "MEM-046": "P3-mem-semantic-purge-prefix",
    "MEM-047": "P3-mem-semantic-chunk-text",
    "MEM-048": "P3-mem-semantic-strip-gutenberg",
    "MEM-049": "P3-mem-semantic-delete-triple",
    "MEM-050": "P3-mem-semantic-lowest-confidence",
    "MEM-051": "P3-mem-semantic-low-confidence-count",
    "MEM-052": "P3-mem-semantic-low-confidence-triples",
    "memory-salience-001": "P3-mem-salience-hemingway-test",
    "memory-salience-002": "P3-mem-salience-wilde-test",
    "memory-salience-003": "P3-mem-salience-declared-method-test",
    "memory-salience-004": "P3-mem-salience-zero-empty-test",
    "memory-salience-005": "P3-mem-salience-shared-entities-test",
    "memory-salience-006": "P3-mem-salience-clustering-zero-test",
    "memory-salience-007": "P3-mem-salience-bridge-higher-test",
    "memory-salience-008": "P3-mem-salience-methods-graph-test",
    "memory-salience-009": "P3-mem-salience-budget-per-page-test",
    "memory-salience-010": "P3-mem-salience-budget-absolute-test",
    "memory-salience-011": "P3-mem-salience-tag-case-insensitive-test",
    "memory-salience-012": "P3-mem-salience-dialogue-ratio-test",
    "semantic-001": "P3-mem-semantic-centroid-dimensions-test",
    "semantic-002": "P3-mem-semantic-centroid-short-test",
}

ANNOTATIONS = {
    "P3-mem-recall-eav-hash": (
        "[P3] Motivating: Generative Space — canonical recall dedup enables reuse of factual content across memory",
        "[P8] Constraining: Semantic Grounding — deterministic BLAKE3 hash over canonical EAV content",
    ),
    "P3-mem-recall-dedup-triples": (
        "[P3] Motivating: Generative Space — deduplication preserves generative storage budget",
        "[P5] Constraining: Essentialism — first-seen wins, no speculative retention policy",
    ),
    "P3-mem-consolidation-bridge-new": (
        "[P3] Motivating: Generative Space — bridges episodic experience into shared semantic memory",
        "[P4] Constraining: Clear Boundaries — links stores without bypassing their membranes",
    ),
    "P3-mem-consolidation-bridge-consolidate": (
        "[P3] Motivating: Generative Space — promotes sovereign episodic triples to shared knowledge",
        "[P1] Constraining: User Sovereignty — strips perspective only under Curator authority; [P4] Constraining: Clear Boundaries — requires ConsolidationToken from expected curator",
    ),
    "P3-mem-consolidation-candidate-count": (
        "[P3] Motivating: Generative Space — surfaces how much episodic content is ready for promotion",
        "[P9] Constraining: Homeostatic Self-Regulation — count-only query avoids loading full store",
    ),
    "P3-mem-episodic-loop-new": (
        "[P3] Motivating: Generative Space — wraps episodic memory in a regulated generative loop",
        "[P9] Constraining: Homeostatic Self-Regulation — storage_budget is the cybernetic set-point",
    ),
    "P3-mem-episodic-loop-with-consolidation": (
        "[P3] Motivating: Generative Space — enables promotion path when episodic budget is exceeded",
        "[P9] Constraining: Homeostatic Self-Regulation — consolidation bridge fires only under token authority",
    ),
    "P3-mem-episodic-loop-storage-budget": (
        "[P3] Motivating: Generative Space — exposes the generative budget set-point for context assembly",
        "[P9] Constraining: Homeostatic Self-Regulation — budget value is immutable after construction",
    ),
    "P3-mem-ranking-rrf-score": (
        "[P3] Motivating: Generative Space — fuses rank positions for context retrieval",
        "[P8] Constraining: Semantic Grounding — reciprocal rank fusion is a standard ranking signal",
    ),
    "P3-mem-ranking-parse-age": (
        "[P3] Motivating: Generative Space — converts human-readable age strings into comparable temporal signals",
        "[P5] Constraining: Essentialism — returns -1.0 for unparseable input, no exceptions",
    ),
    "P3-mem-ranking-normalize-date-bucket": (
        "[P3] Motivating: Generative Space — buckets parsed age into human-readable recency labels",
        "[P8] Constraining: Semantic Grounding — five fixed buckets preserve stable ordering",
    ),
    "P3-mem-consolidation-service-new": (
        "[P3] Motivating: Generative Space — user-facing entry point for memory consolidation and cleanup",
        "[P4] Constraining: Clear Boundaries — requires Curator-issued ConsolidationToken",
    ),
    "P3-mem-consolidation-service-consolidate": (
        "[P3] Motivating: Generative Space — combines episodic promotion with semantic cleanup",
        "[P9] Constraining: Homeostatic Self-Regulation — enforces confidence floor and max triple limits; [P4] Constraining: Clear Boundaries — delegates to token-gated bridge",
    ),
    "P3-mem-consolidation-service-candidate-count": (
        "[P3] Motivating: Generative Space — reports how many episodic triples can be promoted",
        "[P9] Constraining: Homeostatic Self-Regulation — count-only, graceful degradation on error",
    ),
    "P3-mem-consolidation-service-low-confidence-count": (
        "[P3] Motivating: Generative Space — reports low-confidence semantic triples for cleanup",
        "[P9] Constraining: Homeostatic Self-Regulation — threshold-driven pruning signal",
    ),
    "P3-mem-consolidation-service-triple-count": (
        "[P3] Motivating: Generative Space — reports total semantic memory size",
        "[P9] Constraining: Homeostatic Self-Regulation — count used for budget monitoring",
    ),
    "P3-mem-semantic-loop-new": (
        "[P3] Motivating: Generative Space — wraps semantic memory in a regulated knowledge loop",
        "[P9] Constraining: Homeostatic Self-Regulation — default budget and low-confidence threshold are set-points",
    ),
    "P3-mem-semantic-loop-with-budget": (
        "[P3] Motivating: Generative Space — customizes storage budget per user or agent",
        "[P9] Constraining: Homeostatic Self-Regulation — configurable set-point for memory homeostasis",
    ),
    "P3-mem-semantic-loop-with-budget-threshold": (
        "[P3] Motivating: Generative Space — customizes both budget and cleanup threshold",
        "[P7] Constraining: Evolutionary Architecture — thresholds emerge from usage patterns",
    ),
    "P3-mem-semantic-loop-storage-budget": (
        "[P3] Motivating: Generative Space — exposes the semantic storage set-point",
        "[P9] Constraining: Homeostatic Self-Regulation — immutable budget reference for regulation",
    ),
    "P3-mem-semantic-loop-low-confidence-threshold": (
        "[P3] Motivating: Generative Space — exposes the low-confidence cleanup set-point",
        "[P9] Constraining: Homeostatic Self-Regulation — threshold triggers pruning of uncertain knowledge",
    ),
    "P3-mem-episodic-memory-new": (
        "[P3] Motivating: Generative Space — creates a sovereign first-person experience store",
        "[P9] Constraining: Homeostatic Self-Regulation — default decay and budget are regulation defaults",
    ),
    "P3-mem-episodic-store": (
        "[P3] Motivating: Generative Space — stores a first-person experience triple",
        "[P1] Constraining: User Sovereignty — rejects Public visibility (episodic is sovereign); [P4] Constraining: Clear Boundaries — requires perspective owner",
    ),
    "P3-mem-episodic-query-deduped": (
        "[P3] Motivating: Generative Space — recalls deduplicated episodic triples for an entity",
        "[P9] Constraining: Homeostatic Self-Regulation — applies confidence decay and temporal attention at recall",
    ),
    "P3-mem-episodic-storage-usage": (
        "[P3] Motivating: Generative Space — reports episodic storage usage per perspective",
        "[P9] Constraining: Homeostatic Self-Regulation — COUNT query avoids loading full store",
    ),
    "P3-mem-episodic-storage-budget": (
        "[P3] Motivating: Generative Space — exposes the episodic storage set-point",
        "[P9] Constraining: Homeostatic Self-Regulation — budget bounds per-agent experience growth",
    ),
    "P3-mem-episodic-candidate-count": (
        "[P3] Motivating: Generative Space — reports how many episodic triples are eligible for consolidation",
        "[P9] Constraining: Homeostatic Self-Regulation — uses decayed confidence for prioritization",
    ),
    "P3-mem-salience-method-signals": (
        "[P3] Motivating: Generative Space — extracts cheap stylometric signals for method-aware retrieval",
        "[P8] Constraining: Semantic Grounding — signals are deterministic heuristics over raw text",
    ),
    "P3-mem-salience-declared-method-matches": (
        "[P3] Motivating: Generative Space — matches passage signals against declared method thresholds",
        "[P8] Constraining: Semantic Grounding — unconfigured thresholds are always satisfied",
    ),
    "P3-mem-salience-tag-entities": (
        "[P3] Motivating: Generative Space — tags passages with declared entities for the salience graph",
        "[P8] Constraining: Semantic Grounding — case-insensitive substring matching",
    ),
    "P3-mem-salience-all-tags": (
        "[P3] Motivating: Generative Space — flattens entity categories for graph construction",
        "[P5] Constraining: Essentialism — minimal iterator over existing vectors",
    ),
    "P3-mem-salience-tag-count": (
        "[P3] Motivating: Generative Space — counts distinct tags across all categories",
        "[P5] Constraining: Essentialism — simple sum of category lengths",
    ),
    "P3-mem-salience-compute-batch": (
        "[P3] Motivating: Generative Space — scores passage salience to gate triple storage budget",
        "[P9] Constraining: Homeostatic Self-Regulation — graph centrality bounded by neighbor sampling",
    ),
    "P3-mem-salience-budget-resolve": (
        "[P3] Motivating: Generative Space — resolves passage count into absolute triple budget",
        "[P9] Constraining: Homeostatic Self-Regulation — budget caps generative storage growth",
    ),
    "P3-mem-semantic-memory-new": (
        "[P3] Motivating: Generative Space — creates shared semantic knowledge store",
        "[P8] Constraining: Semantic Grounding — unifies triple and embedding stores",
    ),
    "P3-mem-semantic-query-deduped": (
        "[P3] Motivating: Generative Space — recalls deduplicated public semantic triples",
        "[P4] Constraining: Clear Boundaries — filters to Public visibility",
    ),
    "P3-mem-semantic-store": (
        "[P3] Motivating: Generative Space — stores shared semantic triple",
        "[P4] Constraining: Clear Boundaries — requires Public visibility and no perspective",
    ),
    "P3-mem-semantic-triple-count": (
        "[P3] Motivating: Generative Space — reports total shared knowledge triples",
        "[P9] Constraining: Homeostatic Self-Regulation — count feeds storage budget loop",
    ),
    "P3-mem-semantic-triple-count-entity": (
        "[P3] Motivating: Generative Space — reports semantic triples per entity",
        "[P9] Constraining: Homeostatic Self-Regulation — per-entity budget monitoring",
    ),
    "P3-mem-semantic-query-attribute": (
        "[P3] Motivating: Generative Space — queries shared triples by attribute",
        "[P8] Constraining: Semantic Grounding — attribute-based recall expands context",
    ),
    "P3-mem-semantic-store-embedding": (
        "[P3] Motivating: Generative Space — indexes embedding vector for similarity retrieval",
        "[P8] Constraining: Semantic Grounding — vector indexed by triple entity_ref",
    ),
    "P3-mem-semantic-search-similar": (
        "[P3] Motivating: Generative Space — KNN search augments recall beyond exact matches",
        "[P8] Constraining: Semantic Grounding — results ordered by embedding distance",
    ),
    "P3-mem-semantic-embedding-count": (
        "[P3] Motivating: Generative Space — reports indexed embedding count",
        "[P9] Constraining: Homeostatic Self-Regulation — count used for embedding budget monitoring",
    ),
    "P3-mem-semantic-embedding-store": (
        "[P3] Motivating: Generative Space — exposes embedding store for advanced operations",
        "[P5] Constraining: Essentialism — direct accessor avoids duplicate wrappers",
    ),
    "P3-mem-semantic-compute-centroid": (
        "[P3] Motivating: Generative Space — computes mean style vector for corpus validation",
        "[P8] Constraining: Semantic Grounding — arithmetic mean over matching embeddings",
    ),
    "P3-mem-semantic-purge-prefix": (
        "[P3] Motivating: Generative Space — purges embeddings for idempotent re-ingest",
        "[P5] Constraining: Essentialism — prefix-based deletion, count of successes returned",
    ),
    "P3-mem-semantic-chunk-text": (
        "[P3] Motivating: Generative Space — chunks text into passage-sized units for embedding",
        "[P5] Constraining: Essentialism — paragraph/sentence boundary splitting with min/max words",
    ),
    "P3-mem-semantic-strip-gutenberg": (
        "[P3] Motivating: Generative Space — removes boilerplate for clean corpus ingestion",
        "[P5] Constraining: Essentialism — marker-based trim, no regex",
    ),
    "P3-mem-semantic-delete-triple": (
        "[P3] Motivating: Generative Space — deletes semantic triple for budget enforcement or cleanup",
        "[P9] Constraining: Homeostatic Self-Regulation — used by regulation loops to free space",
    ),
    "P3-mem-semantic-lowest-confidence": (
        "[P3] Motivating: Generative Space — identifies lowest-confidence triples for pruning",
        "[P9] Constraining: Homeostatic Self-Regulation — ordered by confidence and age",
    ),
    "P3-mem-semantic-low-confidence-count": (
        "[P3] Motivating: Generative Space — counts uncertain semantic triples",
        "[P9] Constraining: Homeostatic Self-Regulation — threshold-driven count",
    ),
    "P3-mem-semantic-low-confidence-triples": (
        "[P3] Motivating: Generative Space — retrieves uncertain semantic triples for review",
        "[P9] Constraining: Homeostatic Self-Regulation — bounded by threshold and limit",
    ),
}

DYNAMIC = {
    "MEM-001": {
        "test": "P3-mem-salience-valid-range-test",
        "prod": "P3-mem-recall-eav-hash",
    },
    "MEM-002": {
        "test": "P3-mem-salience-empty-tags-proptest",
        "prod": "P3-mem-recall-dedup-triples",
    },
}

REQ_RE = re.compile(r"^(\s*(?:///|//)\s*REQ:\s*)([^—\n]+)(.*)$")


def choose_new_id(old_id: str, line: str) -> str:
    if old_id in DYNAMIC:
        if " — " in line:
            return DYNAMIC[old_id]["test"]
        return DYNAMIC[old_id]["prod"]
    return OLD_TO_NEW[old_id]


def process_file(path: Path) -> None:
    text = path.read_text()
    out_lines = []
    for line in text.splitlines(keepends=True):
        m = REQ_RE.match(line)
        if not m:
            out_lines.append(line)
            continue
        prefix, old_id, rest = m.group(1), m.group(2).strip(), m.group(3)
        if rest:
            rest = re.sub(r"\s*\(P\d+(?:,\s*P\d+)*\)", "", rest)
        new_id = choose_new_id(old_id, line)
        out_lines.append(f"{prefix}{new_id}{rest}\n")
        if new_id in ANNOTATIONS:
            indent = " " * (len(prefix) - len(prefix.lstrip()))
            comment_prefix = "///" if "///" in prefix else "//"
            for ann in ANNOTATIONS[new_id]:
                for sub_ann in ann.split("; "):
                    out_lines.append(f"{indent}{comment_prefix} {sub_ann}\n")
    path.write_text("".join(out_lines))


def main() -> None:
    for p in sorted(ROOT.rglob("*.rs")):
        process_file(p)
    print("hkask-memory realignment complete")


if __name__ == "__main__":
    main()
