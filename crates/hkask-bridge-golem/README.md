# hkask-bridge-golem

GOLEM narrative/literary ontology bridge — canonical predicate URIs for narrative concepts.

Part of the dual-axis ontological framework (P5.4). Thin mapping layer: canonical URI constants, no dependencies, no reasoners, no overhead. Mirrors `hkask-bridge-dublincore` and `hkask-bridge-pko`.

Used by `docproc extract_triples` for narrative passages (prose, fiction, memoir, biography, narrative nonfiction).

## Concepts

- **Characters and agents:** `HAS_CHARACTER`, `HAS_NARRATOR`, `HAS_PERSPECTIVE`
- **Plot and structure:** `HAS_EVENT`, `HAS_PLOT`, conflict/tension concepts
- **Themes and devices:** themes, literary devices, interpretive relationships
- `ALL_PREDICATES` — aggregated constant slice

## Usage

```rust
use hkask_bridge_golem::{HAS_CHARACTER, HAS_EVENT, GolemConcept};

let pred: GolemConcept = HAS_EVENT; // "golem:hasEvent"
```

## Dependencies

None. Pure constant definitions.