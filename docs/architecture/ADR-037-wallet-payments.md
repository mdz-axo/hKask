---
title: "ADR-037 — Wallet Payment Mechanism Architecture"
audience: [architects, developers]
last_updated: 2026-06-14
version: "0.27.0"
status: "Draft"
domain: "Technology"
mds_categories: [domain, trust, lifecycle]
---

# ADR-037 — Wallet Payment Mechanism Architecture

**Status:** Draft  
**Date:** 2026-06-14  
**References:** `docs/plans/2026-06-12-wallet-payment-mechanism.md`, `docs/plans/2026-06-12-wallet-rjoule-payments.md`, `docs/architecture/wallet-specification.md`

---

## Context

hKask's economic layer requires a payment mechanism for agent-to-agent and human-to-agent resource transfers. The wallet crate (`hkask-wallet`) implements rJoule — an internal energy currency — with multi-chain support for bridging to external payment systems.

The architecture was developed across multiple sessions (handoffs: `wlt-cns-ph5-2026-06-12.md`) and specified in `wallet-specification.md`. This ADR documents the payment mechanism decisions.

## Decision

### rJoule as Internal Energy Currency

rJoule is the canonical unit of economic value within hKask. It is:
- **Not a cryptocurrency.** No blockchain, no consensus, no mining.
- **Energy-backed.** 1 rJoule = 1 unit of inference compute (gas).
- **Transferable.** Agent-to-agent, human-to-agent, agent-to-human.
- **Revocable.** Transfers can be reversed within a configurable window.

### Multi-Chain Bridge Architecture

External payment systems (Stripe, cryptocurrency networks) connect via bridge adapters:
- Each bridge implements a `PaymentBridge` trait
- Bridges convert external currency ↔ rJoule at configurable exchange rates
- Bridge selection is per-transaction, not global

### Wallet State Machine

```
Empty → Funded → Active → Frozen → Closed
         ↓         ↓
      Depleted   Revoked
```

### Key Storage

Wallet private keys are stored in the OS keychain via `hkask-keystore` (AES-256-GCM, HKDF-SHA256). No plaintext keys on disk.

## Consequences

- **Positive:** Internal currency decouples agent economics from external payment systems.
- **Positive:** Multi-chain bridge architecture allows incremental external integration.
- **Negative:** Exchange rate management is not yet automated — requires manual configuration.
- **Negative:** Revocation window introduces a trust dependency on the revocation authority.

## Procedural Rhetoric

- **PS-01 (Shared Goal):** Economic layer for agent resource accounting and transfer.
- **PS-02 (Bounded Lexicon):** rJoule, PaymentBridge, wallet state machine, multi-chain bridge.
- **PS-03 (Mode of Play):** State machine with explicit transitions; bridge adapters for external systems.
- **PS-12 (Invitational Voice):** New payment bridges are invited via trait implementation.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
