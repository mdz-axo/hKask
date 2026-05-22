# GML (Allosteric Thinking) Documentation

**Generalized Monad Logic — KnowAct for conceptual analysis**

---

## Quick Start

1. **New to GML?** Start with the [User Guide](./gml-user-guide.md) — introduces the Five Questions method
2. **Implementing GML?** See the [Architecture](./gml-architecture.md) — crate structure, types, algebra
3. **Using the API?** Check the [API Reference](./gml-api.md) — function signatures, examples
4. **Contributing research?** Review the [Research Agenda](./gml-research-agenda.md) — open questions

---

## Documentation Index

| Document | Purpose | Audience |
|----------|---------|----------|
| [gml-user-guide.md](./gml-user-guide.md) | How to use GML for conceptual analysis | End users, analysts |
| [gml-architecture.md](./gml-architecture.md) | System design and implementation | Developers, architects |
| [gml-api.md](./gml-api.md) | API reference and SQL schema | Developers integrating GML |
| [gml-research-agenda.md](./gml-research-agenda.md) | Open questions and research directions | Researchers, contributors |

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
- [CNS Documentation](../architecture/) — monitoring and alerts

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
