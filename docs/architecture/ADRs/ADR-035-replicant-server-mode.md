---
title: "ADR-035: Replicant Server Mode — AgentMode, Daemon Transport, Dual Memory Encoding"
audience: [architects, developers]
last_updated: 2026-06-30
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, trust, lifecycle]
---

# ADR-035: Replicant Server Mode

**Date:** 2026-06-12
**Status:** Active
**Related:** [`MDS.md`](../core/MDS.md), [`magna-carta.md`](../core/magna-carta.md)

## Context

hKask's 11 MCP servers provide tool capabilities to agents. The original architecture treated MCP servers as standalone binaries spawned by the hKask runtime (`McpRuntime::start_server()`). This created a gap: MCP servers installed in IDEs (Zed, VSCode) had no access to hKask's agent identity, OCAP governance, or memory infrastructure. They operated as anonymous tool providers with no narrative context.

**Problem Statement:** How should MCP servers operate when installed in IDE environments, while maintaining hKask's Magna Carta principles (User Sovereignty, Affirmative Consent, OCAP boundaries) and integrating with the agent memory stack?

**Stakeholders:** IDE users, agent developers, MCP server implementers

**Constraints:** Headless (P1.6), out-of-process isolation (P1), Magna Carta P1-P4

## Decision

**MCP servers are served by replicants operating in "server mode."** The MCP binary is a thin launcher that authenticates the serving replicant through a Unix domain socket daemon. The daemon mediates P4 dual-gate verification (OCAP capability + sovereignty/consent assignment) and dual memory encoding (episodic + semantic). Every 10 tool calls, the daemon triggers internal narrative generation via inference — the agent "thinks about" what it's observing.

### Architecture

1. **AgentMode** (`Chat` | `Server`) — a property of the agent, not the MCP runtime. Initially mutually exclusive.
2. **Daemon socket** (`~/.config/hkask/daemon.sock`) — Unix domain socket for out-of-process MCP binary ↔ in-process agent stack communication. JSON newline-delimited protocol.
3. **ServiceDaemonHandler** — bridges daemon queries to ActivePods (assignment, capability), UserStore (authentication), InferencePort (narrative generation), and PodContext (dual memory encoding).
4. **Thin launcher pattern** — each MCP binary reads `HKASK_MCP_HOST` from env, connects to daemon for P4 dual-gate verification, then starts serving. Original tool logic unchanged.
5. **Dual memory encoding** — every tool call produces both episodic (first-person, private, perspective-scoped) and semantic (third-person, public, generalized) triples simultaneously.
6. **Narrative generation** — every 10 stored experiences, the daemon queries the agent's episodic memory, calls inference to produce observations about patterns and user intent, and stores those observations as additional episodic memories.

### Startup Flow

```
1. kask login <replicant>              → session in UserStore (P2: Affirmative Consent)
2. kask pod assign <replicant> <role>  → assigned_mcp_roles populated (P4 Gate 2: sovereignty)
3. kask pod mode <replicant> server -r <role> → enter_server_mode() (P4 Gate 1: OCAP)
4. IDE spawns MCP binary with HKASK_MCP_HOST=<replicant>
5. Binary → daemon: auth_query → assignment_query → capability_query
6. All gates pass → MCP server starts
7. Tool calls → record_experience() → daemon store_experience → dual encoding
8. Every 10 calls → generate_narrative() → inference → narrative observations
```

### Memory Flow

```
Tool call
  │
  ▼
record_experience() [MCP binary, fire-and-forget]
  │
  ▼
daemon store_experience [Unix socket]
  ├─→ episodic: "mcp_session"/"observed" (first-person, private)
  └─→ semantic: "mcp_session"/"observed" (third-person, public, generalized)
  │
  ▼ (every 10 experiences)
generate_narrative()
  ├─→ query episodic "mcp_session" triples
  ├─→ inference: "What patterns? What is the user trying to accomplish?"
  └─→ store observations as episodic "narrative"/"thought"
  │
  ▼ (existing pipeline)
consolidation → semantic knowledge
```

### CLI Commands

```bash
kask pod assign <name> <role>              # Assign MCP role
kask pod mode <name> server -r <role>      # Enter server mode
kask pod mode <name> chat                  # Enter chat mode
kask pod mode <name> exit                  # Exit current mode
```

## Consequences

### Positive

- MCP servers in IDEs have full hKask identity, OCAP governance, and memory integration
- Single binary serves both IDE and hKask contexts — no compile-time mode flags
- P4 dual-gate enforced at connection time (capability + assignment)
- P2 affirmative consent via passphrase-gated session (no passphrase stored with MCP binary)
- Agent accumulates episodic memory from server-mode activity — learns from what it observes
- Narrative generation gives the agent "thoughts" about MCP sessions, parallel to chat-mode cognition
- Unix domain socket provides kernel-enforced local-only transport (P1: local-first)
- Zero configuration — daemon path is well-known (`~/.config/hkask/daemon.sock`)

### Negative

- Requires hKask to be running as background service when MCP servers are used
- Mode mutual exclusion (initial): agent can't chat while serving
- Narrative generation depends on inference availability
- 10-experience threshold for narrative generation is a fixed constant (may need tuning)

### Neutral

- Daemon is started automatically via `AgentService::build()` — no separate `kask daemon` command needed initially
- Graceful fallback: MCP binaries operate without daemon (direct mode) if daemon unavailable
- Thin launcher pattern adds ~50 lines per MCP server — tool logic unchanged

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P1** (User Sovereignty) | ✅ | Unix socket is kernel-enforced local-only. Data stays on user's machine. |
| **P2** (Affirmative Consent) | ✅ | Passphrase entry via `kask login` creates session. Daemon checks session — fail-closed. |
| **P3** (Generative Space) | ✅ | Daemon path is well-known, no hidden settings. All 11 MCP servers equally exposed. |
| **P4** (Clear Boundaries/OCAP) | ✅ | Dual gate: capability (OCAP token) + assignment (sovereignty/consent). Both must pass. |
| **P6** (Delete stubs) | ✅ | No `todo!()`, no `unimplemented!()`. All 11 servers converted. |
| **C1** (Type worn before tailored) | ✅ | `AgentMode` enum with two variants, used by all agents. |
| **C5** (Every error variant unique) | ✅ | `ModeConflict`, `ModeRequiresActivation`, `RoleNotAssigned` — distinct recovery paths. |

## Verification

```bash
# Verify daemon socket path
grep -r "daemon.sock" crates/ --include="*.rs" | wc -l

# Verify all 11 MCP servers have try_daemon_flow
grep -r "try_daemon_flow" mcp-servers/ --include="*.rs" | wc -l

# Verify AgentMode tests
cargo test -p hkask-agents -- mode

# Verify daemon tests
cargo test -p hkask-mcp -- daemon

# Verify no stubs
grep -r "todo!\|unimplemented!" mcp-servers/ crates/hkask-mcp/src/daemon.rs --include="*.rs" | wc -l
```

**Expected Results:**
- Daemon socket path referenced in `hkask-mcp` and service layer subcrates (`the service layer subcrates`)
- All 11 MCP servers implement `try_daemon_flow`
- 4 AgentMode tests pass (activation, exclusion, assignment, switch)
- 5 daemon tests pass (auth, unauth, assignment, capability, dual-encoding)
- Zero stubs

## Related Documents

- [`magna-carta.md`](../core/magna-carta.md) — P1-P4 principles enforced by this architecture
- [`loop-architecture.md`](../loop-architecture.md) — Memory loops (episodic, semantic, consolidation)

## References

[^ocap]: Miller, M. (2006). *Robust Composition: Towards a National Research Agenda for Object Capability Security*. HP Labs.
[^solid]: Berners-Lee, T. et al. (2016). *SOLID: Social Linked Data*. MIT CSAIL.
[^beer-cybernetics]: Beer, S. (1981). *Brain of the Firm*. Wiley.

---

*ℏKask - A Minimal Viable Container for Replicants — ADR-035 — v0.28.0*
