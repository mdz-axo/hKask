# Okapi Capability Model for Template Authors

**Version:** 1.0.0  
**Date:** 2026-05-20  
**Status:** MVP v0.21.0

---

## Overview

hKask templates can declare required Okapi capabilities in their YAML frontmatter. The capability system ensures templates are only registered when the underlying Okapi instance supports the required features.

---

## Capability Types

Okapi exposes the following capabilities via `/api/engine/status`:

| Capability | Type | Description |
|------------|------|-------------|
| `runner_type` | String | `"ollamarunner"` (full features) or `"llamarunner"` (basic only) |
| `token_probs` | Boolean | Token probability output for confidence routing |
| `grammar_native` | Boolean | GBNF grammar-constrained decoding |
| `lora_hot_swap` | Boolean | LoRA adapter hot-swapping |
| `advanced_sampling` | Boolean | Advanced samplers (mirostat, DRY, XTC) |

---

## Template Frontmatter Declaration

Templates declare required capabilities in YAML frontmatter:

```yaml
---
template_type: Prompt
domain: WordAct
requires_okapi:
  n_probs: 5          # Requires token_probs capability
  grammar: null       # Requires grammar_native capability (null = not required)
  adapter: null       # Requires lora_hot_swap capability (null = not required)
confidence:
  threshold: 0.75
  escalate_to_model: "qwen3:70b"
lexicon_terms:
  - classify
  - recognize
---

[inference]
Template content here...
```

---

## Validation Rules

### Prompt Templates

**Required:** `n_probs` for confidence-based routing

```yaml
---
template_type: Prompt
requires_okapi:
  n_probs: 5  # REQUIRED for Prompt templates
---
```

**Error if missing:**
```
Template type 'Prompt' requires 'n_probs' in requires_okapi, but it was not specified.
Add 'n_probs: 5' to enable token probability-based confidence routing.
```

### Process Templates

**Required:** `grammar` constraint for grammar-constrained decoding

```yaml
---
template_type: Process
requires_okapi:
  grammar: "path/to/constraint.gbnf"  # REQUIRED for Process templates
---
```

**Error if missing:**
```
Process template requires 'grammar' constraint in requires_okapi, but it was not specified.
Add 'grammar: "path/to/constraint.gbnf"' to enable grammar-constrained decoding.
```

---

## Capability Compatibility Errors

### Token Probabilities Not Supported

**When:** Template requires `n_probs` but Okapi's `token_probs` is `false`

**Error:**
```
Template requires 'n_probs' but Okapi's token_probs capability is disabled.
Okapi runner type: llamarunner.
Token probabilities are only available with ollamarunner.
```

**Resolution:** Use an `ollamarunner`-compatible model or remove `n_probs` requirement.

### Grammar Not Supported

**When:** Template requires `grammar` but Okapi's `grammar_native` is `false`

**Error:**
```
Template requires 'grammar' but Okapi's grammar_native capability is disabled.
Okapi runner type: llamarunner.
Grammar constraints are only available with ollamarunner.
```

**Resolution:** Use an `ollamarunner`-compatible model or remove `grammar` requirement.

### LoRA Adapter Not Supported

**When:** Template requires `adapter` but Okapi's `lora_hot_swap` is `false`

**Error:**
```
Template requires LoRA adapter 'my-adapter', but Okapi's lora_hot_swap capability is disabled.
Okapi runner type: llamarunner.
Use an ollamarunner-compatible model or remove the adapter requirement.
```

**Resolution:** Use an `ollamarunner`-compatible model or remove `adapter` requirement.

---

## Confidence Configuration

Confidence-based escalation requires `token_probs` capability:

```yaml
---
template_type: Prompt
requires_okapi:
  n_probs: 5  # Required for confidence calculation
confidence:
  threshold: 0.75  # Must be 0.0 - 1.0
  escalate_to_model: "qwen3:70b"
---
```

**Error if threshold invalid:**
```
Confidence threshold 1.5 is outside valid range [0.0, 1.0].
Use a value between 0.0 and 1.0 inclusive.
```

---

## hLexicon Validation

All `lexicon_terms` must be canonical hLexicon terms:

```yaml
---
lexicon_terms:
  - classify    # Valid
  - recognize   # Valid
  - unknown     # INVALID - not in hLexicon
---
```

**Error if unknown term:**
```
Invalid lexicon term 'unknown' - not found in hLexicon.
Available terms: ["classify", "discriminate", "route", "recognize"].
Use only canonical hLexicon terms to ensure consistent LLM interpretation.
```

---

## Capability-Based Security

hKask uses capability tokens to authorize Okapi operations:

### Default System Capability

Grants: `Generate`, `Chat`, `ReadMetrics`, `ReadCapabilities`

### Read-Only Capability

Grants: `ReadMetrics`, `ReadCapabilities`

### Template-Scoped Capability

Grants operations only for specific template execution.

---

## CNS Integration

Template validation emits CNS spans:

- `cns.prompt.validation` — Template registration validation
- `cns.prompt.escalation` — Confidence-based model escalation
- `cns.connector.llm.tokens` — Token throughput monitoring
- `cns.connector.llm.context` — Context utilization monitoring
- `cns.tool.adapter_swap` — LoRA adapter swap events

---

## Example: Valid Prompt Template

```yaml
---
template_type: Prompt
domain: WordAct
requires_okapi:
  n_probs: 5
confidence:
  threshold: 0.75
  escalate_to_model: "qwen3:70b"
lexicon_terms:
  - classify
  - recognize
contract:
  input:
    type: object
    properties:
      question: {type: string}
  output:
    type: object
    properties:
      answer: {type: string}
      confidence: {type: number}
---

[inference]
You are answering factual questions with confidence scores.
```

---

## Example: Valid Process Template

```yaml
---
template_type: Process
domain: FlowDef
requires_okapi:
  grammar: "constraints/json-schema.gbnf"
lexicon_terms:
  - route
  - discriminate
contract:
  input:
    type: object
    properties:
      raw_input: {type: string}
  output:
    type: object
    properties:
      structured_output: {type: object}
---

[process]
Step 1: Parse input
Step 2: Apply grammar constraint
Step 3: Return structured output
```

---

## Troubleshooting

### "Runner type: llamarunner" Errors

**Cause:** Model loaded with basic `llamarunner` instead of full-featured `ollamarunner`

**Solution:**
```bash
# Set environment variable before starting Okapi
export OKAPI_SIMPLE_ENGINE=1

# Restart Okapi
okapi serve
```

### Capability Fetch Errors

**Cause:** Okapi not running or unreachable

**Solution:**
```bash
# Check Okapi status
curl http://127.0.0.1:11435/api/engine/status

# Expected response includes:
# {
#   "capabilities": {
#     "runner_type": "ollamarunner",
#     "token_probs": true,
#     ...
#   }
# }
```

---

## Reference

- **Okapi Capabilities:** `/api/engine/status` endpoint
- **hLexicon:** `docs/architecture/hKask-hLexicon.md`
- **Template Types:** Prompt, Process, Cognition
- **CNS Spans:** `docs/architecture/hKask-architecture-master.md` (CNS section)

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
