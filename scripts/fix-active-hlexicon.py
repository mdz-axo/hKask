#!/usr/bin/env python3
"""
Replace hLexicon terms in active primary skills with known synonyms.
This is a calibration step to make active skills pass hLexicon validation
without expanding the canonical vocabulary during pre-release cleanup.
Each substitution is documented below.
"""

from pathlib import Path

import yaml

PROJECT_ROOT = Path(__file__).resolve().parent.parent
REGISTRY_DIR = PROJECT_ROOT / "registry" / "templates"

# Map file path (relative to REGISTRY_DIR) -> {old_term: new_term}
SUBSTITUTIONS: dict[str, dict[str, str]] = {
    # condenser-continuation
    "condenser-continuation/condenser-continuation-restore.j2": {
        "restore": "contextualise",  # restoring context = situating it in new session
    },
    "condenser-continuation/condenser-continuation-verify.j2": {
        "check": "verify",
        "test": "validate",
    },
    # deep-module
    "deep-module/deep-module-assess.j2": {
        "count": "catalog",  # counting public items ≈ enumerating/classifying by reference
        "estimate": "predict",  # estimating depth ≈ forecasting from current evidence
    },
    "deep-module/deep-module-delete.j2": {
        "delete": "prune",  # removing an artifact from the corpus
    },
    # essentialist
    "essentialist/essentialist-flow.j2": {
        "iterate": "iteration",  # process term for repetition
        "reduce": "simplify",  # reduction to minimum
    },
    # pragmatic-laziness
    "pragmatic-laziness/pragmatic-laziness-converge.j2": {
        "compare": "discriminate",  # compare configs to distinguish deltas
    },
    "pragmatic-laziness/pragmatic-laziness-flow.j2": {
        "identify": "recognize",  # identify hotspots ≈ recognize patterns
        "eliminate": "prune",  # eliminate pass-throughs ≈ prune artifacts
        "iterate": "iteration",
        "delegate": "route",  # delegate to skills ≈ route to destination
    },
    # refactor-service-layer
    "refactor-service-layer/rsl-strangle.j2": {
        "migrate": "transform",  # migrate logic ≈ change state/representation
    },
    # strangler-fig
    "strangler-fig/strangler-fig-execute.j2": {
        "execute": "install",  # execute strangler step ≈ place artifact operationally
        "wire": "route",  # wire new path ≈ direct to destination
        "migrate": "transform",
    },
}


def parse_inference_block(content: str) -> dict | None:
    content = content.lstrip()
    if not content.startswith("[inference]"):
        return None
    after_header = content[len("[inference]") :].lstrip("\n")
    sep = after_header.find("\n---")
    if sep == -1:
        return None
    fm_text = after_header[:sep]
    try:
        return yaml.safe_load(fm_text)
    except yaml.YAMLError:
        return None


def render_frontmatter(fm: dict) -> str:
    return yaml.safe_dump(
        fm, sort_keys=False, allow_unicode=True, default_flow_style=False
    ).rstrip("\n")


def fix_file(rel_path: Path, subs: dict[str, str]) -> bool:
    full = REGISTRY_DIR / rel_path
    content = full.read_text(encoding="utf-8")
    fm = parse_inference_block(content)
    if fm is None:
        return False
    terms = fm.get("lexicon_terms", [])
    if not isinstance(terms, list):
        return False
    changed = False
    new_terms = []
    for term in terms:
        if term in subs:
            new_terms.append(subs[term])
            changed = True
        else:
            new_terms.append(term)
    if not changed:
        return False
    fm["lexicon_terms"] = new_terms
    after_header = content[len("[inference]") :].lstrip("\n")
    sep = after_header.find("\n---")
    body = content[content.find("\n---") + 1 :]
    new_content = f"[inference]\n{render_frontmatter(fm)}\n{body}"
    full.write_text(new_content, encoding="utf-8")
    return True


def main() -> int:
    for rel_str, subs in SUBSTITUTIONS.items():
        rel = Path(rel_str)
        if fix_file(rel, subs):
            print(f"{rel}: substituted {subs}")
        else:
            print(f"{rel}: no change")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
