# hkask-bridge-eso

Epistemic Science Ontology (ESO) bridge — canonical predicate URIs for epistemic and scientific reasoning concepts.

Part of the dual-axis ontological framework (P5.4). Thin mapping layer: canonical URI constants, no dependencies, no reasoners, no overhead. Mirrors `hkask-bridge-dublincore` and `hkask-bridge-pko`.

Used by `docproc extract_triples` for expository passages on science, systems thinking, forecasting, complexity, and research methodology. Relevant corpus content: Tetlock (superforecasting), Meadows (systems), Miller (complex adaptive systems), Popper (falsifiability), Ousterhout (software design), Marletto (can and can't), Kauffman (complexity).

## Concepts

- **Epistemic entities:** `HAS_HYPOTHESIS`, `HAS_THEORY`, `HAS_MODEL`, `HAS_CLAIM`, `HAS_ASSUMPTION`, `HAS_EVIDENCE`, `HAS_LIMITATION`
- **Inferential relations:** `IMPLIES`, `CONTRADICTS`, `FALSIFIED_BY`, `CORROBORATED_BY`, `GENERALIZES_TO`, `HAS_COUNTERARGUMENT`
- **Epistemic qualities:** `HAS_UNCERTAINTY`, `HAS_CONFIDENCE`, `USES_METHOD`
- `ALL_PREDICATES` — aggregated constant slice

## Usage

```rust
use hkask_bridge_eso::{HAS_HYPOTHESIS, FALSIFIED_BY, EsoConcept};

let pred: EsoConcept = FALSIFIED_BY; // "eso:falsifiedBy"
```

## Dependencies

None. Pure constant definitions.