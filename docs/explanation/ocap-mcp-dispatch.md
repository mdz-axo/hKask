---
title: "Object Capability (OCAP) MCP Dispatch — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Object Capability (OCAP) MCP Dispatch

## What OCAP Means in hKask

Object Capability (OCAP) security is a design discipline rooted in Mark Miller's work: you can only access something if you hold an unforgeable reference (a "capability") to it. In hKask, the capability is a `DelegationToken` — an Ed25519-signed, attenuatable bearer proof that the holder has authority from a specific issuer to perform a specific action on a specific resource.

This design exists because hKask's Magna Carta P4 (Clear Boundaries) requires that every agent, pod, and template invocation operates within explicit, unforgeable capability boundaries. There is no ambient authority. No "god token." No admin override. Every access path goes through the same gate. The `DenyAllConsent` default at `crates/hkask-agents/src/sovereignty.rs:31` makes this explicit: before a real consent port is wired, everything is denied.

## The DelegationToken

Located in `crates/hkask-capability/src/token_types.rs`, the `DelegationToken` struct carries:

- `resource: DelegationResource` — what kind of thing (Tool, Template, Registry, Key)
- `resource_id: String` — which specific thing (e.g., `"cns_health"`, `"cns"`)
- `action: DelegationAction` — Read, Write, or Execute
- `delegated_from: WebID` and `delegated_to: WebID` — provenance chain
- `signature: TokenSignature` — 64-byte Ed25519 signature
- `public_key: Ed25519PublicKey` — asymmetric verification key
- `attenuation_level: u8` / `max_attenuation: u8` — bounded to `SYSTEM_MAX_ATTENUATION` (7)
- `caveats: Vec<Caveat>` — additive restrictions inherited by children

Tokens are built via `DelegationTokenBuilder` and signed using the issuer's Ed25519 `SigningKey`. The token ID is a SHA-256 hash of (resource + resource_id + action + from + to), making tokens content-addressable. The `verify()` method at line 274 reconstructs the canonical signing payload byte-for-byte and checks the signature against the token's embedded public key. This is asymmetric — you only need the public key to verify, not a shared secret.

### Attenuation

The `attenuate()` method (line 355) creates a child token with `attenuation_level + 1`, 1-hour expiry, and a chained context nonce (`"root-attenuated-uuid"`). All caveats are inherited. `can_attenuate()` returns false when `attenuation_level >= max_attenuation`, enforcing that tokens can only be weakened, never strengthened — the core OCAP property.

## The GovernedTool Membrane

The `GovernedTool<P: ToolPort>` at `crates/hkask-cns/src/governed_tool.rs` is the singular membrane through which all MCP tool invocations pass. It implements `ToolPort` itself — this is Miller's membrane object pattern: the wrapper IS a tool port, indistinguishable to callers, but it adds governance.

### The 6-Step Dispatch

The `invoke()` method (line 200) enforces a strict sequence:

1. **Check (cryptographic)** — `token.verify()` performs Ed25519 signature verification. Failure returns `ToolPortError::CapabilityDenied`. No further steps execute.

2. **Check (OCAP authority)** — Two-path verification. Path 1 (exact): `verify_capability_exact()` checks `token.is_valid_for(DelegationResource::Tool, tool_name, DelegationAction::Execute)`. This handles ad-hoc invocation tokens minted with the exact tool name. Path 2 (domain): `verify_capability_domain_fallback()` looks up the tool's `required_capability` metadata and uses `capabilities_match()` to test whether the token's domain covers the required domain. For example, an agent token for `tool:cns:execute` grants access to any tool whose `required_capability` is `tool:cns:execute` (via `capabilities_match` at `crates/hkask-capability/src/resources.rs:122`). The action hierarchy is Execute ≥ Write ≥ Read.

3. **Reserve (gas)** — `CyberneticsLoop::can_proceed()` checks whether the agent's gas budget has enough remaining capacity. If `ToolStats` is wired (Layer 1), the reserve uses the 90th percentile of the fitted LogNormal cost distribution; otherwise it falls back to the `EnergyEstimator` point estimate. If insufficient, a `cns.gas.depleted` span is emitted and `ToolPortError::EnergyBudgetExceeded` is returned. Then `reserve_gas()` atomically decrements the budget. This is the hold-settle pattern: gas is reserved before invocation, then settled after with the actual cost. If actual < reserved, the difference is refunded — preventing gas leaks from over-estimation.

4. **ν-event (invoked)** — A `NuEvent` with span `SpanNamespace("cns.tool")` and path `"invoked"` is persisted via `NuEventSink`. The phase is `CyclePhase::Sense`, marking this as an observation entering the CNS. The payload carries server, tool, estimated_cost, and `settled: false`.

5. **Delegate** — The inner `ToolPort` is called. This is the only step that does actual work. Everything before was permission; everything after is accounting.

6. **Settle + ν-event (completed)** — `settle_gas()` refunds the difference between reserved and actual cost. A `cns.gas.settled` span is emitted. Then a `cns.tool.completed` span is emitted with the parent set to the invoked event's ID, creating a causal chain. Additionally, `ToolStats::record()` logs the outcome for statistical learning, and `ToolConsumptionEvent` is sent on a direct `mpsc` channel to CyberneticsLoop.

### Fail-Closed Semantics

Every step in the chain fails closed. Cryptographic verification fails? Denied. OCAP check fails? Denied. Gas budget exhausted? Denied. The `governed_tool.rs` tests at line 505 verify this: `exact_match_denies_wrong_tool`, `domain_capability_denies_different_domain`. The `CapabilityDenied` error is returned before any resource is consumed. In the `sovereignty.rs` module, `SovereigntyChecker::can_access()` adds a second gate: even with a valid capability token, sovereign data requires the requester to BE the owner AND have explicit consent. `DenyAllConsent` is the default — a misconfigured checker always denies.

## PerPodToolBinding — Structural Pod Isolation

At `crates/hkask-agents/src/pod/deployment.rs`, `PodDeployment` holds `tools: PerPodToolBinding`, which wraps `mcp_runtime: Arc<dyn MCPRuntimePort>` and an optional `governed_tool: Option<Arc<GovernedTool<RawMcpToolPort>>>`. A pod can invoke tools only through its own binding and the configured system/A2A trust roots; cross-pod access requires a valid `DelegationToken`. SQLCipher database encryption is a separate concern and consistently uses the canonical `HKASK_DB_PASSPHRASE` resolver.

This means P4.1 (pod boundary = OCAP perimeter) is enforced by construction: the type system prevents a pod from even addressing another pod's governed tool. The only path is through the proper delegation chain.

## How This Enforces P4 (Clear Boundaries)

The Magna Carta at `docs/architecture/core/magna-carta.md` defines P4 as a dual enforcement gate: `require_capability` (do you hold a token?) and `require_sovereignty` (does the data owner consent?). `GovernedTool` enforces the capability half; `SovereigntyChecker` enforces the sovereignty half. Together they form the unbypassable two-gate membrane. Every tool invocation, memory access, and template execution passes through both. The `CapabilityChecker` at `crates/hkask-capability/src/verification.rs` provides a unified check entry point, combining both gates into a single fail-closed decision.

The system trusts the type system, not runtime configuration. If the code compiles with these membranes in place, the boundaries hold.
