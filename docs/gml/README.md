---
title: "GML (Allosteric Thinking) Documentation"
audience: [architects, developers]
last_updated: 2026-05-28
version: "0.1.1"
status: "Active"
domain: "Application"
ddmvss_categories: [domain]
---

# GML (Allosteric Thinking) Documentation

**Generalized Monad Logic — KnowAct for conceptual analysis**

**Implementation:** GML is currently implemented as `hkask-mcp-gml` (1,022 LOC, 5 MCP tools: `recognize`, `equilibrium`, `parse`, `discriminate`, `analogy`). Multi-crate decomposition is deferred to v1.1+.

---

## Quick Start

GML is available as an MCP server. Invoke its tools through the standard `kask` surfaces:

```bash
kask chat              # Interactive chat with GML tools
kask mcp call gml recognize --args '{"concept": "democracy"}'
```

---

## Documentation Index

| Document | Purpose | Audience |
|----------|---------|----------|
| This document | GML overview and quick start | All |

> **Note:** The aspirational multi-crate GML architecture and API reference documents were archived during the 2026-05-28 documentation refresh. They described a planned decomposition that has not been implemented. The current implementation is the single `hkask-mcp-gml` crate.

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

*ℏKask — A Minimal Viable Container for Agents — GML v0.1.0*
