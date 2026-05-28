---
title: "Template Header Standard — hLexicon Functional Roles"
audience: [developers, template authors]
last_updated: 2026-05-24
version: "0.21.0"
status: "Active"
domain: "Application"
ddmvss_categories: [composition]
---

# Template Header Standard — hLexicon Functional Roles

standard:
  # Required Header Format (Jinja2 templates)
  jinja2_header:
    format: |
      {# Template: {path_from_templates_dir} #}
      {# Functional Role: {wordact|flowdef|knowact} (description) #}
      {# Implementation: Jinja2 prompt #}
      {# Produces: {output_artifacts} #}
      {# {template_title} #}
    
    example_wordact: |
      {# Template: kata/habit-intervention.j2 #}
      {# Functional Role: WordAct (action-word — message generation) #}
      {# Implementation: Jinja2 prompt #}
      {# Produces: intervention_message #}
      {# Generate intervention for habit support #}
    
    example_flowdef: |
      {# Template: kata/improvement-cycle.j2 #}
      {# Functional Role: FlowDef (flow-definition — 4-step process guide) #}
      {# Implementation: Jinja2 prompt #}
      {# Steps: direction, current, target, experiment #}
      {# Improvement Kata: 4-step scientific capability development #}
    
    example_knowact: |
      {# Template: kata/consent-and-select.j2 #}
      {# Functional Role: KnowAct (knowledge-action — judgment/decision) #}
      {# Implementation: Jinja2 prompt #}
      {# Produces: consent_decision, kata_selection #}
      {# Verify consent and select Kata pattern #}

  # Required Header Format (YAML manifests)
  yaml_manifest_header:
    format: |
      # {Manifest Name}
      # ℏKask {version} — {description}
      # Functional Role: flowdef (process orchestration)
      # Implementation: YAML manifest
      # Steps: {step_list}
    
    example: |
      # Kata Pattern Manifest
      # ℏKask v0.21.4 — Unified Kata execution with iteration support
      # Functional Role: FlowDef (process orchestration)
      # Implementation: YAML manifest
      # Steps: consent-select, kata-cycle, outcome-habit, memory-record, cns-emit, ...

  # Required Header Format (YAML ports)
  yaml_port_header:
    format: |
      # {Port Group Name}
      # ℏKask {version} — {description}
      # Functional Role: {wordact|knowact} (based on port type)
      # Implementation: YAML port specification
      # Inbound Ports: {list}
      # Outbound Ports: {list}

  # Functional Role Quick Reference
  functional_roles:
    wordact:
      keyword: "action-word"
      test: "Does it produce an output artifact (message, span, triple)?"
      examples:
        - habit-intervention.j2 (generate intervention message)
        - cns:emit:kata port (emit CNS span)
        - memory:record:kata port (record memory triple)
    
    flowdef:
      keyword: "flow-definition"
      test: "Does it define a sequence of steps or stages?"
      examples:
        - improvement-cycle.j2 (4-step Kata process)
        - kata-pattern.yaml (5+3 step execution flow)
        - coaching-cycle.j2 (5-question dialogue flow)
    
    knowact:
      keyword: "knowledge-action"
      test: "Does it produce a decision, judgment, or assessment?"
      examples:
        - consent-and-select.j2 (verify consent, select pattern)
        - iteration-comparison.j2 (judge variance, confidence)
        - kata:execute port (authorization judgment)

  # Governance Process
  governance:
    new_template:
      - Add standard header with functional_role
      - Update subsystem hLexicon registry
      - Run validation script
      - Verify 100% compliance
    
    quarterly_review:
      - Run validation script
      - Review functional distribution
      - Identify and fix missing headers
      - Update workspace hLexicon registry
    
    compliance_target: 100%
    current_compliance: 43%  # After Kata templates updated: ~50%

---

## References

[^jinja2]: Ronacher, A. (2026). *Jinja2 Documentation*. <https://jinja.palletsprojects.com/>.

[^minijinja]: Mitsuhiko. (2026). *minijinja: Jinja2-compatible template engine for Rust*. <https://github.com/mitsuhiko/minijinja>.

[^hKask-hLexicon]: hKask Project. (2026). *hKask-hLexicon.md*. `/home/mdz-axolotl/Clones/hKask/docs/architecture/hKask-hLexicon.md`. Functional role definitions.

[^hKask-templates]: hKask Project. (2026). *crates/hkask-templates/src/registry_sqlite.rs*. Template registry implementation.

[^austin]: Austin, J. L. (1962). *How to Do Things with Words*. Harvard University Press. Speech act theory (WordAct, KnowAct foundations).

[^searle]: Searle, J. R. (1969). *Speech Acts: An Essay in the Philosophy of Language*. Cambridge University Press.

[^van-der-aalst]: van der Aalst, W. M. P. (2016). *Process Mining: Data Science in Action*. Springer. FlowDef process patterns.