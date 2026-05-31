//! hLexicon source-of-truth alignment.
//!
//! Architectural lifecycle of the hLexicon (three file types, three lifecycles):
//!
//! 1. `docs/architecture/reference/hKask-hLexicon.md` — **canonical**, authored
//!    by humans. The single source of truth for the vocabulary.
//! 2. `registry/registries/hlexicon-workspace.yaml` — **derived data**. A
//!    committed artifact regenerated from the markdown. It has a data lifecycle:
//!    it can evolve and be customized (e.g. subsystem registries) in ways the
//!    compiled Rust cannot.
//! 3. Rust ([`hkask_types::lexicon`]) — **compiled types**, not user-editable.
//!
//! The markdown → YAML derivation is explicit and human-driven, never silent:
//! - [`parse_markdown_catalog`] reads the canonical markdown.
//! - [`render_workspace_yaml`] produces the derived YAML text.
//! - The `hlexicon_yaml_matches_markdown` test is a **consistency check**: it
//!   fails if the committed YAML and the markdown disagree, so the maintainer is
//!   asked to decide whether the markdown was corrupted (restore from git) or
//!   intentionally evolved (regenerate).
//! - The `regenerate_workspace_yaml` test is the **explicit, opt-in** regen step
//!   (`#[ignore]`d; run manually with `--ignored` only when you intend to update
//!   the YAML to reflect markdown evolution).
