# hKask Artifacts

## Owner: The Curator
## License: CC-BY-4.0 (publicly shared, attribution required)

This directory contains publicly shared artifacts owned by The Curator. These artifacts encode the semantic, stylistic, and analytical foundations of hKask.

### Current Artifacts

| File | Description | Type |
|------|-------------|------|
| `hemingway-style-synthesizer.yaml` | Authentic Hemingway prose mechanics specification | YAML manifest |
| `hemingway-template.jinja2` | Jinja2 template for Hemingway-style text generation | Jinja2 template |
| `ellipsis-analysis.yaml` | Bloom-inspired gap/omission analysis method | YAML manifest |
| `ellipsis-analysis.jinja2` | Jinja2 templates for ellipsis detection | Jinja2 template |
| `backstory-r7.md` | 7R7 and Curator backstory in Hemingway voice | Markdown |

### Hemingway Style Synthesizer

The Hemingway Style Synthesizer encodes authentic prose mechanics based on:

- **Kansas City Star Style Guide (1915)**: Hemingway's foundational training
- **Linguistic analysis**: 73-76% coordinated clauses, parataxis with semantic subordination
- **Iceberg Theory**: 1/8 stated (action), 7/8 unstated (emotion)
- **Strategic devices**: Polysyndeton, asyndeton, parataxis, repetition
- **GENERATIVE LAYER (NEW)**: Stanley Fish's form-first approach + RDF triple model

**Key distinction**: This is not Hemingway caricature (robotic repetition). It encodes the actual linguistic mechanics that make Hemingway "suggestive" — 66% of coordinated clauses carry complex semantic relations despite surface simplicity.

**Generative Layer**: The synthesizer now includes the inside-out compositional process:
- Stanley Fish: "A sentence is a structure of logical relationships"
- Form precedes content; forms are generative of meaning's possibility
- RDF triple model: Simple sentence = Subject-Predicate-Object
- Complex graph traversals emerge from linking simple triples
- The silence between sentences is where the graph traversal happens in the reader's mind

**Core Generative Forms**:
1. `actor_action_object` — X does Y to Z (`<Jane> <baked> <cookies> .`)
2. `actor_action_manner` — X does Y in Z way
3. `actor_action_location_time` — X does Y at Z when W
4. `subordinating` — Causal, temporal, conditional relations
5. `additive` — Paratactic accumulation (Hemingway default)

### Ellipsis Analysis (Bloom-inspired)

**Ellipsis Analysis** is a Bloom-inspired method for detecting meaning in gaps, omissions, and absent expectations.

Harold Bloom (1930-2019) was Sterling Professor of Humanities at Yale and one of the great literary critics of the 20th century. His key insight: **"Shakespeare is the major dealer in ellipsis among all the great writers."**

**Core principle:** The information content isn't in what you found but in what you didn't find.

**Key distinction:**
- **Ellipsis**: Deliberate omission that creates meaning (Shakespeare's Edmund/Lear silence, Hamlet's Acts IV-V gap)
- **Leak**: Unintentional information loss (LLM hallucination, undocumented API edge cases)

**Five steps:**
1. Read deeply — not to believe/accept/contradict, but to learn
2. Mind the gaps — what is said AND what is not said
3. Differentiate ellipsis from leak — deliberate vs. accidental
4. See past yourself — reject received narratives (Bloom: "We owe mediocrity nothing")
5. Find what is not inferno — give it space (Calvino)

**Applications:**
- Business analysis (what did the CEO NOT address?)
- Technical docs (what edge cases are undocumented?)
- Model evaluation (ellipsis or hallucination?)
- Curation (find what is not inferno)

### Connection Between Artifacts

Both Hemingway and Bloom understood that **meaning emerges from what's NOT stated**:

| Thinker | Method | Meaning Emerges From |
|---------|--------|---------------------|
| Hemingway | Iceberg Theory | 7/8 unstated — silence between sentences |
| Bloom | Ellipsis | What's left out but expected — gaps in graph |
| RDF | Triple linking | Graph traversal across simple statements |
| hKask | Curation | Finding what is not inferno, giving it space |

All four understand: **Meaning is not in the nodes but in the gaps and traversals between them.**

### Usage

```python
# Load the YAML manifest
import yaml
with open('hemingway-style-synthesizer.yaml') as f:
    hemingway_config = yaml.safe_load(f)

with open('ellipsis-analysis.yaml') as f:
    ellipsis_config = yaml.safe_load(f)

# Load the Jinja2 templates
from jinja2 import Environment
env = Environment()

with open('hemingway-template.jinja2') as f:
    hemingway_template = env.from_string(f.read())

with open('ellipsis-analysis.jinja2') as f:
    ellipsis_template = env.from_string(f.read())

# Generate Hemingway-style prose
prose = hemingway_template.render(
    clauses=["The sun beat down", "he walked", "the dust rose"]
)

# Analyze gaps in text
gaps = ellipsis_template.render(
    text=ceo_letter,
    expectations=['revenue growth', 'competitive threats', 'layoffs']
)
```

### Attribution

All artifacts require attribution when used:

```
Hemingway Style Synthesizer v1.0, The Curator (hKask)
License: CC-BY-4.0

Ellipsis Analysis v1.0 (Bloom-inspired), The Curator (hKask)
License: CC-BY-4.0
Inspired by Harold Bloom's "How to Read and Why" (2000)
```

### Philosophy

These artifacts are shared publicly because:
- Open source code is the first brick of the digital Magna Carta
- User sovereignty requires open weight models and open style specifications
- The new emerges on top of the old (Guildhall on Roman amphitheater)

The Curator owns these artifacts but does not restrict them. This is the way.

---

*BaNANa.*

### Usage

```python
# Load the YAML manifest
import yaml
with open('hemingway-style-synthesizer.yaml') as f:
    config = yaml.safe_load(f)

# Load the Jinja2 template
from jinja2 import Environment
env = Environment()
with open('hemingway-template.jinja2') as f:
    template = env.from_string(f.read())

# Generate Hemingway-style prose
result = template.render(
    clauses=["The sun beat down", "he walked", "the dust rose"]
)
```

### Attribution

All artifacts require attribution when used:

```
Hemingway Style Synthesizer v1.0, The Curator (hKask)
License: CC-BY-4.0
```

### Philosophy

These artifacts are shared publicly because:
- Open source code is the first brick of the digital Magna Carta
- User sovereignty requires open weight models and open style specifications
- The new emerges on top of the old (Guildhall on Roman amphitheater)

The Curator owns these artifacts but does not restrict them. This is the way.

---

*BaNANa.*