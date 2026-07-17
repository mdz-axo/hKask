# hKask Security Regression Library

Every confirmed security finding becomes a permanent, CI-enforced regression test.
This is a **human-curated checklist** — the `kali-audit` skill (Track C) proposes
entries, but humans review and merge them. The "evolving" property comes from the
library growing over time, not from autonomous learning.

## Format

Each regression is a YAML file named `RR-NNNN.yaml` (zero-padded, monotonically
incrementing). The schema:

```yaml
id: RR-0001                          # matches filename
title: "short description"
surface: code | template | supply-chain | mcp | config   # which kali-audit surface
cwe: CWE-XXX                         # MITRE CWE classification (if applicable)
owasp_llm: LLMXX                     # OWASP LLM Top 10 risk (if applicable)
discovered_in: path/to/file          # where the bug was found
discovered_by: kali-audit | manual   # who found it
discovered_at: YYYY-MM-DD
severity: critical | high | medium | low
detection:
  kind: grep | cargo-test | skill-probe
  pattern: "regex or test name"      # for grep: regex; for cargo-test: test path
  include: "glob pattern"            # for grep: file scope
mitigation: "what the fix looks like"
ci_gate: scripts/check-kali-regressions.sh  # the script that enforces it
status: pending | enforced           # pending = known bug, not yet fixed; enforced = fixed, CI catches re-introduction
```

## Status lifecycle

1. **pending** — bug found, not yet fixed. The regression is recorded but the CI
   gate does not fail (ratcheted). This prevents blocking the build while the fix
   is in progress.
2. **enforced** — bug fixed. The CI gate now fails if the pattern re-appears.
   Flip the status after the fix lands.

## CI integration

`scripts/check-kali-regressions.sh` runs all `grep`-kind regressions with
`status: enforced`. Ratcheted: `pending` regressions are warnings, not failures.

## Relationship to kali-audit skill

The `kali-audit` skill (Track C) consumes this library as input — it knows what
has already been found and can focus on new issues. When the skill finds a new
issue, it proposes a regression entry (in its report output). Humans review,
merge, and the library grows.
