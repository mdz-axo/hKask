---
title: "GML (Allosteric Thinking) Documentation"
audience: [architects, developers]
last_updated: 2026-05-24
togaf_phase: "C — Application"
version: "0.1.0"
status: "Draft"
domain: "Application"
---

# GML (Allosteric Thinking) Documentation

**Generalized Monad Logic — KnowAct for conceptual analysis**

**Status:** Draft — GML is currently implemented as `hkask-mcp-gml` (1,022 LOC, 5 MCP tools). The multi-crate decomposition described in sub-documents is aspirational for v1.1+.

---

## Quick Start

1. **Implementing GML?** See the [Architecture](./gml-architecture.md) — crate structure, types, algebra
2. **Using the API?** Check the [API Reference](./gml-api.md) — function signatures, examples

---

## Documentation Index

| Document | Purpose | Audience |
|----------|---------|----------|
| [gml-architecture.md](./gml-architecture.md) | System design and implementation | Developers, architects |
| [gml-api.md](./gml-api.md) | API reference and SQL schema | Developers integrating GML |

---

## Overview

GML applies the Monod-Wyman-Changeux (MWC) allosteric model to abstract concepts:

- **Concepts** exist as probability distributions over interpretive states (T/R)
- **Effectors** (context) bind to ports and shift interpretive equilibrium
- **Cooperativity** amplifies or dampens conceptual shifts

**Mathematical kernel:**
```
R̄ = (1 + α)ⁿ / ((1 + α)ⁿ + L·(1 + cα)ⁿ)
```

---

## The Five Questions

| # | Question | Operation |
|---|----------|-----------|
| 1 | "What states is this idea dancing between?" | `recognize` + `equilibrium` |
| 2 | "What are its ports — what could bind and shift it?" | `parse` + `discriminate` |
| 3 | "What ideas amplify each other when co-present?" | `analogy` + `cooperate` |
| 4 | "What is suppressing this idea's generative state?" | `detect` + `inhibit` |
| 5 | "Is this idea-network self-reinforcing or decaying?" | `evaluate` + `homeostasis` |

---

## Related Documentation

- [hKask Architecture](../architecture/hKask-architecture-master.md) — overall system design
- [AGENTS.md](../../AGENTS.md) — project operating guide
- [CNS Documentation](../architecture/PRINCIPLES.md) — monitoring and alerts

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
