---
name: skill-discovery
visibility: public
description: "Find, evaluate, and install skills for hKask. Registry crate (manifest.yaml + *.j2) is the canonical source of truth. Detect capability gaps in the skill corpus, search for candidate registry crates, validate format and quality, and guide installation. Full lifecycle from gap to verified skill.
"
---

# Skill Discovery

Find, evaluate, and install skills for hKask. Registry crate (manifest.yaml + *.j2) is the canonical source of truth. Detect capability gaps in the skill corpus, search for candidate registry crates, validate format and quality, and guide installation. Full lifecycle from gap to verified skill.


## When to Use

- Detect capability gaps in the registry corpus by comparing task patterns an agent encounters against existing registry crates.
- Evaluate a candidate registry crate against format, quality, and safety criteria to determine whether it should be installed, revised, or rejected.
- Compute a normalized convergence metric for discovery iterations to assess whether an identified capability gap is sufficiently resolved.

## Instructions

### skill-discovery-detect-gap

1. Map every task pattern to determine if an existing skill covers it fully, partially, or not at all.
2. Classify partial matches as Feature gaps rather than Coverage gaps.
3. Detect latent gaps where a quality or governance rule exists in `docs/architecture/PRINCIPLES.md` but no skill enforces it, classifying these as Governance gaps.
4. Score the impact of each gap on agent effectiveness as `critical`, `high`, `medium`, or `low`.
5. Prioritize gaps by impact, then by frequency of the associated task pattern.
6. Respond with a JSON object containing the `gap_list` and `priority_ranking`.
7. Evaluate every task pattern without skipping niche patterns.
8. Ensure each gap references exactly one category, even if it spans multiple task patterns.
9. Justify impact scoring using task pattern frequency and consequence.
10. Do not recommend `ignore` for any gap with `critical` or `high` impact.

### skill-discovery-evaluate

1. Evaluate the candidate skill against format, quality, and safety criteria.
2. Validate the format by checking for YAML frontmatter, a valid name, a specific description, and the absence of deprecated markers.
3. Assess instruction quality to ensure steps are imperative, concrete, actionable, bounded in scope, and have clear trigger conditions.
4. Check Magna Carta compliance and system constraints, including user sovereignty (P1), affirmative consent (P2), generative space (P3), clear boundaries (P4), headless compliance, CNS span validity, and crate path validity.
5. Score each check from 0 to 2, where 0 is fail, 1 is partial, and 2 is pass.
6. Respond with a JSON object containing `format_validation`, `quality_evaluation`, `safety_evaluation`, `overall_score`, and `recommendation`.
7. Score every check without omitting any.
8. Reject the skill if any safety check scores 0.
9. Revise the skill if the overall score is less than 16 but there are no safety failures.

### skill-discovery-convergence-check

1. Compute a normalized `convergence_metric` in the range [0,1], where 0 means the gap is sufficiently resolved and 1 means it is unresolved.
2. Start scoring at 1.0 and adjust downward based on candidate evaluation results.
3. Set the metric to 0.1 or lower if the recommendation is to install and safety checks pass.
4. Set the metric between 0.2 and 0.6 if the recommendation is to revise but the candidate is close.
5. Set the metric to 0.7 or higher if the recommendation is to reject and no fallback exists.
6. Clamp the final metric to the [0,1] range.
7. Return a JSON object containing the `convergence_metric`, `convergence_method`, `rationale`, `blockers`, and `remaining_gap`.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `skill-discovery-detect-gap.j2` | KnowAct | Detect capability gaps in the registry corpus. Analyze task patterns against existing registry crate descriptions and template_type coverage. Classify gaps (coverage, feature, automation, governance) and prioritize by impact.  |
| `skill-discovery-evaluate.j2` | KnowAct | Evaluate a candidate registry crate against format, quality, and safety criteria. Check manifest structure, .j2 frontmatter validity, Magna Carta compliance, and CNS span validity. Produce scored recommendation.  |
| `skill-discovery-convergence-check.j2` | KnowAct | Compute a normalized convergence metric for discovery iterations. Synthesizes gap detection + candidate evaluation into `convergence_metric` in [0,1], where 0 means the capability gap is sufficiently resolved.  |

## Constraints

- `skill-discovery-detect-gap.j2`: Public.
- `skill-discovery-evaluate.j2`: Public.
- `skill-discovery-convergence-check.j2`: Public.
- Registry is authoritative — when this SKILL.md disagrees with registry templates, the registry wins.
