# hKask Artifacts

## Owner: The Curator
## License: CC-BY-4.0 (publicly shared, attribution required)

This directory contains publicly shared artifacts owned by The Curator. These artifacts encode the semantic, stylistic, and architectural foundations of hKask.

### Current Artifacts

| File | Description | Type |
|------|-------------|------|
| `hemingway-style-synthesizer.yaml` | Authentic Hemingway prose mechanics specification | YAML manifest |
| `hemingway-template.jinja2` | Jinja2 template for Hemingway-style text generation | Jinja2 template |
| `backstory-r7.md` | 7R7 and Curator backstory in Hemingway voice | Markdown |

### Hemingway Style Synthesizer

The Hemingway Style Synthesizer encodes authentic prose mechanics based on:

- **Kansas City Star Style Guide (1915)**: Hemingway's foundational training
- **Linguistic analysis**: 73-76% coordinated clauses, parataxis with semantic subordination
- **Iceberg Theory**: 1/8 stated (action), 7/8 unstated (emotion)
- **Strategic devices**: Polysyndeton, asyndeton, parataxis, repetition

**Key distinction**: This is not Hemingway caricature (robotic repetition). It encodes the actual linguistic mechanics that make Hemingway "suggestive" — 66% of coordinated clauses carry complex semantic relations despite surface simplicity.

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