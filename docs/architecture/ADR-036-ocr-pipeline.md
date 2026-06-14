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

hKask's document processing pipeline (`hkask-mcp-docproc`) requires OCR capabilities for extracting text from images and scanned PDFs. The pipeline must support multiple OCR backends (Tesseract, PaddleOCR, fal.ai) with deterministic routing based on document characteristics.

The architecture was developed incrementally across multiple agent sessions (handoffs: `ocr-fal-self-2026-06-13.md`, `media-server-continuation-2026-06-14.md`) and encoded in the type system of `hkask-mcp-markitdown`. This ADR retroactively documents the decisions.

## Decision

### Sealed Type Hierarchy for Backend Selection

OCR backends are represented as a sealed enum with compile-time exhaustiveness checking:

```rust
pub enum OcrBackend {
    Tesseract,    // Local, offline, English-optimized
    PaddleOCR,    // Local, offline, multi-language
    FalAI,        // Cloud, high-accuracy, multi-language
}
```

### Deterministic Routing

Backend selection is deterministic based on document metadata, not LLM-driven:
- Language detection → PaddleOCR for non-English
- Quality requirements → FalAI for high-accuracy needs
- Offline requirement → Tesseract or PaddleOCR (local only)
- Fallback chain: Tesseract → PaddleOCR → FalAI

### Pluggable Backend Trait

Each backend implements a common `OcrEngine` trait, allowing new backends to be added without changing routing logic.

## Consequences

- **Positive:** Compile-time guarantee that all backends are handled. No runtime `unreachable!()`.
- **Positive:** New backends require only implementing the trait + adding an enum variant.
- **Negative:** FalAI backend requires network access and API key — not available in air-gapped deployments.
- **Negative:** No ADR existed during implementation — architectural knowledge was encoded only in code and handoffs. This ADR rectifies that.

## Procedural Rhetoric

This ADR follows the stewardship principles in PRINCIPLES.md §4:
- **PS-01 (Shared Goal):** Reliable text extraction from diverse document formats.
- **PS-02 (Bounded Lexicon):** `OcrBackend`, `OcrEngine`, deterministic routing, fallback chain.
- **PS-03 (Mode of Play):** Compile-time exhaustiveness + runtime fallback.
- **PS-12 (Invitational Voice):** New backends are invited via trait implementation.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
