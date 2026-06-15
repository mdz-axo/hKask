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

hKask's economic layer uses rJoule — an internal energy currency — for agent-to-agent and human-to-agent resource transfers. The wallet crate (`hkask-wallet`) implements rJoule with multi-chain bridges to external payment systems.

Agent sessions developed the architecture across multiple handoffs (`wlt-cns-ph5-2026-06-12.md`). The `wallet-specification.md` specifies the crate architecture. This ADR documents the payment mechanism decisions.

## Decision

### rJoule as Internal Energy Currency

rJoule is the canonical unit of economic value within hKask:
- **Not a cryptocurrency.** No blockchain, no consensus, no mining.
- **Energy-backed.** 1 rJoule = 1 unit of inference compute (gas).
- **Transferable.** Agents and humans can send rJoule to each other.
- **Revocable.** A transfer can be reversed within a configurable window.

### Multi-Chain Bridge Architecture

External payment systems (Stripe, cryptocurrency networks) connect through bridge adapters:
- Each bridge implements the `PaymentBridge` trait
- Bridges convert between external currency and rJoule at configurable exchange rates
- Each transaction selects its bridge independently

### Wallet State Machine

```
Empty → Funded → Active → Frozen → Closed
         ↓         ↓
      Depleted   Revoked
```

### Key Storage

The OS keychain stores wallet private keys via `hkask-keystore` (AES-256-GCM, HKDF-SHA256). No plaintext keys touch disk.

## Consequences

- **Positive:** rJoule decouples agent economics from external payment systems.
- **Positive:** Multi-chain bridges allow incremental external integration — add one bridge at a time.
- **Negative:** Exchange rates require manual configuration. Rate automation is not implemented.
- **Negative:** Revocation depends on a revocation authority. This creates a trust dependency.

## Procedural Rhetoric

- **PS-01 (Shared Goal):** Economic layer for agent resource accounting and transfer.
- **PS-02 (Bounded Lexicon):** rJoule, PaymentBridge, wallet state machine, multi-chain bridge.
- **PS-03 (Mode of Play):** State machine with explicit transitions; bridge adapters for external systems.
- **PS-12 (Invitational Voice):** New payment bridges integrate via trait implementation.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
