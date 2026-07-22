# hkask-bridge-fibo

FIBO Financial Industry Business Ontology bridge — canonical concept URIs for financial and business analysis.

Part of the dual-axis ontological framework (P5.4). Thin mapping layer: canonical URI constants, no dependencies, no reasoners, no overhead. Mirrors `hkask-bridge-dublincore` and `hkask-bridge-pko`.

Used by `docproc extract_triples` for expository passages on finance, investing, and business strategy.

Reference: [FIBO (Financial Industry Business Ontology)](https://spec.edmcouncil.org/fibo/), EDM Council. Canonical namespace: `https://spec.edmcouncil.org/fibo/`.

## Concepts

- **Competitive advantage:** `COMPETITIVE_ADVANTAGE`, `BARRIER_TO_ENTRY`, `RETURN_ON_CAPITAL`, `ECONOMIC_PROFIT`
- **Valuation:** valuation, intrinsic value, and related metrics
- **Capital allocation & risk:** capital allocation, risk, and economic-profit concepts
- `ALL_PREDICATES` — aggregated constant slice

## Usage

```rust
use hkask_bridge_fibo::{COMPETITIVE_ADVANTAGE, ECONOMIC_PROFIT, FiboConcept};

let pred: FiboConcept = ECONOMIC_PROFIT; // "fibo:economicProfit"
```

## Dependencies

None. Pure constant definitions.