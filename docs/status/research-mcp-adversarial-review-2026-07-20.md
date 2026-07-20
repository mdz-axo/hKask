---
title: "hkask-mcp-research — Adversarial Code Review (Follow-Up)"
audience: [developers, maintainers, security-reviewers]
last_updated: 2026-07-20
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, composition, trust, lifecycle, curation]
last-verified-against: "mcp-servers/hkask-mcp-research/src/; crates/hkask-services-research/src/"
---

# hkask-mcp-research — Adversarial Code Review (Follow-Up)

A second adversarial pass over the `hkask-mcp-research` MCP server crate and
its service crate `hkask-services-research`, conducted after the 2026-07-17
review ([`research-mcp-adversarial-review-2026-07-17.md`](research-mcp-adversarial-review-2026-07-17.md))
closed all 15 prior findings. The goal of this pass is to catch issues the
first review missed by taking a deliberately skeptical posture and probing
deeper into security semantics, transaction boundaries, and dead-state
artifacts that a conventional review would overlook.

## Methodology

Same multi-skill stack as the 2026-07-17 review: `improve-codebase-architecture`,
`bug-hunt`, `diagnose`, `coding-guidelines`, `idiomatic-rust`, `pragmatic-laziness`,
`pragmatic-semantics`, `pragmatic-cybernetics`, with `essentialist` and
`grill-me` as adversarial challengers. Each finding is decomposed into the
smallest independently-actionable step.

## Fix status (2026-07-20)

All 11 new findings have been fixed in the codebase.

| Finding | Status | Files changed |
|---------|--------|---------------|
| N1 — `CapabilityContext` dead (all calls pass `None`) | **Fixed** | `types/mod.rs` (struct deleted), `providers/mod.rs` (trait + impl signatures, `check_capability` deleted), `lib.rs` (both crates — `None` args removed) |
| N2 — `edit_tags` label ops relabel entire feed | **Fixed** | `db.rs` — `add_label`/`remove_label` SQL removed; fields retained on `EditTagRequest` for backward-compatible deserialization but now ignored |
| N3 — No SQL transactions in multi-statement ops | **Fixed** | `db.rs` — `mark_stream_read`, `edit_tags`, `import_opml` wrapped in `BEGIN`/`COMMIT`/`ROLLBACK`; `lib.rs` — `rss_subscribe`, `rss_fetch` wrapped |
| N4 — `SearchStrategy::News` silent 0 results | **Fixed** | `providers/mod.rs` — `WebSearchPort::search` now returns `ProviderUnavailable` when a capability-filtered strategy matches zero providers |
| N5 — `TavilyProvider::health()` and `SerapiProvider::health()` stubs | **Fixed** | `providers/tavily.rs`, `providers/serapi.rs` — minimal liveness checks implemented (429 treated as healthy) |
| N6 — `import_opml` swallows DB errors via `unwrap_or(0)` | **Fixed** | `db.rs` — `unwrap_or(false)` → `?`, `unwrap_or(0)` → `?`, `Err(_) => errors += 1` → `Err(ConstraintViolation) => skipped`, else `return Err(e)` |
| N7 — `strip_html` `<li>` concatenation + no comment handling | **Fixed** | `strip_html.rs` — `<li>` now inserts newline before `- `; HTML comments `<!-- ... -->` stripped via 3-char sliding window; 3 new tests added |
| N8 — `ResponseCache` O(n) eviction | **Documented** | `cache.rs` — trade-off comment added; acceptable because `max_entries` capped at 200 |
| N9 — arXiv `pdf_url` extraction breaks on multi-line `<link>` | **Fixed** | `providers/arxiv.rs` — `entry.lines().find` → `entry.find("title=\"pdf\"")` with bidirectional `href` search |
| N10 — `discover_feeds` fetches URL without SSRF validation | **Fixed** | `feed.rs` — `validate_provider_url` call added before `client.get(url)` |
| N11 — Stored SSRF in `rss_fetch` + `import_opml` | **Fixed** | `lib.rs` — `validate_tool_url(&feed_url)` added in `rss_fetch`; `db.rs` — `validate_provider_url` per-URL in `import_opml` |

## Findings

### N1 — `CapabilityContext` is dead state (Prohibition #1)

**Constraint force:** Prohibition (no pass-through abstractions; dead code)
**Severity:** High (OCAP at port is bypassed; security control is theatrical)
**Location:** `crates/hkask-services-research/src/types/mod.rs:406-418`, `providers/mod.rs:71-100,484-494,500-659`, `mcp-servers/hkask-mcp-research/src/lib.rs:238,305,405,474`

The `WebSearchPort` trait takes `ctx: Option<&CapabilityContext>` on every
method, and `ProviderPool` calls `check_capability(ctx, "tool_name")?` at the
top of each. But **every tool handler in the MCP server passes `None`**:

```rust
self.pool.search(&search_query, strat, None)   // lib.rs:238
self.pool.find_similar(&url, num, None)        // lib.rs:305
self.pool.extract(&url, &opts, None)           // lib.rs:405
self.pool.browse(&url, &instr, timeout, None)  // lib.rs:474
```

`check_capability` with `ctx = None` is `Ok(())` — the capability gate is
never exercised. OCAP is enforced at the dispatcher membrane (`GovernedTool`
in `crates/hkask-mcp/src/dispatch.rs`), not at the port. The port-level
`CapabilityContext` is speculative API surface that was never wired.

**Essentialist (G1 — Exist):** Delete `CapabilityContext`, `check_capability`,
and the `ctx` parameter from `WebSearchPort` and its impl. Complexity
vanishes; no behavior changes. **FAIL → delete.**

**Grill-me (Rationale):** Why does `CapabilityContext` exist if it's never
constructed? It appears to be a speculative OCAP-at-port design that was
superseded by the dispatcher-membrane model before it was ever wired. The
2026-07-17 review's fleet audit ([`mcp-fleet-test-seam-audit-2026-07-17.md`](mcp-fleet-test-seam-audit-2026-07-17.md))
confirmed OCAP is a membrane concern, not a server concern — so the
port-level check is architecturally redundant, not just unwired.

**Fix:** Delete `CapabilityContext` struct, `check_capability` function, and
the `ctx: Option<&CapabilityContext>` parameter from `WebSearchPort` and its
impl. Remove the `None` arguments from all 4 tool calls in the MCP server.

---

### N2 — `edit_tags` label operations relabel the entire feed (Prohibition #4)

**Constraint force:** Prohibition (silent wrong behavior; pass-through abstraction that does the wrong thing)
**Severity:** High (data corruption: labeling one entry relabels every entry in that feed)
**Location:** `crates/hkask-services-research/src/db.rs:436-449`

```rust
if let Some(ref label) = req.add_label {
    conn.execute(
        "UPDATE subscriptions SET label = ?1 WHERE feed_id = (SELECT feed_id FROM entries WHERE id = ?2)",
        rusqlite::params![label, id],
    )?;
}
```

The `EditTagRequest` has `add_label`/`remove_label` fields documented as
"Edit tags on entries: ... add/remove labels". But the SQL updates the
**subscription's** label based on the entry's feed_id. This means:

- Calling `rss_edit_tag` with `add_label: "tech"` on entry 42 updates the
  subscription for entry 42's feed, relabeling **every entry in that feed**
  as "tech" — not just entry 42.
- The `remove_label` path has the same bug.

The schema has no per-entry label table — labels live on `subscriptions`.
The label operations are semantically broken at the data model level.

**Pragmatic-semantics (IS vs OUGHT):**
- IS: `add_label` on an entry updates the subscription's label.
- OUGHT: `add_label` on an entry should label that entry only.

**Fix:** Remove the `add_label`/`remove_label` SQL from `edit_tags`. The
fields remain on `EditTagRequest` for backward-compatible deserialization
but are now ignored. Per-entry labels require a schema change (a
`entry_labels` table keyed by `entry_id`) and are out of scope for this fix.

---

### N3 — No SQL transactions in multi-statement operations (Guardrail)

**Constraint force:** Guardrail (data integrity; ACID violation)
**Severity:** High (partial failures leave DB inconsistent)
**Location:** `crates/hkask-services-research/src/db.rs` (`mark_stream_read`, `edit_tags`, `import_opml`), `mcp-servers/hkask-mcp-research/src/lib.rs` (`rss_subscribe`, `rss_fetch`)

Five operations execute multiple SQL statements without a transaction:

1. `rss_subscribe`: `upsert_feed` + `insert_entries` + `update_feed_cache_headers` + `INSERT subscription`
2. `rss_fetch`: `upsert_feed` + `insert_entries` + `update_feed_cache_headers`
3. `mark_stream_read`: `SELECT entry_ids` + N× `INSERT INTO entry_states`
4. `edit_tags`: per-entry `SELECT COUNT` + up to 4× `INSERT INTO entry_states`
5. `import_opml`: per-URL `SELECT COUNT` + `INSERT feed` + `SELECT id` + `INSERT subscription`

If any statement fails mid-loop, prior statements are committed but later
ones are not — leaving the DB in an inconsistent state. For example, if
`insert_entries` fails halfway through a feed, `upsert_feed` has already
updated `last_fetched_at`, so a retry won't re-fetch the missing entries
(the feed appears "fresh" but is missing entries).

**Pragmatic-cybernetics (loop closure):** The feedback loop is broken: a
partial failure leaves the system in a state that looks "done" to the next
fetch (ETag/Last-Modified updated) but is actually incomplete. The loop
cannot detect the inconsistency because the sensor (ETag) was updated
before the action (entry insertion) completed.

**Fix:** Wrap each multi-statement operation in `BEGIN`/`COMMIT`/`ROLLBACK`.
On error, rollback and propagate the error.

---

### N4 — `SearchStrategy::News` silently returns 0 results (Guardrail)

**Constraint force:** Guardrail (silent failure; user surprise)
**Severity:** Medium (user asks for news, gets empty results with no error)
**Location:** `crates/hkask-services-research/src/providers/mod.rs:212-392`

`SearchStrategy::News` filters providers to `SearchCapability::News`. Only
`BraveProvider` and `SerapiProvider` declare `News` capability. If neither
API key is set, `search_compound` runs with an empty `filtered` list,
returns an empty `CompoundSearchResult`, and the user sees 0 results with
no explanation.

**Grill-me (Edge Cases):** What happens when a user calls `web_search` with
`strategy: "news"` and no News-capable provider has an API key? The search
"succeeds" with 0 results. The `providers_queried` field is empty,
`providers_succeeded` is empty, `providers_failed` is empty. There's no
signal that the strategy is unsupported — it just looks like the query
matched nothing.

**Fix:** In `WebSearchPort::search`, before dispatching a compound search,
check that the strategy's provider filter matches at least one configured
provider. If not, return `WebError::ProviderUnavailable` with a message
naming the required capabilities and the missing API keys.

---

### N5 — `TavilyProvider::health()` and `SerapiProvider::health()` are stubs (Guardrail)

**Constraint force:** Guardrail (cybernetic fidelity; the 2026-07-17 review caught Exa but missed these two)
**Severity:** Medium (health check has zero fidelity for Tavily and SerpAPI)
**Location:** `crates/hkask-services-research/src/providers/tavily.rs:101-103`, `providers/serapi.rs:288-290`

```rust
async fn health(&self) -> Result<(), WebError> {
    Ok(())
}
```

Both providers' `health()` always return `Ok(())`. The `web_ping` tool
reports them as healthy even if the API key is invalid or the service is
down. The 2026-07-17 review (G8) caught the same pattern in `ExaProvider`
and fixed it, but missed these two.

**Pragmatic-cybernetics (fidelity):** A health check that always returns Ok
is not a sensor — it's a constant. The feedback loop cannot detect Tavily
or SerpAPI outages because the sensor produces no signal.

**Fix:** Implement minimal liveness checks following the `BraveProvider::health()`
pattern: send a minimal search request, treat 2xx and 429 as healthy,
treat 401/403 as unhealthy (invalid key), treat 5xx as unhealthy.

---

### N6 — `import_opml` swallows DB errors via `unwrap_or(0)` (Guardrail)

**Constraint force:** Guardrail (error swallowing; silent data loss)
**Severity:** Medium (DB errors look like "no feed found")
**Location:** `crates/hkask-services-research/src/db.rs:581-617`

```rust
let exists: bool = conn
    .query_row(...)
    .map(|c| c > 0)
    .unwrap_or(false);  // ← swallows query errors

let feed_id: i64 = conn
    .query_row(...)
    .unwrap_or(0);  // ← swallows query errors

if feed_id == 0 {
    errors += 1;  // ← looks like "feed not found", actually a query error
    continue;
}

match conn.execute("INSERT INTO subscriptions ...") {
    Ok(_) => imported += 1,
    Err(_) => errors += 1,  // ← swallows all errors, including non-constraint
}
```

Three `unwrap_or` calls swallow DB errors. A poisoned lock, schema drift, or
disk-full condition looks like "no feed found" or "insert failed" — the real
error is lost. The `Err(_) => errors += 1` arm catches everything including
`SQLITE_BUSY`, `SQLITE_READONLY`, and constraint violations — all silently
counted as "errors" with no diagnostic.

**Fix:** Replace `unwrap_or(false)` with `?`, replace `unwrap_or(0)` with `?`,
and split the `Err(_)` arm into `Err(ConstraintViolation) => skipped` (the
only expected error) and `Err(e) => return Err(e)` (propagate real errors).

---

### N7 — `strip_html` `<li>` concatenation bug + no comment handling (Guideline)

**Constraint force:** Guideline (correctness; documented in own test)
**Severity:** Low (output formatting; no data loss)
**Location:** `crates/hkask-services-research/src/strip_html.rs:35-37,60-66`

Two issues:

1. **`<li>` concatenation:** Consecutive `<li>` elements produce
   `"- item1- item2"` instead of `"- item1\n- item2"`. The existing test
   (`strip_html_list_items`) documented this as expected behavior, but it's
   a bug — list items should be on separate lines.

2. **No HTML comment handling:** `<!-- ... -->` comments are not stripped.
   The parser treats `!` as a tag name character, so `<!-- comment -->`
   produces `!--` as a tag name and the comment content leaks into output.

**Fix:** For `<li>`, insert a newline before `- ` unless already at line
start. For comments, add a 3-char sliding window to detect `<!--` and
skip until `-->`. Update the test to assert the corrected behavior.

---

### N8 — `ResponseCache` O(n) eviction scan (Guideline)

**Constraint force:** Guideline (performance; acceptable trade-off)
**Severity:** Low (bounded by `max_entries` cap of 200)
**Location:** `crates/hkask-services-research/src/cache.rs:70-79`

`ResponseCache::insert` scans all entries to find the
least-recently-accessed one for LRU eviction. This is O(n) per insert.

**Essentialist (G1 — Exist):** Could this be O(1)? Yes, with a
`LinkedHashMap` or a doubly-linked-list + HashMap composite. But neither is
in std, and `max_entries` is capped at `MAX_CACHE_MAX_ENTRIES` (200). The
scan is bounded at 200 iterations — sub-microsecond on modern hardware.

**Fix:** Document the trade-off. No code change — the bounded size makes
O(n) acceptable. If `max_entries` grows beyond ~1000, revisit.

---

### N9 — arXiv `pdf_url` extraction breaks on multi-line `<link>` tags (Guideline)

**Constraint force:** Guideline (correctness; parser fragility)
**Severity:** Low (arXiv usually emits single-line `<link>` tags, but not guaranteed)
**Location:** `crates/hkask-services-research/src/providers/arxiv.rs:133-140`

```rust
let pdf_url = entry
    .lines()
    .find(|line| line.contains("title=\"pdf\""))
    .and_then(|line| {
        let start = line.find("href=\"")? + 6;
        let end = line[start..].find('"')?;
        Some(line[start..start + end].to_string())
    });
```

The extraction is line-oriented, but arXiv's Atom XML may emit `<link>` tags
with attributes spanning multiple lines. If `title="pdf"` and `href="..."`
are on different lines, the extraction fails and falls back to the arXiv
abstract URL.

**Fix:** Search the whole `entry` string (not line-by-line) for
`title="pdf"`, then search bidirectionally for `href="..."` within the same
`<link>` tag.

---

### N10 — `discover_feeds` fetches URL without SSRF validation (Guardrail)

**Constraint force:** Guardrail (SSRF; missing defense-in-depth)
**Severity:** Medium (reachable from `rss_discover_feeds` tool)
**Location:** `crates/hkask-services-research/src/feed.rs:68-72`

```rust
pub async fn discover_feeds(client: &Client, url: &str) -> Result<...> {
    let response = client.get(url).send().await?;  // ← no validation
    ...
}
```

`discover_feeds` is called from the `rss_discover_feeds` tool, which does
call `validate_tool_url(&url)?`. But `discover_feeds` is a `pub` function
that could be called from other code paths without validation. The
`RawFetchProvider` follows the defense-in-depth pattern (validates at the
provider boundary), but `discover_feeds` does not.

**Fix:** Add `validate_provider_url(url)` at the top of `discover_feeds`.

---

### N11 — Stored SSRF in `rss_fetch` and `import_opml` (Prohibition)

**Constraint force:** Prohibition (SSRF; stored cross-site request)
**Severity:** High (a malicious OPML file or compromised DB can seed internal URLs that `rss_fetch` later fetches)
**Location:** `mcp-servers/hkask-mcp-research/src/lib.rs:586-593` (`rss_fetch`), `crates/hkask-services-research/src/db.rs:564-625` (`import_opml`)

`rss_fetch` fetches the feed URL stored in the DB without re-validating it:

```rust
let fetch_result = fetch_feed(&self.rss_client, &feed_url, ...).await?;
```

The URL was originally user-supplied via `rss_subscribe` (which validates)
or `rss_import_opml` (which did **not** validate before this fix). A
malicious OPML file could seed the DB with `http://169.254.169.254/...`
(internal AWS metadata) or `http://localhost:8080/admin` URLs. When
`rss_fetch` runs, it fetches those internal URLs from the server's network
context — a stored SSRF.

**Pragmatic-semantics (IS vs OUGHT):**
- IS: `rss_fetch` fetches any URL in the DB without validation.
- OUGHT: `rss_fetch` should validate DB-stored URLs before fetching, because
  the URL was user-supplied and the DB is not a trusted source.

**Fix:** (1) Add `validate_tool_url(&feed_url)?` in `rss_fetch` before
`fetch_feed`. (2) Add `validate_provider_url(&url)` per-URL in `import_opml`
before inserting into the feeds table.

---

## Essentialist challenge: does each recommendation survive?

| Finding | G1 (Exist) | G2 (Surface) | G3 (Contract) | Verdict |
|---------|-----------|-------------|----------------|---------|
| N1 — `CapabilityContext` dead | Delete: no behavior change | N/A | Removes speculative API | **Delete** |
| N2 — label ops relabel feed | Remove SQL: fixes silent wrong behavior | N/A | Fields retained for compat | **Remove** |
| N3 — no transactions | Add `BEGIN`/`COMMIT` | N/A | Real ACID behavior | **Add** |
| N4 — News silent 0 | Add empty-filter check | N/A | Real error signal | **Add** |
| N5 — Tavily/SerpAPI stubs | Implement check | 2 methods | Real behavior | **Implement** |
| N6 — `unwrap_or` swallows | Replace with `?` | N/A | Real error propagation | **Fix** |
| N7 — `<li>` + comments | Fix `<li>`, add comment handling | N/A | Real behavior | **Fix** |
| N8 — O(n) eviction | Document trade-off | N/A | No code change | **Document** |
| N9 — arXiv multi-line | Fix extraction | N/A | Real behavior | **Fix** |
| N10 — `discover_feeds` SSRF | Add validation | N/A | Defense-in-depth | **Add** |
| N11 — stored SSRF | Add validation | N/A | Real defense | **Add** |

---

## Grill-me challenge: escalating interrogation

### Level 1 — Recall

**Q:** How many of the 11 findings are security-relevant?
**A:** 4 — N1 (dead OCAP), N2 (data corruption), N3 (ACID), N11 (stored SSRF).

### Level 2 — Mechanism

**Q:** How does the stored-SSRF (N11) work end-to-end?
**A:** A user calls `rss_import_opml` with a malicious OPML file containing
`xmlUrl="http://169.254.169.254/latest/meta-data/iam/security-credentials/"`.
Before the fix, `import_opml` inserted this URL into the `feeds` table
without validation. Later, `rss_fetch` resolves the feed URL from the DB
and calls `fetch_feed`, which fetches the internal URL from the server's
network context — leaking the AWS metadata response into the RSS entries
table, where the user can read it via `rss_get_entries`.

### Level 3 — Rationale

**Q:** Why was `CapabilityContext` (N1) never wired?
**A:** The 2026-07-17 fleet audit confirmed OCAP is enforced at the
dispatcher membrane (`GovernedTool`), not at the port. The port-level
`CapabilityContext` was a speculative design that was superseded by the
membrane model before it was ever constructed. It survived because it
looked like a security feature, and deleting security-looking code feels
risky — but unwired security code is worse than no code, because it creates
false confidence.

### Level 4 — Edge Cases

**Q:** What happens if `edit_tags` (N2) is called with `add_label` after the fix?
**A:** The `add_label` field is still on `EditTagRequest` (for backward-
compatible deserialization), but the SQL is removed. The field is silently
ignored — the response reports `updated` counts only for read/starred ops.
A caller relying on `add_label` will see no error but no label change
either. This is a breaking change in behavior, but the prior behavior was a
data-corruption bug, so the break is the fix.

### Level 5 — Synthesis

**Q:** What is the root cause that N1, N5, and N11 share?
**A:** Unwired security/fidelity controls. `CapabilityContext` (N1) is an
unwired OCAP gate. `TavilyProvider::health()` (N5) is an unwired health
sensor. `rss_fetch` (N11) is an unwired SSRF defense. All three are
"security-looking" code that provides no actual security because it was
never connected to the input flow. The pattern is: a developer adds a
security/fidelity feature at the type level, but the call sites never
construct the required context. The fix in all three cases is either to
wire it (N5, N11) or to delete it (N1) — never to leave it unwired.

---

## Pragmatic-cybernetics assessment

### Feedback loop: OPML import → DB → rss_fetch → internal network

| Property | Assessment |
|----------|-----------|
| Polarity | Negative (malicious URL → internal fetch → data leak) |
| Delay | **Broken** — the loop has no sensor for "is this URL safe?" (N11) |
| Gain | N/A (the loop is the attack vector) |
| Closure | **Open** — the validation step is missing |
| Fidelity | **Zero** — `rss_fetch` trusts the DB as a trusted source |

**Remediation:** Close the loop by validating DB-stored URLs at fetch time
(N11 fix). The DB is not a trusted source — it was populated from user input.

### Feedback loop: `web_ping` → provider health → user

| Property | Assessment |
|----------|-----------|
| Polarity | Negative (outage → unhealthy → user notified) |
| Delay | Low (async HTTP, sub-second) |
| Gain | 1:1 (health status directly reported) |
| Closure | Closed (ping → health → report → user sees status) |
| Fidelity | **Degraded** — Tavily and SerpAPI health checks were stubs (N5); now fixed |

### Variety check (Ashby's Law)

The 2026-07-17 review noted a variety deficit: no per-provider circuit
breaker. This remains true after the 2026-07-20 fixes — the circuit-breaker
recommendation is deferred as a separate enhancement. The 2026-07-20 fixes
address variety in the **error-reporting** dimension: N4 adds a new error
class (strategy-unsupported), and N6 adds proper error propagation for DB
failures. These increase the regulator's variety to distinguish "no results
because nothing matched" from "no results because the strategy is
unsupported" from "no results because the DB is broken."

---

## Documentation status

### Current state (2026-07-20)

| Document | Status | Issue |
|----------|--------|-------|
| `mcp-servers/hkask-mcp-research/README.md` | Lists 17 tools | Accurate; tool descriptions current |
| `docs/reference/mcp-servers/README.md` | Research row accurate | Fixed in 2026-07-17 review |
| `DIAGRAMS_INDEX.md` | DIAG-IC-013, DIAG-IC-014, DIAG-DC-012 registered | Fixed in 2026-07-17 review |
| This document | New | Follow-up review for 2026-07-20 fixes |

### Required updates

1. **This document** registers the 11 new findings and their fixes.
2. **`docs/reference/mcp-servers/README.md`** cross-link to this document.
3. **`DIAGRAMS_INDEX.md`** — no new diagrams required; the 2026-07-17
   diagrams (DIAG-IC-013, DIAG-IC-014, DIAG-DC-012) remain accurate after
   the 2026-07-20 fixes. The `CapabilityContext` removal (N1) simplifies
   the `WebSearchPort` trait but does not change the architecture diagram.

---

## Fix priority (smallest step first)

| Priority | Finding | Lines changed | Risk |
|----------|---------|----------------|------|
| 1 | N5 — Tavily/SerpAPI health stubs | ~40 (2 files) | Low |
| 2 | N7 — `strip_html` `<li>` + comments | ~30 + 3 tests | Low |
| 3 | N9 — arXiv multi-line `<link>` | ~20 | Low |
| 4 | N8 — cache eviction trade-off comment | ~10 | None |
| 5 | N10 — `discover_feeds` SSRF | ~5 | Low |
| 6 | N6 — `import_opml` error swallowing | ~30 | Low |
| 7 | N4 — News silent 0 results | ~15 | Low |
| 8 | N11 — stored SSRF (rss_fetch + import_opml) | ~15 | Low |
| 9 | N3 — SQL transactions | ~60 (5 ops) | Medium (transaction behavior) |
| 10 | N2 — `edit_tags` label ops | ~20 (remove SQL) | Medium (breaking behavior change) |
| 11 | N1 — `CapabilityContext` dead | ~80 (3 files) | Medium (API change) |

---

## Cross-links

- [Research MCP Adversarial Review (2026-07-17)](research-mcp-adversarial-review-2026-07-17.md) — prior review, all 15 findings fixed
- [MCP Server Registry](../reference/mcp-servers/README.md) — server catalog
- [MCP Fleet Test-Seam Audit](mcp-fleet-test-seam-audit-2026-07-17.md) — OCAP membrane vs. server distinction
- [Documentation Standards](../specifications/DOCUMENTATION_STANDARDS.md) — diagram and metadata rules
- [Architecture Patterns](../explanation/architecture-patterns.md) — hexagonal ports, MCP dispatch, GovernedTool membrane
