# Lexicon Audit Report (E1)

**Date:** 2026-07-21 · **Author:** agent (Zed)

## Summary

The lexicon is a **controlled vocabulary** — not a free-form tag list as initially suspected. It consists of:

- **`KNOWN_TERMS`** in `crates/hkask-templates/src/vocabulary.rs`: 347 entries, sorted, binary-search lookup
- **`is_known()`**, **`is_well_formed()`**, **`validate_entry()`**, **`unrecognized()`** — public validation functions
- **Integration test** `tests/lexicon_coverage.rs` — enforces every manifest's `lexicon_terms` are known and well-formed
- **Enforcement** (as of this audit): `validate_entry` now returns an **error** (not a warning) on unknown terms — implemented in `registry.rs` and `registry_sqlite.rs`

## Data

| Metric | Count |
|---|---|
| Distinct `lexicon_terms` values across all manifests | 293 |
| Entries in `KNOWN_TERMS` | 347 |
| Terms in manifests but NOT in `KNOWN_TERMS` | 3 real (`accommodate`, `engage`, `repair`) + 4 test artifacts (`<term1>`, `<term2>`, `term`, blank) |
| Terms in `KNOWN_TERMS` but NOT in manifests | ~54 (unused but available — not a problem) |
| Production callers of `validate_entry` | 2 (`registry.rs:194`, `registry_sqlite.rs:153`) |

## Verdict

The lexicon is **already a closed vocabulary** with validation. The gap was enforcement (warnings → errors), which has been fixed. No `LexiconTerm` enum is needed — the `&[&str]` array + binary search + `validate_entry` → error is the right shape.

**Action needed:** Add the 3 missing terms (`accommodate`, `engage`, `repair`) to `KNOWN_TERMS` in `vocabulary.rs`.

## Recommendation

- **Option B (closed vocabulary with validation)** is the correct choice — already implemented.
- **Option A (delete + full-text search)** rejected — the vocabulary provides semantic grounding (P8).
- **Option C (replace with typed enums)** rejected — 347 variants is too large for a deep module (depth score = 1.0).
- **E2 (LexiconTerm enum)** rejected — not needed; the array + validation is sufficient.
