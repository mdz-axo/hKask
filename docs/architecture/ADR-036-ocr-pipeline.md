---
title: "ADR-036 — OCR Pipeline Architecture"
audience: [architects, developers]
last_updated: 2026-06-14
version: "0.27.0"
status: "Draft"
domain: "Technology"
mds_categories: [composition, curation]
---

# ADR-036 — OCR Pipeline Architecture

**Status:** Draft  
**Date:** 2026-06-14  
**Supersedes:** `docs/architecture/OCR-PIPELINE-INSIGHTS.md` (archived 2026-06-14)

---

## Context

hKask's document processing pipeline (`hkask-mcp-docproc`) extracts text from images and scanned PDFs via OCR. The pipeline routes documents to one of three backends (Tesseract, PaddleOCR, fal.ai) based on document characteristics — language, quality requirements, and network availability.

Agent sessions developed the architecture incrementally across multiple handoffs (`ocr-fal-self-2026-06-13.md`, `media-server-continuation-2026-06-14.md`) and encoded it in the type system of `hkask-mcp-markitdown`. This ADR documents those decisions retroactively.

## Decision

### Sealed Type Hierarchy for Backend Selection

OCR backends use a sealed enum. The compiler enforces exhaustiveness — every backend variant must have a handler:

```rust
pub enum OcrBackend {
    Tesseract,    // Local, offline, English-optimized
    PaddleOCR,    // Local, offline, multi-language
    FalAI,        // Cloud, high-accuracy, multi-language
}
```

### Deterministic Routing

The router selects backends deterministically from document metadata. No LLM participates in routing decisions:
- Language detection → PaddleOCR for non-English
- Quality requirements → FalAI for high-accuracy needs
- Offline requirement → Tesseract or PaddleOCR (local only)
- Fallback chain: Tesseract → PaddleOCR → FalAI

### Pluggable Backend Trait

Each backend implements the `OcrEngine` trait. Adding a new backend requires only implementing the trait and adding an enum variant — routing logic stays unchanged.

## Consequences

- **Positive:** The compiler guarantees every backend variant has a handler. No runtime `unreachable!()`.
- **Positive:** New backends require only a trait implementation and an enum variant.
- **Negative:** The FalAI backend needs network access and an API key. Air-gapped deployments cannot use it.
- **Negative:** Architectural knowledge lived only in code and handoffs until this ADR. This document closes that gap.

## Procedural Rhetoric

This ADR follows the stewardship principles in PRINCIPLES.md §4:
- **PS-01 (Shared Goal):** Reliable text extraction from diverse document formats.
- **PS-02 (Bounded Lexicon):** `OcrBackend`, `OcrEngine`, deterministic routing, fallback chain.
- **PS-03 (Mode of Play):** Compile-time exhaustiveness + runtime fallback.
- **PS-12 (Invitational Voice):** New backends are invited via trait implementation.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
