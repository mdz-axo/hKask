# Task 6 — Capability Model (Mark Miller)

> **Six primitives, no more, no less.** This is the design layer.
> Tasks 4 and 5 are the *code* layer; this is the *composition* layer.
>
> A capability model is not a list of types. It is the answer to
> one question: *for every authority-bearing type, what is the
> smallest set of primitives that makes its security properties
> unforgeable?*
>
> **Method:** take the six Miller primitives (Sealer/Unsealer,
> Membrane, Brand, Attenuation, Revocation, Revocable Forwarder).
> For each `pub` type in `crates/hkask-types/src/capability/`,
> `crates/hkask-types/src/curation.rs`, `crates/hkask-keystore/`,
> and `crates/hkask-agents/src/pod/`, classify it. A type that
> requires a primitive the codebase does not implement is a finding.

## The six primitives

| # | Primitive | Definition | Question it answers |
|---|-----------|-----------|---------------------|
| 1 | **Brand** | An unforgeable proof of origin, attached to a type. | Can it be forged? |
| 2 | **Sealer/Unsealer** | A pair of references; the sealer produces sealed values, the unsealer recovers them. | Can it be smuggled past a boundary? |
| 3 | **Membrane** | An object that wraps another and translates every capability into a possibly-attenuated one. | Can authority leak across a boundary? |
| 4 | **Attenuation** | The only way to weaken a capability. Attenuation must be monotonic and capped. | Can it be amplified? |
| 5 | **Revocation** | The only way to invalidate a capability. Revocation is centralized. | Can it be replayed / outlived? |
| 6 | **Revocable Forwarder** | An object that holds a capability on behalf of another; revocation severs all forwarders. | Can it outlive its grantor? |

These are the **only six**. Anything else is composition of these
six. The hKask capability vocabulary *should* reduce to instances of
these primitives; if it does not, the vocabulary has invented a
seventh verb and the system is harder to reason about than it needs
to be.

## Type × primitive matrix

The audit below walks every authority-bearing type in hKask and
classifies which primitives it requires, which it implements, and
which it *should* implement but does not (a finding).

Legend: **✓** implemented · **~** partially · **✗** missing · **n/a** not required.

| Type | Brand | Sealer | Membrane | Attenuation | Revocation | Forwarder | Notes |
|------|-------|--------|----------|-------------|------------|-----------|-------|
| `OcapCapability::Token(OcapTokenKind)` | ✓ | n/a | n/a | n/a | n/a | n/a | The brand. The `String` variant is the F-SYN-001 finding. |
| `OCAPBoundary` | ~ | n/a | ~ | n/a | n/a | n/a | Brand is "enforced" — but the field is a `bool` (F-SYN-002). Membrane is implicit in the constructor. |
| `DelegationToken` | ✓ | n/a | n/a | ✓ | ~ | n/a | Attenuation implemented (`attenuation()`); revocation is centralized but the *verify* path may not check the log (F-SYN-006). |
| `DelegationToken::expires_at` | n/a | n/a | n/a | n/a | ~ | n/a | Revocation by time. The check is unverified by a negative test (F-SYN-014). |
| `OcapTokenKind` (inner enum) | ✓ | n/a | n/a | n/a | n/a | n/a | The sealed vocabulary. |
| `CapabilitySpec::parse` (in `hkask-types/src/capability/`) | n/a | ~ | n/a | n/a | n/a | n/a | A *sealer* in the sense that the parser produces a typed brand from a string. Correctly used as a membrane by both consumers. |
| `parse_capability` (in `acp/mod.rs` and `mcp-ocap`) | n/a | n/a | ✓ | n/a | n/a | n/a | The thin adapter is a membrane. Correct shape. |
| `OcapServer` (mcp-ocap) | n/a | n/a | ✓ | n/a | n/a | n/a | The MCP-OCAP server is the membrane over the canonical capability impl. |
| `MemoryStoragePort` (in `AgentPod`) | n/a | n/a | ✗ | n/a | n/a | ✗ | A port passed by value, not a membrane. The pod's *persistence* is not capability-gated. (F-SYN-008.) |
| `AgentPod` | ~ | n/a | n/a | n/a | ~ | ~ | The pod is a revocable forwarder: when the master is rotated, the pod is implicitly severed (because its OCAP secret is HKDF-derived from the master). But the *forwarding* is implicit; there is no explicit `RevocableForwarder` trait. |
| `RussellAcpAdapter` | n/a | n/a | n/a | n/a | ✗ | ✗ | Shared secret via HKDF — not a capability. Revocation = master rotation. (FUT-004.) |
| `Dampener` (CNS) | n/a | n/a | n/a | n/a | n/a | n/a | Not a capability primitive; a *regulation* primitive. Out of scope for this matrix. |
| `Dampener::override_cooldown` | n/a | n/a | n/a | n/a | ~ | n/a | Cooldown is a form of "soft revocation" — the override is invalidated, not the token. Per-issuer granularity is the open question. (FUT-003.) |
| `WebID` | ✓ | n/a | n/a | n/a | n/a | n/a | The unforgeable identity. |
| `Bot`, `Replicant`, `Curator` | ~ | n/a | n/a | n/a | n/a | n/a | The brand is implicit (the WebID is the brand); the *type* is a role. |
| `Visibility` (and `DataSovereigntyBoundary`) | n/a | n/a | n/a | n/a | n/a | n/a | Not a capability; an *access mode*. Adjacent to OCAP but not part of it. The `is_shared` collision is F-SYN-003. |

## Findings from the matrix (recap from SYNTHESIS, but at the model level)

1. **F-SYN-001 / F-SYN-002** — the `OcapCapability::String` variant and
   the `OCAPBoundary::enforced: bool` together are the only
   *missing* Brand primitive in the system. Everything else is
   correctly branded.
2. **F-SYN-006** — Revocation may be incomplete: verify checks the
   log, but issuance may not. The matrix says "~" not "✓".
3. **F-SYN-008** — `MemoryStoragePort` is a *port*, not a *membrane*.
   The pod has authority to write triples that the OCAP boundary
   does not gate. This is the only "✗ Membrane" entry.
4. **F-SYN-017 / FUT-004** — `RussellAcpAdapter` is a *shared
   secret*, not a *capability*. The matrix's two "✗" entries
   (Revocation, Forwarder) are appropriate — those primitives do
   not apply to a shared secret — but the *type* itself should be
   documented as out-of-scope-for-OCAP.
5. **F-SYN-012 / FUT-003** — `Dampener.override_cooldown` is a
   *regulation* primitive, not an OCAP one. The matrix correctly
   marks it "n/a" for the OCAP columns and "~" for the
   implicit-revocation column.

## How to use this matrix

When you add a new authority-bearing type:

1. Add a row to the matrix *before* writing the code.
2. Every column must be ✓, ~, ✗, or n/a. An unjustified ✗ is a finding.
3. The synthesis's "Fix shape" sections reference the matrix
   primitives. When you implement the fix, update the matrix to ✓.
4. When you remove a type (per P6), remove the row.

## The single-line capability model

> **In hKask, every authority-bearing type reduces to a Brand + a
> Membrane over one of the six primitives, with Revocation
> centralized in the OCAP server and Forwarding implicit in the
> HKDF-derived OCAP secret per AgentPod.**
>
> If a type you are about to add does not fit this sentence, stop
> and ask whether the type is *actually* authority-bearing. If it
> is, the sentence is wrong, not the type. Fix the sentence first.
