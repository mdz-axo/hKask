---
title: "GML (Allosteric Thinking) Documentation"
audience: [architects, developers]
last_updated: 2026-05-24
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

GML applies the Monod-Wyman-Changeux (MWC) allosteric model to abstract concepts [^mwc1965]:

- **Concepts** exist as probability distributions over interpretive states (T/R) [^wiener1948]
- **Effectors** (context) bind to ports and shift interpretive equilibrium [^beer1972]
- **Cooperativity** amplifies or dampens conceptual shifts [^hill1910]

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
| 5 | "Is this idea-network self-reinforcing or decaying?" | `evaluate` + `homeostasis` [^cannon1932] |

---

## Related Documentation

- [hKask Architecture](../architecture/hKask-architecture-master.md) — overall system design
- [AGENTS.md](../../AGENTS.md) — project operating guide
- [CNS Documentation](../architecture/PRINCIPLES.md) — monitoring and alerts

---

[^mwc1965]: Monod, J., Wyman, J., & Changeux, J.-P. (1965). On the nature of allosteric transitions: A plausible model. *Journal of Molecular Biology*, 12(1), 88–118. https://doi.org/10.1016/S0022-2836(65)80285-6

[^wiener1948]: Wiener, N. (1948). *Cybernetics: Or Control and Communication in the Animal and the Machine*. MIT Press.

[^beer1972]: Beer, S. (1972). *Brain of the Firm: The Managerial Cybernetics of Organization*. Allen Lane.

[^hill1910]: Hill, A. V. (1910). The possible effects of the aggregation of the molecules of haemoglobin on its dissociation curves. *Journal of Physiology*, 40(Suppl), iv–vii.

[^cannon1932]: Cannon, W. B. (1932). *The Wisdom of the Body*. W. W. Norton.

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
