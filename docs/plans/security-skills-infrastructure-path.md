---
title: "Security Skills — Path of Action for Remaining Infrastructure Work"
audience: [architects, userpods, security auditors]
last_updated: 2026-07-18
status: active
version: 0.1.0
domain: security / infrastructure / mcp
---

# Path of Action — Security Skills Infrastructure Work

This document addresses the open infrastructure questions raised during the
security skill development. The skills themselves are complete and pass all
CI gates. The remaining work is **runtime infrastructure** — making the
skills fully invocable in practice.

## Current State (2026-07-18)

All four security skills are registry-committed and pass `kask skill audit`
(score 1.00, 0 defects, 52/52 skills active):

| Skill | Registry | Status | Invocable? |
|-------|----------|--------|-----------|
| `kali-audit` | `registry/templates/kali-audit/` | Enforced | ✅ Yes — uses `file:read`, `code:search`, `terminal` MCP tools |
| `supply-chain-sentinel` | `registry/templates/supply-chain-sentinel/` | Active | ✅ Yes — uses `file:read`, `code:search`, `terminal` MCP tools |
| `runtime-posture-monitor` | `registry/templates/runtime-posture-monitor/` | Active | ⚠️ Partial — needs CNS span history reader (see §1) |
| `attack-taxonomy-mapper` | `registry/templates/attack-taxonomy-mapper/` | Active | ✅ Yes for post-audit mapping; ⚠️ Limited for real-time (see §2) |

## 1. CNS Span History Reader (for `runtime-posture-monitor`)

### Problem

The `runtime-posture-monitor` skill instructs the agent to observe `hkask.*`
performative spans and `cns.*` canonical spans. However, there is no MCP
tool for querying CNS span history. The existing infrastructure has:

- `RegulationArchive` (`crates/hkask-storage/src/nu_event_store.rs`) — stores
  events in SQLite, has `query_algedonic()` but NO general-purpose
  `query_by_target()` or `query_by_namespace()`.
- `LedgerStoragePort` trait (`crates/hkask-ports/src/cns.rs`) — defines
  `query_algedonic()` and `replay_weighted()` but NO general query method.
- No MCP server for CNS span queries (existing MCP servers: codegraph,
  communication, companies, condenser, curator, docproc, filesystem,
  kata-kanban, media, memory, replica, research, scenarios, skill, training).

### Recommended Path

**Option A: Add `query_by_namespace` to `RegulationArchive` + `LedgerStoragePort`**

Minimal, surgical change:
1. Add `query_by_namespace(namespace: &str, since: DateTime<Utc>, limit: u64)`
   to `RegulationArchive` — queries `nu_events` table by `span_category` prefix.
2. Add the method to the `LedgerStoragePort` trait.
3. Create `mcp-servers/hkask-mcp-cns/` MCP server with a `cns_query_spans`
   tool that calls `LedgerStoragePort::query_by_namespace`.
4. The `runtime-posture-monitor` skill can then instruct the agent to use
   the `cns_query_spans` MCP tool.

**Effort:** Medium (1 new MCP server + 1 trait method + 1 store method).
**Risk:** Low — follows existing patterns (codegraph MCP server is the model).
**Value:** High — enables `runtime-posture-monitor` and any future skill
that needs to query CNS telemetry.

**Option B: Extend existing `curator` MCP server**

Add a `cns_query_spans` tool to `mcp-servers/hkask-mcp-curator/` (which
already has access to the CuratorContext and RegulationArchive).

**Effort:** Low (1 new tool in existing server).
**Risk:** Low — but conflates curator concerns with CNS query concerns.
**Value:** Medium — works but violates separation of concerns.

**Recommendation:** Option A. Create a dedicated `hkask-mcp-cns` MCP server.
This follows the hKask pattern of one MCP server per domain (codegraph for
code understanding, curator for curation, etc.) and keeps CNS query
concerns separate from curator concerns.

### Implementation Steps (Option A)

1. Add `query_by_namespace()` to `RegulationArchive` (query `nu_events` by
   `span_category` LIKE prefix).
2. Add `query_by_namespace()` to `LedgerStoragePort` trait.
3. Implement `LedgerStoragePort::query_by_namespace` in `RegulationArchive`.
4. Create `mcp-servers/hkask-mcp-cns/` with:
   - `cns_query_spans` tool (query by namespace + time window)
   - `cns_span_stats` tool (aggregate counts by namespace)
   - Tool-behavior contract tests (per CI gate requirement)
5. Register in `hkask-cli` MCP server bootstrap.
6. Update `runtime-posture-monitor` SKILL.md to reference the `cns_query_spans`
   MCP tool.

## 2. Finding Consumption API (for `attack-taxonomy-mapper`)

### Problem

The `attack-taxonomy-mapper` skill reads `security/regressions/` YAML files
for findings to map. This works for **post-audit taxonomy mapping** (mapping
already-merged regression entries) but NOT for **real-time incident
investigation** (mapping fresh findings from a current `supply-chain-sentinel`
or `kali-audit` audit cycle that haven't been merged yet).

### Recommended Path

**For post-audit mapping (primary use case):** No change needed. The skill
reads `security/regressions/` YAML files, which is sufficient for mapping
findings after they've been reviewed and merged. This is the primary use
case documented in the SKILL.md.

**For real-time incident investigation (secondary use case):** This would
require an inter-skill data flow mechanism — either:
- A shared findings store (e.g., `security/findings/` for pending findings
  not yet merged into `security/regressions/`).
- A CNS-based finding passing mechanism (findings emitted as `cns.taxonomy.*`
  spans that the mapper can query).

**Recommendation:** Do NOT implement the real-time path now. The primary
use case (post-audit mapping) is fully functional. The real-time path is
speculative — it would add complexity (P5 violation) without a confirmed
use case. If a real-time incident investigation need arises, it can be
addressed then with a dedicated finding-passing mechanism.

## 3. `kind: cns-span` CI Enforcement

### Problem

`scripts/check-kali-regressions.sh` now acknowledges `kind: cns-span`
regressions (added in this work) but cannot mechanically enforce them —
it defers them with a "deferred" message. The script runs `grep` on source
files, but `cns-span` regressions check for runtime span patterns that
cannot be detected by source-file grep.

### Recommended Path

**Option A: Runtime CI check (deferred)**

Add a separate CI step that queries the CNS span history (using the
`hkask-mcp-cns` server from §1) to check for span patterns that should
NOT be present. This would be a new script: `scripts/check-runtime-regressions.sh`.

**Effort:** Medium (depends on §1 being completed first).
**Risk:** Medium — requires a running hKask instance with CNS telemetry
in CI, which may not be feasible in the current CI setup.
**Value:** Medium — only needed when the first `surface: runtime`
regression is flipped to `status: enforced`.

**Option B: Static code correlate (simpler)**

For runtime threats that have a static code correlate (e.g., a tool call
chain that can be statically detected), use `kind: grep` against the code
pattern instead of `kind: cns-span`. This narrows the skill's scope but
is mechanically enforceable today.

**Effort:** None (already supported).
**Risk:** Low.
**Value:** Medium — covers threats with static correlates; misses
pure-runtime threats (e.g., inference rate spikes).

**Recommendation:** Option B for now (use `kind: grep` where possible).
Option A when the first `surface: runtime` regression with a pure-runtime
pattern needs enforcement. The `kind: cns-span` detection type is
documented and acknowledged by the CI gate — it's ready for when the
runtime infrastructure (§1) is in place.

## 4. OWASP SC Codes Verification

### Problem

The `attack-taxonomy-mapper` skill references OWASP SC codes (SC04–SC09)
as PROPOSED mappings. OWASP does not publish a numbered Supply Chain Top 10
in the same format as the LLM Top 10. The convergence metric treats OWASP SC
as a secondary presence/absence signal.

### Recommended Path

**Option A: Drop OWASP SC codes entirely**

Remove the `owasp_sc_reference` field from the `taxonomy_mapping` format.
Rely solely on OSC&R (verified) as the taxonomy mapping. This is the
cleanest approach — OSC&R is the verified, open-source framework designed
specifically for supply chain attacks.

**Effort:** Low (remove field from templates + SKILL.md).
**Risk:** Low — OSC&R is the primary taxonomy anyway.
**Value:** Medium — simplifies the skill, removes unverified claims (P8).

**Option B: Verify against OWASP documentation**

Research the actual OWASP Software Supply Chain Security project
documentation and map the proposed codes to real OWASP categories.

**Effort:** Medium (research + verification).
**Risk:** Low.
**Value:** Low — OSC&R already provides the attack pattern taxonomy;
OWASP SC adds a parallel but unverified layer.

**Recommendation:** Option A. Drop the OWASP SC codes. OSC&R is the
verified, purpose-built taxonomy for supply chain attacks. The OWASP SC
reference adds complexity without verified value. The `taxonomy_mapping`
field becomes: `osc_r_tactic`, `osc_r_technique`, `osc_r_categories` (all
verified against `github.com/pbom-dev/OSCAR`).

## 5. `security-scan-pdca` Skill Disposition

### Problem

The `security-scan-pdca` skill is superseded by the newer native security
skills (`kali-audit`, `supply-chain-sentinel`, `runtime-posture-monitor`,
`attack-taxonomy-mapper`). It references external services (Snyk, Semgrep)
rather than providing native hKask audit, and has unverified claims (FIBO
analog, dependency manifest mapping). I fixed its audit defects (FlowDef →
KnowAct, added SKILL.md) so it passes CI, but it's still stale design.

### Recommended Path

**Option A: Remove the skill**

Delete `registry/templates/security-scan-pdca/`, `registry/manifests/security-scan-pdca.yaml`,
`.agents/skills/security-scan-pdca/SKILL.md`, and the two
`security/unverified/` evidence documents. Update any references.

**Effort:** Low (delete files + update references).
**Risk:** Low — the skill is not used by any other skill or CI gate.
**Value:** Medium — removes stale, superseded design with unverified claims.

**Option B: Keep as historical reference**

Leave the skill in place but mark it as deprecated in the SKILL.md (already
done — "Superseded — retained for backward compatibility").

**Effort:** None (already done).
**Risk:** None.
**Value:** Low — preserves history but adds clutter.

**Recommendation:** Option A. Remove the skill. It's superseded, has
unverified claims that violate P8, and keeping it adds clutter. The
`security/unverified/` evidence documents can also be removed since the
unverified claims they document belong to the removed skill.

## Prioritized Implementation Order

1. **§4: Drop OWASP SC codes** — Low effort, removes unverified claims (P8).
   Can be done immediately.
2. **§5: Remove `security-scan-pdca`** — Low effort, removes stale design.
   Can be done immediately.
3. **§2: Finding consumption API** — No action needed (post-audit mapping
   is the primary use case and is functional).
4. **§1: CNS span history reader** — Medium effort, enables
   `runtime-posture-monitor`. Implement when the skill is needed for
   production use.
5. **§3: `kind: cns-span` CI enforcement** — Depends on §1. Implement when
   the first `surface: runtime` regression with a pure-runtime pattern
   needs enforcement.

## Summary

The security skills are complete and pass all CI gates. The remaining work
is infrastructure — primarily the CNS span history reader (§1) that would
make `runtime-posture-monitor` fully invocable. The other items (§2–§5) are
either no-ops (§2), low-effort cleanups (§4, §5), or deferred until needed
(§3). None of the remaining work blocks the skills from being used for
their primary use cases.
