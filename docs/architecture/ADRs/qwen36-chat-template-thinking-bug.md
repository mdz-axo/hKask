---
title: "ADR: Qwen3.6 Chat Template — enable_thinking Default Bug"
audience: [developers, ML engineers]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Training"
mds_categories: [domain, lifecycle]
---

# ADR: Qwen3.6 Chat Template — enable_thinking Default Bug

**Date:** 2026-07-10 | **Severity:** Critical | **Status:** Fixed

## Problem

Qwen3.6's Jinja2 `chat_template` defaults `enable_thinking=True`. During Unsloth training, `tokenizer.apply_chat_template()` applies this template to every training example. Since our Company Researcher QAs have no `reasoning_content`, the template generates empty `<think>` / `</think>` blocks around every assistant response.

**Impact:** The model learns to output empty thinking tags before answers. During inference, it will generate `<think>\n\n</think>\n\n` before every response — wasting tokens and potentially degrading answer quality.

**Evidence:** Qwen3 docs state: "When explicitly setting `enable_thinking=True` or leaving it as the default value in `tokenizer.apply_chat_template`, the model will engage its thinking mode." The default is `True`.

## Decision

Explicitly set `enable_thinking=False` when applying the chat template during training.

## Implementation

In pipeline YAML training config:
```yaml
chat_template_kwargs: {"enable_thinking": false, "add_generation_prompt": false}
```

In Unsloth training script:
```python
tokenizer.apply_chat_template(
    messages,
    enable_thinking=False,
    add_generation_prompt=False,
    tokenize=False
)
```

## Consequences

- **Positive:** Training data correctly represents non-thinking Company Researcher responses
- **Positive:** No wasted tokens on empty thinking blocks during inference
- **Negative:** Model loses ability to use thinking mode for this domain (acceptable — Company Researcher needs direct answers)

## References

- Qwen3 HuggingFace docs: "thinking capabilities enabled by default"
- `tokenizer_config.json`: Jinja2 template with `enable_thinking` parameter
- Reddit: Known template bugs in Qwen 3.5/3.6 with thinking/pre-thinking interaction
- `froggeric/Qwen-Fixed-Chat-Templates`: Community fix for template bugs (we use Unsloth's version)
