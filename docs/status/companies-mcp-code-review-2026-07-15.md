---
title: "hkask-mcp-companies — Adversarial Code Review"
audience: [developers, maintainers]
last_updated: 2026-07-15
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, composition, trust]
last-verified-against: "mcp-servers/hkask-mcp-companies/src/"
---

# hkask-mcp-companies — Adversarial Code Review

A multi-skill adversarial review of the `hkask-mcp-companies` MCP server crate
(41 tools, 14 source files, ~3 800 lines). The review applies six analytical
skills (`improve-codebase-architecture`, `coding-guidelines`, `idiomatic-rust`,
`pragmatic-laziness`, `pragmatic-semantics`, `pragmatic-cybernetics`) and then
challenges its own recommendations through the `essentialist` and `grill-me`
lenses. The goal is to catch issues that a conventional review would miss by
taking a deliberately skeptical posture.

## Methodology

| Skill | Role in this review |
|-------|---------------------|
| `improve-codebase-architecture` | Surface shallow modules, coupling, missing locality |
| `coding-guidelines` | Karpathy's four principles; surgical-change audit |
| `idiomatic-rust` | Hoare's principles: type-driven design, invalid states, ownership |
| `pragmatic-laziness` | Path of least action; delete before adding |
| `pragmatic-semantics` | Classify findings by constraint force (Prohibition→Hypothesis) |
| `pragmatic-cybernetics` | Feedback-loop health, variety balance |
| `essentialist` (challenge) | Delete-test every recommendation: does it earn its keep? |
| `grill-me` (challenge) | Escalating interrogation of each recommendation's rationale |
| `diataxis-diagram` | Documentation currency and missing required diagrams |

Findings are classified by pragmatic-semantics constraint force:

- **Prohibition** — violates an explicit project rule (Magna Carta or CI gate). Must fix.
- **Guardrail** — violates a documented standard or strong convention. Should fix.
- **Guideline** — idiomatic improvement. Worth doing.
- **Evidence** — factual observation about current state. Informational.

Each finding is decomposed into the smallest independently-actionable step so
that no single fix is larger than it needs to be.

---

## Fix status (2026-07-15)

The following findings have been fixed in the codebase:

| Finding | Status | Files changed |
|---------|--------|---------------|
| A1 — CI regex bug | **Fixed** | `scripts/check-string-errors.sh` — `>>` → `>` (one character) |
| A2 — `Result<_, String>` in portfolio.rs | **Fixed** | `src/portfolio.rs` — `PortfolioError` thiserror enum with 4 variants + `From` impls for `rusqlite::Error`, `serde_json::Error`, `LedgerError`, `String`, `&str`; all 25+ function signatures converted; `map_portfolio_error` helper classifies `InvalidArgument` → `invalid_argument`, others → `internal` |
| A2 — `Result<_, String>` in research.rs | **Fixed** | `src/research.rs` — 6 functions converted to `anyhow::Result`; error construction changed to `anyhow!()` macro |
| A2 — `Result<_, String>` in lib.rs | **Fixed** | `src/lib.rs` — `StoredForecast::from_snapshot` now returns `Result<Self, serde_json::Error>` (via A3 derive) |
| A2 — Tool-layer error mapping | **Fixed** | `src/tools/portfolio.rs` — `run_portfolio` uses `map_portfolio_error` instead of flat `invalid_argument`; `src/tools/analytics.rs` and `src/tools/valuation.rs` — 5 call sites updated to `crate::map_portfolio_error` |
| A3 — Hand-rolled JSON in `StoredForecast` | **Fixed** | `src/financial_model.rs` — `Serialize, Deserialize` added to `ProjectedLineItems`, `ProjectedModel`, `ProjectionAssumptions`; `src/lib.rs` — `snapshot()` replaced with `serde_json::to_value`, `from_snapshot()` replaced with `serde_json::from_value`; ~100 lines of hand-rolled code deleted; `as u8` truncation hazard eliminated |
| B1 — Stringly-typed provider routing | **Fixed** | `LearningState` now takes `Provider` enum (Copy, Hash, Display) instead of `&str`; `preferred_provider` returns `Option<Provider>`; `providers.rs` routing simplified — `format!("{:?}")` conversion and string comparisons deleted; tests updated to `Provider::Fmp` / `Provider::Eodhd` |
| B2 — Duplicated tool handler pattern | **Fixed** | `record_fetch_outcome` helper extracted to `CompaniesServer`; 7 financial-data tools simplified from ~12-line match blocks to 1-line calls (~49 lines removed) |
| B4 — endpoint_mapping silent fallthrough | **Fixed** | `endpoint_mapping` returns `Option<EndpointMapping>`; unknown tools error with `McpToolError::invalid_argument("unknown tool")` instead of silent 404 |
| B5 — Silent error swallowing | **Fixed** | `portfolio.rs` — `new()` and `with_dir()` use `expect()` instead of `let _ = ...`; `providers.rs` — 4× `resp.text().await.unwrap_or_default()` replaced with `.map_err(...)`; `research.rs` — 3× same pattern fixed |
| B6 — Schema DDL duplication | **Fixed** | `SCHEMA_DDL` const extracted; `with_dir()` now uses same DDL as `new()` (FK constraints restored in tests); ~70 lines of duplicated DDL removed |
| C2 — Magic-number thresholds | **Fixed** | `FLAKY_PROBABILITY_THRESHOLD`, `FLAKY_MIN_OBSERVATIONS`, `CHRONIC_STALENESS_DAYS` named constants replace inline `0.70`, `5`, `90` |
| C3 — Restating comment | **Fixed** | "P5 Essentialism — modular tool groups" comment replaced with clean "Combined tool router" |
| D1 — Broken README cross-reference | **Fixed** | `mcp-servers/hkask-mcp-companies/README.md` — link corrected to `README.md#companies-mcp-server` |
| D2 — Missing portfolio schema ERD | **Fixed** | `docs/architecture/hKask-architecture-master.md` — ERD (DIAG-IC-012) inlined with DIAGRAM_ALIGNMENT metadata |
| D3 — Stale "Inlined from" notes | **Fixed** | Updated to "Previously in `docs/diagrams/`; inlined per DOCUMENTATION_STANDARDS §6" |

**Remaining:** B3 (stringly-typed request fields → enums), C1 (ProjectionAssumptionError → thiserror enum). These are lower-priority guideline-level improvements documented for future work.

---

## Cross-crate review: pre-existing `Result<_, String>` violations

The fixed CI script (A1) revealed **84 `Result<_, String>` violations across 15 crates**.
These were silently passing the broken regex for the entire project history.

### Violation distribution

| Crate | Count | Layer | Priority |
|-------|------:|-------|----------|
| `hkask-cli` | 29 | User-facing CLI commands | Medium |
| `hkask-mcp-media` | 27 | MCP server (37 tools) | **High** |
| `hkask-repl` | 7 | REPL host + handlers | Medium |
| `hkask-services-core` | 6 | Self-heal subsystem | Medium |
| `hkask-templates` | 3 | Skill loader / Jinja2 | Low |
| `hkask-api` | 3 | Auth routes | Medium |
| `hkask-tui` | 2 | TUI kanban bridge | Low |
| `hkask-memory` | 2 | Consolidation service | Medium |
| `hkask-agents` | 2 | ActivePods management | Medium |
| 6 other crates | 1 each | Various | Low |

### Architectural analysis (hkask-mcp-media — 27 violations)

The media crate exhibits the **same architectural pattern** as the companies crate
before the fix: `Result<_, String>` throughout the helper layer, with the
tool-layer adapter flattening all String errors to a single `McpToolError` variant.

**Error flow:**
```text
GalleryStoreError / IoError / MutexError / serde_json::Error
  → .map_err(|e| format!("...: {}", e))    [structured → String, information lost]
  → match helper_result { Err(e) => McpToolError::internal(e) }  [String → single variant]
```

Notably, `GalleryStoreError` is already a structured error type imported from
`hkask-storage` — but the media crate converts it to String and discards the
structure. The fix is the same as the companies crate: define a `MediaError`
thiserror enum, implement `From<GalleryStoreError>`, `From<std::io::Error>`,
`From<serde_json::Error>`, and map variants to appropriate `McpToolError`
constructors.

**Highest-impact fix targets (by cybernetic variety deficit):**

1. `lib.rs` helper methods (8 violations) — `access_gallery`, `resolve_image_*`,
   `render_prompt`, `crop_face_region`, `load_meme_font`. These are the core
   access layer called by all 37 media tools. Fixing these 8 functions alone
   would improve error classification across the entire crate.

2. `video/ffmpeg.rs` (7 violations) — ffmpeg availability is mixed with
   processing errors in the same String type. `"ffmpeg not available"` should
   be `McpToolError::unavailable` while processing failures should be
   `McpToolError::internal`.

3. `gallery/vision.rs` (7 violations) — vision API errors (HTTP, JSON parse,
   missing fields) are all String. Cannot distinguish transient API failures
   from permanent misconfiguration.

### Architectural analysis (hkask-cli — 29 violations)

The CLI crate's violations are less cybernetically critical than the MCP
servers because CLI commands print errors directly to the user — there is no
programmatic consumer that needs to match on error variants. However, the
violations still prevent the `main` function from distinguishing recoverable
errors (retry) from permanent errors (exit) from user errors (show help).

The `pod.rs` module (10 violations) is the most affected — every pod
management function returns `Result<_, String>`, preventing the caller from
distinguishing "pod not found" from "daemon unreachable" from "permission
denied".

### Recommended fix priority

| Priority | Crate | Rationale |
|----------|-------|----------|
| 1 | `hkask-mcp-media` | MCP server — same cybernetic variety deficit as the companies crate; 37 tools affected; structured `GalleryStoreError` already exists but is discarded |
| 2 | `hkask-services-core` | Self-heal subsystem — `Result<_, String>` in a healing loop means the loop cannot distinguish failure modes to decide retry vs escalate |
| 3 | `hkask-cli` (pod.rs) | User-facing — `pod_id` parsing and daemon communication errors conflated; 10 functions in one module |
| 4 | `hkask-repl` | REPL host — trait definitions with `Result<_, String>` constrain all implementors |
| 5 | `hkask-memory` | Consolidation — async service with conflated error types |

### Fix approach (same as companies crate)

For each crate:
1. Define a crate-local `thiserror` error enum with variants matching the
disturbance classes (e.g., `GalleryNotInitialized`, `ImageNotFound`, `Io`,
`VisionApi`, `FfmpegUnavailable`, `FfmpegFailed`).
2. Implement `From` for each source error type (`GalleryStoreError`,
`std::io::Error`, `serde_json::Error`, `minijinja::Error`).
3. Convert function signatures one at a time (each is independently compilable).
4. Update tool-layer adapters to map error variants to appropriate
`McpToolError` constructors (same `map_*_error` pattern as `map_portfolio_error`).
5. Run `cargo test -p <crate>` after each function conversion.

The fix is mechanical once the error enum is defined. The companies crate
conversion (38 violations) took ~20 discrete steps; the media crate (27
violations) would take ~15 steps by the same methodology.

---

## A. Prohibition-level findings

### A1. CI string-error detector has a broken regex — 38 violations slip through

**Constraint force:** Prohibition (CI gate integrity).

`scripts/check-string-errors.sh` line 30 uses the regex
`->[[:space:]]*Result<.+,[[:space:]]*String[[:space:]]*>>`. The trailing `>>`
requires **two** closing angle brackets, so it only matches nested generics
like `Result<Vec<u8>, String>>`. The common case `Result<(), String>` (single
`>`) is never matched.

Empirical verification (2026-07-15):

```text
$ printf 'pub fn save_forecast(...) -> Result<(), String> {\n' \
  | grep -qE -- '->[[:space:]]*Result<.+,[[:space:]]*String[[:space:]]*>>' \
  && echo MATCHED || echo "NOT MATCHED"
NOT MATCHED
```

The script reports `OK: No Result<_, String> patterns found` while this single
crate contains **38** `Result<_, String>` signatures that the rule is supposed
to forbid.

| File | Count |
|------|------:|
| `src/portfolio.rs` | 25+ |
| `src/research.rs` | 6 |
| `src/lib.rs` | 2 |
| `src/tools/portfolio.rs` | 1 |

**Smallest fix:** Change the regex trailing `>>` to `>`. One character. This
will immediately surface all 38 violations so they can be triaged.

```
- '->[[:space:]]*Result<.+,[[:space:]]*String[[:space:]]*>>'
+ '->[[:space:]]*Result<.+,[[:space:]]*String[[:space:]]*>'
```

**Grill-me challenge:** Does the regex bug "earn its keep" as a finding, or is
it just a tooling nit? It is a Prohibition because the project explicitly lists
this script as a CI-enforced gate in `AGENTS.md`. A gate that silently passes
while the ruled-against pattern is pervasive is worse than no gate at all — it
creates false assurance. The one-character fix is the smallest possible action
with the largest blast radius.

### A2. `Result<_, String>` throughout `portfolio.rs` and `research.rs`

**Constraint force:** Prohibition (Magna Carta P5/P7; `AGENTS.md` Prohibition #1
adjacent; CI gate A1).

The portfolio module returns `Result<_, String>` from 25+ public and private
functions: `open`, `save_forecast`, `get_forecast`, `list_forecasts`,
`validate_forecast_revision`, `record_forecast_outcome`, `create`, `delete`,
`list`, `check_exists`, `add_transaction`, `commit_to_ledger`, `append_note`,
`validate`, `export_json`, `export_csv`, `compare`, `add_note`, `delete_note`,
`attach_file`, `delete_file`, `check_request_size`, and `get_transactions`.

`research.rs` returns `Result<Vec<ResearchClaim>, String>` from 6 parser
functions: `parse_exa_results`, `parse_tavily_results`, `parse_brave_results`,
and three caller functions.

`lib.rs::StoredForecast::from_snapshot` returns `Result<Self, String>`.

**Downstream consequence (pragmatic-cybernetics):** The tool-layer adapter
`run_portfolio` (`tools/portfolio.rs:14`) maps **every** `String` error to
`McpToolError::invalid_argument`. A database-open failure, a serialisation
failure, and a user-supplied bad portfolio name all surface to the MCP client as
"invalid argument". The error channel has lost all variety — the regulator
(caller) cannot distinguish disturbance classes (Ashby's Law violated). The
client cannot tell "your input was bad" from "the database is unreachable".

**Smallest decomposition:** Introduce a `thiserror` enum per module, then
convert functions one at a time. Each function is an independent migration unit.

```text
Step 1:  Add PortfolioError enum (thiserror) to portfolio.rs        [~30 lines]
Step 2:  Convert PortfolioManager::open                              [~3 lines]
Step 3:  Convert check_request_size                                   [~3 lines]
Step 4:  Convert check_exists                                         [~5 lines]
Step 5:  Convert create / delete / list                               [~15 lines]
Step 6:  Convert add_transaction / commit_to_ledger                   [~20 lines]
Step 7:  Convert append_note / get_transactions / validate             [~30 lines]
Step 8:  Convert export_json / export_csv / compare                    [~20 lines]
Step 9:  Convert add_note / delete_note                               [~15 lines]
Step 10: Convert attach_file / delete_file                            [~20 lines]
Step 11: Convert save_forecast / get_forecast / list_forecasts /
         validate_forecast_revision / record_forecast_outcome          [~40 lines]
Step 12: Update run_portfolio to map PortfolioError variants          [~10 lines]
Step 13: Add ResearchError enum to research.rs; convert 6 functions   [~40 lines]
Step 14: Convert StoredForecast::from_snapshot to use SnapshotError   [~15 lines]
```

Each step is independently compilable and testable. No step exceeds ~40 lines.

**Essentialist challenge (G1 — Exist):** Does the `PortfolioError` enum earn
its keep? Delete-test: inline `PortfolioError` into `McpToolError` directly. If
`run_portfolio` just does `McpToolError::internal(err.to_string())`, the enum
adds no information beyond what `anyhow` would. The enum earns its keep **only
if** `run_portfolio` maps different variants to different `McpToolError`
constructors (`invalid_argument` for `NotFound`/`BadName`, `unavailable` for
`DatabaseError`, `internal` for `SerializeError`). If the conversion is a flat
`to_string()`, delete the enum and use `anyhow` — that is fewer lines with equal
information. **Verdict: the enum earns its keep only with variant-aware mapping.**

### A3. `StoredForecast` hand-rolls JSON instead of deriving `Serialize`/`Deserialize`

**Constraint force:** Prohibition (P5 deep-module; `coding-guidelines`
Simplicity First — "a 200-line solution that could be 50 lines is a critical
violation").

`lib.rs:107-211` — `StoredForecast::snapshot()` (34 lines) and
`from_snapshot()` (69 lines) hand-construct `serde_json::Value` field by field,
then hand-parse it back. This is ~103 lines that re-implement what
`#[derive(Serialize, Deserialize)]` provides for free on `ProjectedModel` and
`ProjectionAssumptions`.

The `as u8` casts at `lib.rs:204-205` (`integer(assumptions, "total_years")? as
u8`) read a `u64` then truncate — a corrupted snapshot with `total_years: 300`
silently becomes `44` with no error.

**Smallest fix:**

```text
Step 1: Add #[derive(Serialize, Deserialize)] to ProjectedLineItems,
        ProjectedModel, and ProjectionAssumptions in financial_model.rs
        [~6 attribute lines]
Step 2: Replace StoredForecast::snapshot with serde_json::to_value
        [~5 lines]
Step 3: Replace StoredForecast::from_snapshot with serde_json::from_value
        into a typed struct; remove the as-u8 casts                  [~10 lines]
Step 4: Delete the number/integer closure helpers                    [delete ~20 lines]
```

Net: ~103 lines → ~15 lines. The `as u8` truncation hazard disappears because
serde deserialises directly into `u8` and errors on overflow.

---

## B. Guardrail-level findings

### B1. Stringly-typed provider routing — enum↔String round-trip at every boundary

**Constraint force:** Guardrail (idiomatic-rust; P3 no hidden parameters).

`providers::Provider` is an enum (`Fmp`, `Eodhd`), but `LearningState` uses
`String` keys for provider throughout. The routing layer converts
enum→String→enum at every boundary:

- `providers.rs:120` — `format!("{:?}", primary_provider(symbol)).to_lowercase()`
  converts enum to lowercased Debug string to pass to `LearningState`.
- `providers.rs:122-124` — `Some(ref p) if p == "FMP"` string-compares back to
  enum.
- `lib.rs:374-376` — `preferred_provider` returns `Some("EODHD".to_string())` /
  `Some("FMP".to_string())` — hardcoded strings, not enum variants.
- `LearningState::record`, `success_probability`, `observation_count`,
  `check_staleness`, `is_flaky`, `is_chronically_stale` all take `provider: &str`.

**Idiomatic-rust violation (Hoare — make invalid states impossible):** A caller
can pass `record("AAPL", "Fmp", Some(5))` or `record("AAPL", "fmp", Some(5))` or
`record("AAPL", "FMP", Some(5))` and get three different hash-map entries. The
type system permits typos and case variants that should be impossible.

**Smallest fix:**

```text
Step 1: Change LearningState API to take Provider (Copy enum) instead of &str
        [~15 signature changes]
Step 2: Update preferred_provider to return Option<Provider>            [~10 lines]
Step 3: Update providers.rs routing to pass Provider directly; delete
        the format!("{:?}") and string-compare code                     [~15 lines]
Step 4: Update tests to use Provider::Fmp / Provider::Eodhd             [~20 lines]
```

### B2. Massive duplication in financial-data tool handlers

**Constraint force:** Guardrail (coding-guidelines — no single-use repetition;
deep-module — extract the pattern).

`tools/financial_data.rs` repeats the same 8-line block in all 7 fetch-based
tools (`company_profile`, `stock_quote`, `income_statement`, `balance_sheet`,
`cash_flow_statement`, `key_metrics`, `historical_price`):

```rust
let result = self.fetch("tool_name", &symbol, &extra).await;
match &result {
    Ok(v) => self.record_experience("tool_name", &format!("symbol={}", symbol), "success", v.clone()),
    Err(e) => self.record_experience("tool_name", &format!("symbol={}", symbol), "error", serde_json::json!({"error": e.to_json_string()})),
}
result
```

This is ~56 lines of pure copy-paste across 7 tools. The same pattern appears
in `analysis.rs`, `valuation.rs`, and `portfolio.rs` tool handlers.

**Smallest fix:** Extract a helper:

```rust
fn record_fetch_outcome(&self, tool: &str, symbol: &str, result: &Result<Value, McpToolError>) {
    match result {
        Ok(v) => self.record_experience(tool, &format!("symbol={}", symbol), "success", v.clone()),
        Err(e) => self.record_experience(tool, &format!("symbol={}", symbol), "error",
            serde_json::json!({"error": e.to_json_string()})),
    }
}
```

Each tool body collapses from ~20 lines to ~5. ~56 lines → ~7 calls.

**Essentialist challenge (G1):** Does `record_fetch_outcome` earn its keep, or
is it a single-use abstraction? It is called 7+ times with identical structure.
Delete-test: inline it back into each tool — complexity reappears as 7 copies of
the same match block. **Verdict: earns its keep.** It has one job (telemetry
side-effect) and is multiply-invoked.

### B3. Stringly-typed request fields that should be enums

**Constraint force:** Guardrail (idiomatic-rust — make invalid states impossible).

| Struct | Field | Comment | Should be |
|--------|-------|---------|-----------|
| `LedgerImportRequest` | `format: String` | `"csv" or "json"` | `enum ImportFormat { Csv, Json }` |
| `LedgerExportRequest` | `format: String` | `"csv" or "json"` | same enum |
| `ForecastRecordRequest` | `horizon: String` | `"3mo".."3yr"` | `enum Horizon { ThreeMo, SixMo, OneYr, TwoYr, ThreeYr }` |
| `EpValuationRequest` | `moat_override: Option<String>` | `"wide"/"narrow"/"none"/"default"` | `enum MoatClass { Wide, Narrow, None }` |
| `EpValuationRequest` | `moat_result: Option<String>` | same | same enum |
| `Transaction` | `tx_type: String` | `CHECK(type IN ('buy','sell',...))` | `enum TxType { Buy, Sell, Dividend, Deposit, Withdrawal }` |
| `ScreenerRequest` | `criteria_overrides: serde_json::Value` | untyped blob | typed struct or `Option<ScreenerCriteria>` |

**Smallest fix:** Each field is an independent unit. Convert one enum at a time.
Each conversion touches one struct + its call sites (~10-15 lines).

**Grill-me challenge (Edge Cases):** What happens if a client sends
`format: "CSV"` (uppercase)? Current code does a case-sensitive string compare
somewhere downstream and silently fails or misroutes. An enum with
`#[serde(rename_all = "lowercase")]` rejects "CSV" at deserialisation with a
clear error. The enum is not cosmetic — it is a correctness fix.

### B4. `endpoint_mapping` silent fallthrough for unknown tools

**Constraint force:** Guardrail (idiomatic-rust — no silent failure).

`providers.rs:76-81` — `endpoint_mapping` returns
`EndpointMapping { fmp_path: "", eodhd_path: "", ... }` for any unrecognised
tool name. Both providers are then called with an empty path segment, producing
a 404 from the upstream API that surfaces as a confusing "FMP request failed"
error instead of "unknown tool: foo".

**Smallest fix:** Return `Option<EndpointMapping>` or `Result`, and error early
in `companies_get` when the mapping is absent. ~10 lines.

### B5. Silent error swallowing in five locations

**Constraint force:** Guardrail (P12; cybernetics — broken feedback loop).

| Location | Pattern | Risk |
|----------|---------|------|
| `providers.rs:241,271` | `resp.text().await.unwrap_or_default()` | Network error silently becomes empty string; parse then fails with "failed to parse" instead of the real network error |
| `portfolio.rs:127` | `let _ = std::fs::create_dir_all(&path);` | Directory creation failure ignored; `Connection::open` then fails with confusing path error |
| `portfolio.rs:130-131` | `if let Ok(conn) = Connection::open(&path) { let _ = conn.execute_batch(...); }` | Both open AND schema creation failure silently ignored; subsequent queries fail with "no such table" |
| `lib.rs:527` | `tokio::spawn` in `record_experience` | Fire-and-forget; daemon errors only logged, never surfaced |
| `lib.rs:451` | `learning.lock().unwrap_or_else(\|e\| e.into_inner()).clone()` | Poison recovery is tested (poison_tests.rs), but the clone means subsequent updates to the cloned snapshot are discarded |

**Smallest fix:** Each is an independent ~3-5 line change. The
`portfolio.rs:130-131` case is the most impactful — schema creation failure
should be a hard startup error, not a silent skip.

### B6. Schema DDL duplicated between `new()` and `with_dir()`

**Constraint force:** Guardrail (coding-guidelines — DRY; testing fidelity).

`portfolio.rs:132-202` (`new`) and `portfolio.rs:222-292` (`with_dir`) contain
**identical** schema DDL. The test version (`with_dir`) omits `REFERENCES
portfolios(name) ON DELETE CASCADE` FK constraints on `transactions`,
`price_cache`, `security_links`, `notes`, and `files`. Tests therefore run
against a schema without FK enforcement — cascade-delete bugs cannot be caught.

**Smallest fix:** Extract the DDL to a `const SCHEMA_DDL: &str` and use it in
both paths. The test version then inherits the FK constraints. ~70 lines → 1
const + 2 one-line calls.

---

## C. Guideline-level findings

### C1. `ProjectionAssumptionError` is a String newtype, not a thiserror enum

**Constraint force:** Guideline (idiomatic-rust; AGENTS.md convention:
`thiserror` enums for library errors).

`financial_model.rs:409` — `pub struct ProjectionAssumptionError(String)`. It
implements `Display + Error` (borderline CI-pass) but exposes no structured
variants. Callers cannot match `OutOfRange { field, value, range }` vs
`NotFinite { field }` vs `HorizonOverflow`.

**Smallest fix:** Convert to a `thiserror` enum with 3-4 variants. ~30 lines.

**Essentialist challenge (G3 — Contract):** Is this a single-use abstraction?
`ProjectionAssumptionError` is returned by `with_overrides`, `validate`,
`validate_sensitivity_range`, and `McRange::validate` — 4 call sites, all in
the same module. It earns its keep as a module-local error type. But the
newtype-wrapping-String is a pass-through: it adds the `Error` impl but no
structured information beyond what `String` already carries. **Verdict: convert
to enum or delete and use `anyhow` in this application-layer module.**

### C2. Magic-number thresholds in `LearningState`

**Constraint force:** Guideline (idiomatic-rust; cybernetics — gain/delay
parameters should be named).

- `lib.rs:353` — `prob < 0.70 && observation_count >= 5` (flaky threshold)
- `lib.rs:362` — `days > 90` (chronic staleness threshold)
- `providers.rs:203` — `match tool { "key_metrics" => 4, _ => 0 }` (approximated field count)

**Smallest fix:** Named constants. ~5 lines.

### C3. `combined_router` comment claims "P5 Essentialism" but adds no depth

**Constraint force:** Guideline (essentialist — does the comment earn its keep?).

`lib.rs:547` — `// ── Combined tool router (P5 Essentialism — modular tool groups) ──`.
The router simply sums 7 sub-routers. The comment claims this is "Essentialism"
but the tool split into 7 files is a structural choice, not an essentialist
reduction. The comment restates the code without adding intent. **Delete or
replace with intent:** "Seven domain routers composed via `ToolRouter::Add`."

---

## D. Documentation findings (diataxis-diagram assessment)

### D1. Broken cross-reference in crate README (Prohibition-adjacent)

`mcp-servers/hkask-mcp-companies/README.md:87` links to
`../../docs/reference/mcp-servers/hkask-mcp-companies.md` — **this file does not
exist**. The actual reference content is inline in
`docs/reference/mcp-servers/README.md` under the "Companies MCP Server" heading.

**Smallest fix:** Update the link to
`../../docs/reference/mcp-servers/README.md#companies-mcp-server`. One line.

### D2. Missing required ERD for the portfolio SQLite schema

**Constraint force:** Guardrail (DOCUMENTATION_STANDARDS §4.1 — "Data model →
erDiagram" is required).

The portfolio module manages a 7-table SQLite schema (`portfolios`,
`transactions`, `price_cache`, `security_links`, `notes`, `files`,
`forecasts`). No ERD exists anywhere in the documentation corpus. The existing
"Storage Schema ERD" in the architecture master doc covers the main hKask
storage, not the companies portfolio schema.

**Smallest fix:** Create an `erDiagram` Mermaid block and inline it into
`docs/architecture/hKask-architecture-master.md` adjacent to the existing
Companies MCP sequence diagrams, following the established convention
(DIAGRAM_ALIGNMENT metadata, plain-English description, cross-link).

### D3. Stale "Inlined from `docs/diagrams/`" notes

The architecture master doc says "Inlined from
`docs/diagrams/sequence-companies-provider-routing.md`" — but the `docs/diagrams/`
directory no longer exists. The "Inlined from" note is now historically accurate
(the files were once there) but practically misleading (a reader may go looking
for them). Consider replacing with "Previously in `docs/diagrams/`; inlined per
DOCUMENTATION_STANDARDS §6."

### D4. Tool-count presentation mismatch (Evidence)

Crate README groups "Economic-profit and expectations analysis | 2" as one row.
`docs/reference/mcp-servers/README.md` splits them as two rows (1 + 1). The
total is the same (41); the presentation differs. Not a defect, but a
consistency opportunity.

### D5. Existing diagrams verified against code — accuracy spot-check

The two existing Companies diagrams (DIAG-IC-010, DIAG-IC-011) were spot-checked
against the current source:

- `verified_against: providers.rs:84-247` — `companies_get` is at L106-197.
  Range is approximately correct. **Accurate.**
- `verified_against: lib.rs:340-361` — `is_flaky`/`is_chronically_stale` are at
  L351-364. **Accurate.**
- The provider-routing diagram omits the `.US` suffix transformation applied
  during FMP→EODHD fallback on plain symbols (`providers.rs:168-172`). Minor
  fidelity gap — the diagram says "retry request" but does not note the symbol
  rewrites. **Minor staleness.**

---

## Essentialist challenge — does each recommendation survive G1 (Exist)?

The essentialist's deletion test asks: delete the recommended artifact — does
complexity reappear in callers, or does it vanish?

| Recommendation | Delete-test result | Verdict |
|----------------|--------------------|---------|
| **A1: Fix CI regex** | Delete the fix → 38 silent violations remain undetected. Complexity (silent bugs) reappears. | **Keep — earns its keep.** |
| **A2: PortfolioError enum** | Delete the enum, use `anyhow` → callers lose variant discrimination. But if `run_portfolio` flattens all variants to `invalid_argument` anyway, the enum adds nothing. **Conditional: keep only with variant-aware mapping.** | **Conditional** |
| **A3: Derive Serialize/Deserialize** | Delete the derive, keep hand-rolled JSON → 103 lines of brittle code and `as u8` truncation persist. Delete the hand-rolled code, use derive → complexity vanishes. | **Keep — the deletion removes complexity.** |
| **B1: Provider enum in LearningState** | Delete the enum change, keep String keys → typos and case-variants create phantom hash entries. Complexity (debugging invisible bugs) reappears. | **Keep.** |
| **B2: record_fetch_outcome helper** | Delete the helper, inline the match → 7 copies of the same block reappear. | **Keep.** |
| **B3: Enum request fields** | Delete the enums, keep String fields → invalid values accepted at the boundary, fail deep inside with confusing errors. | **Keep.** |
| **B6: Extract SCHEMA_DDL const** | Delete the const, duplicate DDL → test schema diverges from prod schema (already happened: test omits FKs). | **Keep.** |
| **C1: ProjectionAssumptionError as enum** | Delete the enum, use `anyhow` → callers in the same module lose nothing (they all `.to_string()` it). The newtype is a pass-through. | **Delete-and-use-anyhow OR convert — do not keep as-is.** |
| **C3: combined_router comment** | Delete the comment → no information lost (the code is self-explanatory). | **Delete.** |

**Essentialism score:** 9 recommendations evaluated; 1 (C3) is a pure deletion;
1 (C1) is a "convert or delete" (the current form fails G3). The remaining 7
survive the deletion test because deleting them lets real complexity reappear.

---

## Grill-me challenge — escalating interrogation of recommendations

**Recall (level 1):** How many `Result<_, String>` signatures are in this crate?
→ 38, confirmed by grep.

**Mechanism (level 2):** Why does the CI script not catch them? → The regex
requires `>>` (double close bracket) and the common case has a single `>`. The
regex was likely written for a nested-generic case and never tested against the
simple case.

**Rationale (level 3):** Why does `portfolio.rs` use `String` errors at all,
given the project convention? → The module was likely written before the CI
gate existed, or the gate was added later and never retro-applied. The
`run_portfolio` adapter was written to bridge `String` → `McpToolError`, which
made the `String` errors "work" well enough that the smell was never felt as
pain. This is cybernetic: the feedback loop (CI gate) has zero gain (the regex
is broken), so the error signal never reaches the developer.

**Edge Cases (level 4):** What happens when `from_snapshot` reads a snapshot
where `total_years` is 300? → `integer()` returns `300u64`, then `as u8`
truncates to `44` (300 - 256). No error. The model then projects 44 years with
`stage1_years` also possibly truncated. The valuation is silently wrong. **This
is a data-integrity bug, not just a style issue.** Deriving `Deserialize`
directly into `u8` would reject 300 with a deserialisation error.

**Synthesis (level 5):** If you fix only A1 (the regex) and nothing else, what
happens? → CI immediately fails with 38 errors. The crate is now un-mergeable
until all 38 are fixed. This is the "broken window" effect in reverse: fixing
the gate forces the cleanup. If you fix A2-A3 first and then fix A1, the
transition is smooth. **Recommended order: A2/A3 first (fix the violations),
then A1 (turn on the gate), then B-series.**

---

## Cybernetic assessment (pragmatic-cybernetics)

The provider-learning loop (`result_feedback` → `LearningState` →
`preferred_provider` → routing) is a feedback loop with the following health
assessment:

| Property | Assessment | Evidence |
|----------|------------|---------|
| **Closure** | Closed | `record` updates state; `preferred_provider` reads state; routing uses it; result quality feeds back via `record`. |
| **Fidelity** | Degraded | The `String` provider key loses enum fidelity (B1). Case-variant typos create phantom entries that dilute the Beta posterior. |
| **Gain** | Low | The flaky threshold (0.70) and observation floor (5) are magic numbers (C2), not tuned. The loop is conservative — it takes 15+ accurate ratings to recover from 5 failures (confirmed by `learning_loop_recovery_after_accurate_ratings` test). |
| **Delay** | High | Feedback is only collected when a user explicitly calls `result_feedback`. There is no passive quality signal. The loop has no automatic sensing — it relies on voluntary reporting. |
| **Polarity** | Correct | Negative feedback (low scores) correctly reduces provider preference. |

**Variety check (Ashby):** The regulator (`run_portfolio`) can produce exactly
one response class: `McpToolError::invalid_argument`. The system produces at
least four disturbance classes (bad input, not-found, database error,
serialisation error). Regulator variety (1) < system variety (4). **Deficit:
3.** Amplification: introduce `PortfolioError` variants mapped to distinct
`McpToolError` constructors (A2).

---

## Recommended action sequence (smallest pragmatic steps, ordered)

The ordering follows "fix the violations, then turn on the gate, then improve":

```text
Phase 1 — Prohibition fixes (make CI green after gate fix)
  1.  A3-step1: Derive Serialize/Deserialize on financial_model types
  2.  A3-step2-4: Replace StoredForecast hand-rolled JSON; delete closures
  3.  A2-step1: Add PortfolioError thiserror enum to portfolio.rs
  4.  A2-steps2-12: Convert 25+ portfolio.rs functions one at a time
  5.  A2-step13: Add ResearchError enum; convert 6 research.rs functions
  6.  A2-step14: Convert StoredForecast::from_snapshot

Phase 2 — Turn on the gate
  7.  A1: Fix the CI regex (one character: >> → >)

Phase 3 — Guardrail fixes (independent, any order)
  8.  B1: LearningState takes Provider enum (4 steps)
  9.  B2: Extract record_fetch_outcome helper
  10. B3: Convert stringly-typed request fields to enums (7 independent units)
  11. B4: endpoint_mapping returns Option/Result
  12. B5: Fix 5 silent-error-swallowing sites (5 independent units)
  13. B6: Extract SCHEMA_DDL const; remove test/prod DDL divergence

Phase 4 — Guideline fixes
  14. C1: Convert ProjectionAssumptionError to thiserror enum (or use anyhow)
  15. C2: Named constants for magic thresholds
  16. C3: Delete the "P5 Essentialism" comment

Phase 5 — Documentation
  17. D1: Fix broken README cross-reference (one line)
  18. D2: Create portfolio schema ERD (inline in architecture master doc)
  19. D3: Update stale "Inlined from" notes
  20. D5: Note the .US suffix gap in the provider-routing diagram
```

Each numbered item is independently compilable and testable. No item depends on
another within the same phase (except the ordering within Phase 1, where later
steps build on the error enum from step 3). The total is ~20 discrete steps, each
small enough to review in isolation.

---

## Validation

- **Findings verified against source:** all line references confirmed by
  reading the files on 2026-07-15.
- **CI regex bug reproduced:** the `printf | grep` test in §A1 was executed and
  confirms the regex fails to match `Result<(), String>`.
- **Violation count (38):** produced by
  `grep -rnE 'Result<[^,]*,\s*String\s*>' mcp-servers/hkask-mcp-companies/src --include='*.rs'`.
- **Tests not run:** `cargo test -p hkask-mcp-companies` was not executed in
  this review session. The review is static-analysis only. Run the test suite
  after applying any code change.

## References

- [DOCUMENTATION_STANDARDS §4 — Mermaid diagrams](../specifications/DOCUMENTATION_STANDARDS.md#4-mermaid-diagrams)
- [CI invariants gate — `scripts/check-string-errors.sh`](../../scripts/check-string-errors.sh)
- [Companies MCP Server Reference](../reference/mcp-servers/README.md#companies-mcp-server)
- [Companies provider-routing diagram](../architecture/core/hKask-architecture-master.md#companies-mcp-provider-routing)
- [Companies forecast-feedback diagram](../architecture/core/hKask-architecture-master.md#companies-mcp-forecast-feedback)