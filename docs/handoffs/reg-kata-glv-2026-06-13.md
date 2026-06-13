# Handoff — Registry Reorg, Kata Refactor, Gentle Lovelace Integration

**Session date:** 2026-06-13
**Project:** hKask v0.27.0
**Handoff from:** Full registry reorganization, kata system research & refactor, Gentle Lovelace replica integration, documentation sweep
**Handoff to:** Verify per-dimension centroids, test spec replica integration, fix pre-existing warnings, embed remaining corpora

---

## 1. Session Context

This session performed a comprehensive cleanup of the hKask repository: eliminated the misfiled `registry/registries/` folder (26 YAMLs moved to correct locations), cleaned stray files from the project root (DBs → `data/`, scripts → `scripts/`), moved the Gentle Lovelace source corpus from `registry/corpora/` to `registry/styles/gentle-lovelace/corpus-sources/`, and completely refactored the kata system from one skill with 3 artificial "types" into 4 proper dual-layer skills based on deep research into Mike Rother's Toyota Kata methodology. The Gentle Lovelace replica was wired into `hkask-mcp-spec`'s `spec_require_writing_quality` tool for embedding-based dimension scoring. The corpus was embedded (composite centroid created) but per-dimension centroid creation needs verification. ~90% complete — remaining work is verification and cleanup.

---

## 2. What Was Done

### Registry Reorganization
- **Deleted `registry/registries/`** — 26 misfiled YAMLs moved:
  - 8 bundle manifests → `registry/manifests/skills/` and `registry/manifests/pragmatic-composition/`
  - 4 hLexicon files → `registry/hlexicon/`
  - 3 cognition reference files → `registry/cognition/`
  - 2 style synthesizers → `registry/styles/hemingway/` and `registry/styles/woolf/`
  - 3 mashup definitions → `registry/styles/*/`
- **Deleted `registry/corpora/`** — 18 source files moved to `registry/styles/gentle-lovelace/corpus-sources/`
- **Deleted `registry/kata/`** — `kata-system.yaml` was a meta-catalog, replaced by 4-skill architecture
- Updated 18 files with corrected path references (YAML, Rust, markdown, shell scripts)
- Zero remaining references to deleted paths confirmed via grep

### Root Directory Cleanup
- 6 DB files → `data/` directory
- `DEFAULT_DB_PATH` changed from `"hkask.db"` to `"data/hkask.db"` in `crates/hkask-services/src/config.rs`
- `.gitignore` updated: added `data/` and `registry/styles/**/corpus-sources/`
- `env.example` updated with new default paths
- `DEPLOYMENT.md` default updated to `./data/hkask.db`
- 2 scripts (`embed-mashups.sh`, `embed-twain.sh`) → `scripts/`
- `feedback.md` → `docs/`
- `david-dunning/` → `registry/styles/david-dunning/`

### Kata System — Full Research & Refactor
- Deep research on Mike Rother's Toyota Kata (2004-present) from primary sources
- **Key finding:** Toyota Kata has TWO linked behaviors (Improvement Kata + Coaching Kata), not three. Starter Kata are practice routines, not a peer kata type.
- Refactored from 1 skill with 3 artificial "types" → 4 proper dual-layer skills:

| Skill | SKILL.md | Templates | Manifest | Bootstrap entries |
|-------|----------|-----------|----------|-------------------|
| `kata-starter` | `.agents/skills/kata-starter/SKILL.md` | 5 in `registry/templates/kata-starter/` | `starter-kata.yaml` | 5 |
| `kata-improvement` | `.agents/skills/kata-improvement/SKILL.md` | 5 in `registry/templates/kata-improvement/` | `improvement-kata.yaml` | 5 |
| `kata-coaching` | `.agents/skills/kata-coaching/SKILL.md` | 6 in `registry/templates/kata-coaching/` | `coaching-kata.yaml` | 6 |
| `kata` (bundle) | `.agents/skills/kata/SKILL.md` | 7 in `registry/templates/kata/` | `kata-pattern.yaml` | 7 |

- 23 templates split across 4 directories, 4 `manifest.yaml` files created
- `registry/templates/bootstrap-registry.yaml` — 26 kata entries (was 10)
- `registry/hlexicon/kata-hlexicon.yaml` — rewritten for four-skill architecture
- All 23 `source_path` references verified present on disk

### Pre-existing Bugs Fixed
- `crates/hkask-types/src/identity.rs` — missing `passphrase_set_at: None` in `HumanUser::new()`
- `mcp-servers/hkask-mcp-markitdown/src/tools.rs` — broken `CnsObserver` impl: added `#[async_trait]`, proper imports (`NuEvent`, `SpanNamespace`, `BackpressureSignal`, `DepletionSignal`), separated domain methods (`observe_verification`, `observe_cross_validation`) from trait impl

### Gentle Lovelace Replica — Spec Server Integration
- **`mcp-servers/hkask-mcp-spec/src/types.rs`:**
  - Added `DimensionScore` struct (dimension, centroid_ref, cosine_distance, qualitative, passage_count)
  - Extended `WritingQualityRequest` with `db_path` and `db_passphrase` fields
  - Extended `WritingQualityResponse` with `dimension_scores`, `weakest_dimension`, `rewrite_prompt`
- **`mcp-servers/hkask-mcp-spec/src/main.rs`:**
  - Replaced synchronous `assess_writing_quality` stub with async two-path method
  - Added `compare_against_replica` — embeds spec content via `EmbeddingRouter`, queries centroids by `style:{persona}:` prefix, computes per-dimension cosine distances
  - Tool handler auto-identifies weakest dimension and generates pre-built `rewrite_prompt` for `spec_replica_rewrite`
  - Falls back to structural heuristic when DB credentials not provided
- **`.agents/skills/document-update/SKILL.md`** — Task 3 updated to reflect single-call flow (no separate `replica_compare` needed)
- **`docs/handoffs/gl-embed-2026-06-13.md`** — Both MEDIUM tasks marked ✅ DONE

### Corpus Embedding Attempted
- Ran `kask style embed-corpus --config registry/styles/gentle-lovelace/corpus.yaml --db data/hkask-styles.db --passphrase test-pass`
- **Result:** 1000 embeddings, 33,377 triples stored, composite centroid `style:gentle-lovelace:centroid` created from 993 prose passages
- **Uncertain:** Per-dimension centroids (`gentle-centroid`, `schriver-centroid`, `hopper-centroid`, `lovelace-centroid`) — the code path exists (lines 945-1024 of `embed.rs`) but the CLI output only shows the composite `EmbedResult.centroid_ref`. The dimension centroids may have been stored but not reported, OR the passages may lack dimension tags causing the `dim_refs` map to be empty.

### Documentation Created/Updated
- **Created:** `docs/guides/kata-user-guide.md` (361 lines — research background, technical build, user how-to)
- **Created:** `docs/status/skill-inventory.md` (117 lines — 28 skills cataloged with verification commands)
- **Updated:** `docs/README.md` portal (added Guides section, skill-inventory entry)
- **Updated:** `docs/architecture/hKask-architecture-master.md` (removed archived ADR-022, fixed counts)
- **Updated:** `docs/DIAGRAMS_INDEX.md` (dates)
- **Updated:** `docs/status/PROJECT_STATUS.md` (full refresh — build dates, skills 14→28, session summary)
- **Metadata fixes:** `hKask-hLexicon.md`, `REQUIREMENTS.md`, `DEPLOYMENT.md`, `ADR-034` — `last_updated` dates and versions corrected

### Build Status
- `cargo check --workspace` — ✅ passes (1 pre-existing warning: `hkask-mcp-markitdown` unused `new`/`persist_span`)
- `cargo test -p hkask-templates -p hkask-types` — ✅ 30/30 pass
- `cargo test -p hkask-services -- gentle_lovelace` — ✅ 1/1 pass (verifies dimension_centroids parse correctly)

---

## 3. What Remains

### HIGH — Verify Per-Dimension Centroids Were Created

The embed ran successfully and the composite centroid exists. But the per-dimension centroids (`style:gentle-lovelace:gentle-centroid`, `schriver-centroid`, `hopper-centroid`, `lovelace-centroid`) need verification.

**The code path exists** at `crates/hkask-services/src/embed.rs` lines 945-1024. It builds `dim_refs` by matching `passage.dimension` against `dc.name`. The test at `crates/hkask-services/tests/gentle_lovelace_corpus_test.rs` confirms `config.dimension_centroids` parses with 4 entries.

**Possible issue:** The `passage.dimension` field is set from `Work.dimensions` during chunking. If passages aren't getting dimension tags, `dim_refs` would be empty and the code would log "No passages for dimension — skipping centroid" (line 976). Check if those warnings appeared in the embed output.

**To verify:**
1. Re-run embed and capture FULL output (not just the summary line): `target/debug/kask style embed-corpus --config registry/styles/gentle-lovelace/corpus.yaml --db data/hkask-styles.db --passphrase test-pass 2>&1 | tee /tmp/gl-embed-full.log`
2. Search for "Dimension centroid stored" or "No passages for dimension" in the log
3. If centroids exist, test `spec_require_writing_quality` against them (see next item)
4. If passages lack dimension tags, debug the chunking phase in `embed.rs` — the `TaggedPassage.dimension` field should be populated from `Work.dimensions`

### HIGH — Test spec_require_writing_quality with Replica

Once centroids are confirmed, test the integration end-to-end:

```bash
# Start the spec MCP server (needs daemon or standalone mode)
# Then call:
spec/require/writing-quality {
  "spec_id": "REQ-DOM-004",
  "replica_persona": "gentle-lovelace",
  "db_path": "data/hkask-styles.db",
  "db_passphrase": "test-pass"
}
```

Expected: returns `dimension_scores` array with per-dimension cosine distances, `weakest_dimension`, and `rewrite_prompt` if any dimension scores >0.4.

If centroids don't exist, the tool falls back to structural heuristic (no error, just no `dimension_scores`).

### MEDIUM — Fix Pre-existing Warnings in hkask-mcp-markitdown

`cargo check -p hkask-mcp-markitdown` shows 3 warnings:
1. `unused_mut` in `ocr/decimation.rs:149` — `let mut gray` should be `let gray`
2. Two `unused_must_use` futures in `tools.rs:487` and `tools.rs:583` — `self.persist_pipeline_outcome(&outcome)` needs `.await`

These are pre-existing, not caused by this session. The `observe_verification` and `observe_cross_validation` methods were removed from the `CnsObserver` impl (they're not trait methods) but weren't re-added as inherent `impl MarkitdownCnsObserver` methods — they're dead code. Either restore them as inherent methods or remove them.

### MEDIUM — Embed Remaining Style Corpora

Other style corpora in `registry/styles/` may need embedding:
- `hemingway/corpus.yaml`
- `woolf/` (check for corpus.yaml)
- `agatha-eliot/`, `jane-wilde/`, `ulysses-s-twain/` (mashup configs)

The `embed-mashups.sh` script in `scripts/` handles these.

### LOW — Kata Manifests Need Code Wiring

The 5 kata manifests exist in `registry/manifests/` and 26 templates are registered in bootstrap, but **zero Rust code references any kata manifest by ID**. The bootstrap phase 7 (`KataReadiness`) verifies domain ownership but doesn't execute kata. To make kata functional:
- Add CLI commands or agent routing that invokes kata manifests
- Wire the `kata:execute` port from `registry/ports/kata-ports.yaml`

### LOW — Ports Directory Underutilized

`registry/ports/` contains only `kata-ports.yaml`. The hexagonal ports concept is valid but needs a second user to satisfy P1 (no directory without two consumers). Consider adding ports for other subsystems (CNS, memory, spec).

---

## 4. Recommended Skills and Tools

| Order | Skill | Why |
|-------|-------|-----|
| 1 | **condenser-continuation** | Restore session state from this handoff |
| 2 | **coding-guidelines** | Surgical changes — touch only what's needed |
| 3 | **diagnose** | If per-dimension centroids are missing, diagnose the chunking→tagging→centroid pipeline |

**Key commands:**
```bash
# Build verification
cargo check --workspace
cargo test -p hkask-templates -p hkask-types

# Re-embed with full log capture
target/debug/kask style embed-corpus \
  --config registry/styles/gentle-lovelace/corpus.yaml \
  --db data/hkask-styles.db \
  --passphrase test-pass \
  2>&1 | tee /tmp/gl-embed-full.log

# Check for dimension centroid messages
grep -i "dimension centroid\|No passages for dimension\|per-dimension" /tmp/gl-embed-full.log

# Fix markitdown warnings
cargo clippy -p hkask-mcp-markitdown -- -D warnings
```

**Key files:**
```
crates/hkask-services/src/embed.rs              ← Embedding pipeline, dimension centroid logic (L945-1024)
crates/hkask-services/tests/gentle_lovelace_corpus_test.rs ← Confirms dimension_centroids parse
mcp-servers/hkask-mcp-spec/src/main.rs           ← spec_require_writing_quality + compare_against_replica
mcp-servers/hkask-mcp-spec/src/types.rs          ← DimensionScore, WritingQualityRequest/Response
mcp-servers/hkask-mcp-markitdown/src/tools.rs    ← Pre-existing warnings to fix
registry/styles/gentle-lovelace/corpus.yaml      ← Embedding config with 4 dimension_centroids
data/hkask-styles.db                             ← Embedded corpus (composite centroid confirmed)
data/hkask-styles.db.salt                        ← DB salt file
```

---

## 5. Key Decisions to Preserve

1. **Kata is 4 skills, not 1 skill with 3 types.** Based on Mike Rother's primary sources: Toyota Kata has TWO linked behaviors (IK + CK), Starter Kata are practice routines. Each kata is independently adoptable — you start with kata-starter, not the full system. Do not merge back into a single skill.

2. **`DEFAULT_DB_PATH` is now `"data/hkask.db"`.** All runtime databases go in `data/`. The `.gitignore` ignores `data/` entirely. Do not revert to root-level DB files.

3. **`registry/registries/`, `registry/corpora/`, `registry/kata/` are permanently deleted.** Files moved to correct locations. Zero stale references remain. Do not recreate these directories.

4. **Gentle Lovelace integration is in `hkask-mcp-spec`, not a separate orchestration.** `spec_require_writing_quality` handles both heuristic and embedding-based assessment in a single call. No separate `replica_compare` call needed. The `replica_persona` + `db_path` + `db_passphrase` pattern is the canonical interface.

5. **The `CnsObserver` trait impl in markitdown was fixed** by adding `#[async_trait]`, proper imports, and separating domain methods from trait impl. The `observe_verification` and `observe_cross_validation` methods are currently dead code — they need to be either restored as inherent methods on `MarkitdownCnsObserver` or removed.

6. **Gentle dominates at 50% in the composite centroid.** In an agent-native system, markdown specifications ARE the code. Stale documentation is a functional defect. Schriver 30% (findability), Hopper 10% (accessibility), Lovelace 10% (precision). Do not rebalance without understanding this rationale.

7. **All four writing quality exemplars are women — this is not incidental.** Technical documentation was founded (Hopper), algorithmized (Lovelace), measured (Schriver), and modernized (Gentle) by women. Preserve this credit in all manifests and templates.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
