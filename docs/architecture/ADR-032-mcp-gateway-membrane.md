---
title: "ADR-032: MCP Gateway Membrane Policy"
audience: [architects, security engineers, developers]
last_updated: 2026-06-07
version: "1.0.0"
status: "Draft"
domain: "Technology"
ddmvss_categories: [capability, trust]
---

# ADR-032: MCP Gateway Membrane Policy

**Date:** 2026-06-07
**Status:** Draft
**Related:** [`domain-and-capability.md`](domain-and-capability.md) §5.5, OPEN_QUESTIONS.md FUT-004

## Context

hKask's 21 MCP servers provide tool capabilities to agents. The capability model documented in `domain-and-capability.md` §5.5 describes a "capability membrane" — a boundary that gates access to tools based on OCAP tokens. However, only 2 of 21 MCP servers currently implement this membrane via `GovernedTool`:

| Server | Membrane | Rationale |
|--------|----------|-----------|
| `hkask-mcp-keystore` | ✅ `GovernedTool` | Secrets must not leak across agent boundaries |
| `hkask-mcp-ocap` | ✅ `GovernedTool` | Capability tokens are trust infrastructure |
| 19 others | ❌ Passthrough | No OCAP check on tool invocation |

This creates an architectural gap: the documented capability membrane is selectively permeable. Agents can invoke 19 servers' tools without presenting any capability token.

**Problem Statement:** Should the MCP gateway be a membrane for all servers, or a passthrough for servers without side effects?

**Stakeholders:** Agent developers, security reviewers, MCP server implementers

**Constraints:** Headless constraint (§1.6) — no visual UI for capability negotiation; 21-server surface area must remain tractable

## Decision

**Classify MCP servers into two tiers with explicit membrane policy per tier.**

### Tier 1: Membrane (OCAP-gated)

Servers that manage state, secrets, or trust infrastructure MUST gate all tool calls through `GovernedTool`. Capability tokens are required for every invocation.

- `hkask-mcp-keystore` (secrets)
- `hkask-mcp-ocap` (trust infrastructure)
- `hkask-mcp-cns` (homeostatic state — write operations)
- `hkask-mcp-spec` (spec governance — write operations)

### Tier 2: Passthrough (OCAP-exempt)

Servers that provide read-only queries or agent-scoped operations with no cross-agent state MAY be exempted from `GovernedTool`. Read-only tools on these servers are documented as "capability-exempt" in their tool descriptions.

- Read operations on spec, CNS (status queries)
- Agent-scoped memory operations (episodic, semantic)
- Inference tools (model selection is per-agent, not cross-agent)

### Implementation

1. Each MCP server declares its membrane tier in its server metadata (`"membrane": "governed"` or `"membrane": "passthrough-read"`).
2. `hkask-mcp` (dispatch) verifies the tier at tool registration time.
3. Tier 1 servers MUST use `GovernedTool` for all tools.
4. Tier 2 servers MUST use `GovernedTool` for any tool with cross-agent side effects (writes, deletes).
5. Tier 2 servers MAY use passthrough for read-only, agent-scoped tools.

**Alternatives Considered:**
1. **All servers membrane** — Rejected: 19 of 21 servers have read-only or agent-scoped tools. Requiring tokens for every read query adds latency and complexity without security benefit (an agent can only read its own memory).
2. **No membrane** — Rejected: Secrets and trust infrastructure must be protected. This was the pre-ADR state and it violates the zero-trust model.
3. **Per-tool membrane** — Rejected: Too granular. A server-level tier is easier to audit and enforce.

**Rationale:** The two-tier model matches the actual threat model: cross-agent state (secrets, trust) needs protection; per-agent state (memory, inference) is already scoped by `WebID`.

## Consequences

### Positive

- Explicit membrane policy eliminates the "selectively permeable" gap
- Tier 2 servers can optimize read paths (no token verification latency)
- New MCP servers have a clear classification process (Tier 1 if cross-agent state, Tier 2 if agent-scoped)
- Security auditors can verify membrane coverage per-server, not per-tool

### Negative

- Two tiers require documentation and enforcement discipline
- Tier 2 classification could be misused to avoid membrane on tools that should be Tier 1
- Write operations on Tier 2 servers still need OCAP checks (mixed-tier server complexity)

### Neutral

- This ADR formalizes the current de facto state, adding classification and documentation
- Implementation is incremental: existing Tier 2 servers don't change, Tier 1 additions are additive

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P1** (No trait without two consumers) | ✅ | `GovernedTool` consumed by keystore + ocap servers; passthrough consumed by 19 others |
| **P2** (No generic without two instantiations) | ✅ | Two tiers, each with multiple instances |
| **P3** (No module directory without encapsulation) | ✅ | Membrane policy encapsulates capability boundary |
| **P6** (Delete stubs, don't publish) | ✅ | No stub membrane — each server is either governed or documented-exempt |
| **C1** (Type worn before tailored) | ✅ | Tier classification declared in server metadata |
| **C5** (Every error variant is unique recovery path) | ✅ | `GovernedTool` errors: `Unauthorized`, `TokenExpired`, `InsufficientCapability` |

## Verification

```bash
# Verify Tier 1 servers use GovernedTool
grep -r "GovernedTool" mcp-servers/hkask-mcp-keystore/ mcp-servers/hkask-mcp-ocap/ | wc -l

# Verify all servers declare membrane tier in metadata
grep -r '"membrane"' mcp-servers/*/src/ | wc -l

# Verify no stub membrane
grep -r "todo!\|unimplemented!" mcp-servers/ --include="*.rs" | wc -l
```

**Expected Results:**
- Tier 1 servers use `GovernedTool` for all tool registrations
- All 21 servers declare membrane tier
- No stub implementations

## Related Documents

- [`domain-and-capability.md`](domain-and-capability.md) §5.5 — Capability membrane model
- [`trust-security-observability.md`](trust-security-observability.md) §1 — Security model
- [`OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md) FUT-004 — MCP gateway membrane question

## References

[^ocap]: Miller, M. (2006). *Robust Composition: Towards a National Research Agenda for Object Capability Security*. HP Labs.
[^stride]: Howard, M. & Lipner, S. (2006). *The STRIDE Threat Model*. Microsoft.

---

*ℏKask - A Minimal Viable Container for Agents — ADR-032 — v0.23.0*