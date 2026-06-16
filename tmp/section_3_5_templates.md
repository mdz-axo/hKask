### 3.5 Templates (`hkask-templates`)

**Motivating Principle:** P3 (Generative Space) — template registry, vocabulary, and execution substrate
**Crate:** `hkask-templates` | **Sources:** `src/*.rs`, `tests/*.rs`

**53 production contracts** + **25 test contracts**.

#### Production Contracts

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-T001 | `P3-tpl-capability-validator-new` | `new()` | [P3] Motivating: Generative Space — registration-time OCAP gate for template capabilities; [P4] Constraining: Clear Boundaries — validator establishes capability boundary |
| FR-T002 | `P3-tpl-validate-capabilities` | `validate_capabilities()` | [P3] Motivating: Generative Space — checks template capability requirements against held tokens; [P4] Constraining: Clear Boundaries — action hierarchy enforcement (Execute ≥ Write ≥ Read) |
| FR-T003 | `P3-tpl-contract-validator-new` | `new()` | [P3] Motivating: Generative Space — passthrough validator for unconstrained registration; [P4] Constraining: Clear Boundaries — default Warn mode allows registration |
| FR-T004 | `P3-tpl-contract-validator-with-lexicon` | `with_lexicon()` | [P3] Motivating: Generative Space — binds vocabulary to registration gate; [P8] Constraining: Semantic Grounding — hLexicon provides canonical term set |
| FR-T005 | `P3-tpl-contract-validator-with-mode` | `with_mode()` | [P3] Motivating: Generative Space — configures validation strictness |
| FR-T006 | `P3-tpl-contract-validator-validate-terms` | `validate_terms()` | [P3] Motivating: Generative Space — vocabulary consistency gate; [P8] Constraining: Semantic Grounding — unknown terms flagged against hLexicon |
| FR-T007 | `P3-tpl-manifest-executor-new` | `new()` | [P3] Motivating: Generative Space — executor for template manifest cascades; [P4] Constraining: Clear Boundaries — requires ACP secret for delegation |
| FR-T008 | `P3-tpl-load-hlexicon-yaml` | `load_hlexicon_from_yaml()` | [P3] Motivating: Generative Space — loads canonical workspace vocabulary; [P8] Constraining: Semantic Grounding — YAML vocabulary round-trips to HLexicon |
| FR-T009 | `P3-tpl-load-hlexicon-file` | `load_hlexicon_from_file()` | [P3] Motivating: Generative Space — loads vocabulary from filesystem path; [P8] Constraining: Semantic Grounding — file contents parsed into HLexicon |
| FR-T010 | `P3-tpl-load-hlexicon-default` | `load_hlexicon_default()` | [P3] Motivating: Generative Space — loads built-in default vocabulary; [P8] Constraining: Semantic Grounding — default terms seed the workspace lexicon |
| FR-T011 | `P3-tpl-parse-markdown-catalog` | `parse_markdown_catalog()` | [P3] Motivating: Generative Space — extracts terms from markdown catalog; [P8] Constraining: Semantic Grounding — markdown tables become structured terms |
| FR-T012 | `P3-tpl-render-workspace-yaml` | `render_workspace_yaml()` | [P3] Motivating: Generative Space — serializes vocabulary to workspace YAML; [P8] Constraining: Semantic Grounding — YAML output preserves term semantics |
| FR-T013 | `P3-tpl-regenerate-workspace-yaml` | `regenerate_workspace_yaml()` | [P3] Motivating: Generative Space — full markdown-to-YAML vocabulary pipeline; [P8] Constraining: Semantic Grounding — regenerated YAML matches canonical source |
| FR-T014 | `P3-tpl-resolve-manifest` | `resolve_manifest()` | [P3] Motivating: Generative Space — resolves template manifest references; [P8] Constraining: Semantic Grounding — manifest terms validated against hLexicon |
| FR-T015 | `P3-tpl-prompt-strategy-from-input` | `from_input()` | [P3] Motivating: Generative Space — constructs prompt strategy from user input |
| FR-T016 | `P3-tpl-prompt-strategy-frame` | `frame()` | [P3] Motivating: Generative Space — frames prompt for a strategy step |
| FR-T017 | `P3-tpl-prompt-strategy-name` | `name()` | [P3] Motivating: Generative Space — names the selected strategy |
| FR-T018 | `P3-tpl-registry-new` | `new()` | [P3] Motivating: Generative Space — in-memory template registry |
| FR-T019 | `P3-tpl-registry-set-lexicon` | `set_lexicon()` | [P3] Motivating: Generative Space — binds vocabulary to registry; [P8] Constraining: Semantic Grounding — hLexicon constrains registered terms |
| FR-T020 | `P3-tpl-registry-reload` | `reload()` | [P3] Motivating: Generative Space — refreshes registry from filesystem |
| FR-T021 | `P3-tpl-registry-validate-template-path` | `validate_template_path()` | [P3] Motivating: Generative Space — path safety for template discovery; [P4] Constraining: Clear Boundaries — rejects paths outside template root |
| FR-T022 | `P3-tpl-registry-register` | `register()` | [P3] Motivating: Generative Space — registers a template in the registry |
| FR-T023 | `P3-tpl-registry-get` | `get()` | [P3] Motivating: Generative Space — retrieves a registered template |
| FR-T024 | `P3-tpl-registry-count` | `count()` | [P3] Motivating: Generative Space — reports registry size |
| FR-T025 | `P3-tpl-registry-list-skills` | `list_skills()` | [P3] Motivating: Generative Space — lists registered skills |
| FR-T026 | `P3-tpl-registry-list-skills-by-visibility` | `list_skills_by_visibility()` | [P3] Motivating: Generative Space — visibility-filtered skill listing |
| FR-T027 | `P3-tpl-registry-remove-skill` | `remove_skill()` | [P3] Motivating: Generative Space — removes a skill from registry |
| FR-T028 | `P3-tpl-registry-register-skill` | `register_skill()` | [P3] Motivating: Generative Space — registers a skill with metadata |
| FR-T029 | `P3-tpl-registry-get-skill` | `get_skill()` | [P3] Motivating: Generative Space — retrieves skill metadata |
| FR-T030 | `P3-tpl-registry-skills-by-domain` | `skills_by_domain()` | [P3] Motivating: Generative Space — domain-filtered skill listing |
| FR-T031 | `P3-tpl-registry-skills-referencing-template` | `skills_referencing_template()` | [P3] Motivating: Generative Space — reverse skill lookup by template |
| FR-T032 | `P3-tpl-registry-register-bundle` | `register_bundle()` | [P3] Motivating: Generative Space — registers a skill bundle |
| FR-T033 | `P3-tpl-registry-get-bundle` | `get_bundle()` | [P3] Motivating: Generative Space — retrieves a skill bundle |
| FR-T034 | `P3-tpl-registry-list-bundles` | `list_bundles()` | [P3] Motivating: Generative Space — lists registered bundles |
| FR-T035 | `P3-tpl-registry-remove-bundle` | `remove_bundle()` | [P3] Motivating: Generative Space — removes a bundle |
| FR-T036 | `P3-tpl-registry-find-bundle-by-skills` | `find_bundle_by_skills()` | [P3] Motivating: Generative Space — finds bundle matching skill set |
| FR-T037 | `P3-tpl-registry-bootstrap` | `bootstrap()` | [P3] Motivating: Generative Space — seeds registry from workspace templates |
| FR-T038 | `P3-tpl-registry-sqlite-new` | `new()` | [P3] Motivating: Generative Space — SQLite-backed template registry |
| FR-T039 | `P3-tpl-registry-sqlite-new-with-conn` | `new_with_conn()` | [P3] Motivating: Generative Space — SQLite registry from existing connection |
| FR-T040 | `P3-tpl-registry-sqlite-set-lexicon` | `set_lexicon()` | [P3] Motivating: Generative Space — binds vocabulary to SQLite registry; [P8] Constraining: Semantic Grounding — hLexicon constrains persisted terms |
| FR-T041 | `P3-tpl-registry-sqlite-register` | `register()` | [P3] Motivating: Generative Space — persists template registration |
| FR-T042 | `P3-tpl-registry-sqlite-get-entry` | `get_entry()` | [P3] Motivating: Generative Space — retrieves persisted template entry |
| FR-T043 | `P3-tpl-registry-sqlite-delete-entry` | `delete_entry()` | [P3] Motivating: Generative Space — removes persisted template entry |
| FR-T044 | `P3-tpl-registry-sqlite-search-by-lexicon` | `search_by_lexicon()` | [P3] Motivating: Generative Space — vocabulary-aware template search; [P8] Constraining: Semantic Grounding — search uses hLexicon terms |
| FR-T045 | `P3-tpl-registry-sqlite-count` | `count()` | [P3] Motivating: Generative Space — reports persisted registry size |
| FR-T046 | `P3-tpl-registry-sqlite-get-skill-owned` | `get_skill_owned()` | [P3] Motivating: Generative Space — retrieves owned skill record |
| FR-T047 | `P3-tpl-registry-sqlite-list-skills-owned` | `list_skills_owned()` | [P3] Motivating: Generative Space — lists owned skill records |
| FR-T048 | `P3-tpl-registry-sqlite-skills-by-domain-owned` | `skills_by_domain_owned()` | [P3] Motivating: Generative Space — domain-filtered owned skill listing |
| FR-T049 | `P3-tpl-registry-sqlite-skills-referencing-template-owned` | `skills_referencing_template_owned()` | [P3] Motivating: Generative Space — reverse owned skill lookup |
| FR-T050 | `P3-tpl-skill-loader-new` | `new()` | [P3] Motivating: Generative Space — loader for skill registry entries |
| FR-T051 | `P3-tpl-skill-loader-load-into` | `load_into()` | [P3] Motivating: Generative Space — loads skill into registry |
| FR-T052 | `P3-tpl-skill-loader-infer-domain` | `infer_domain_from_registry()` | [P3] Motivating: Generative Space — infers skill domain from registry contents |
| FR-T053 | `P3-tpl-skill-loader-parse-front-matter` | `parse_front_matter()` | [P3] Motivating: Generative Space — parses skill front matter metadata |

#### Test Contracts

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-TT001 | `P3-tpl-test-empty-requirements-pass` | `empty_requirements_always_pass()` |
| FR-TT002 | `P3-tpl-test-satisfied-requirement-passes` | `satisfied_requirement_passes()` |
| FR-TT003 | `P3-tpl-test-unsatisfied-requirement-fails` | `unsatisfied_requirement_fails()` |
| FR-TT004 | `P3-tpl-test-execute-satisfies-read` | `execute_token_satisfies_read_requirement()` |
| FR-TT005 | `P3-tpl-test-write-satisfies-read` | `write_token_satisfies_read_requirement()` |
| FR-TT006 | `P3-tpl-test-read-not-satisfies-write` | `read_token_does_not_satisfy_write_requirement()` |
| FR-TT007 | `P3-tpl-test-malformed-requirement-error` | `malformed_requirement_returns_error()` |
| FR-TT008 | `P3-tpl-test-multiple-requirements` | `multiple_requirements_all_must_be_satisfied()` |
| FR-TT009 | `P3-tpl-test-no-held-tokens-fail` | `no_held_tokens_with_requirements_fails()` |
| FR-TT010 | `P3-tpl-test-contract-validator-passthrough` | `validator_without_lexicon_always_passes()` |
| FR-TT011 | `P3-tpl-test-contract-validator-warn-reports` | `validator_warn_mode_reports_unknown_terms()` |
| FR-TT012 | `P3-tpl-test-contract-validator-reject-blocks` | `validator_reject_mode_blocks_unknown_terms()` |
| FR-TT013 | `P3-tpl-test-contract-validator-accepts-known` | `validator_accepts_known_terms()` |
| FR-TT014 | `P3-tpl-test-contract-validator-default-passthrough` | `validator_default_is_passthrough()` |
| FR-TT015 | `P3-tpl-test-contract-validate-terms-never-panics` | `validator_never_panics()` |
| FR-TT016 | `P3-tpl-test-contract-known-terms-accepted` | `known_terms_always_accepted()` |
| FR-TT017 | `P3-tpl-test-parse-catalog-extracts-terms` | `parse_catalog_extracts_terms()` |
| FR-TT018 | `P3-tpl-test-parse-catalog-skips-non-terms` | `parse_catalog_skips_non_term_rows()` |
| FR-TT019 | `P3-tpl-test-parse-catalog-empty-error` | `parse_catalog_empty_input_returns_error()` |
| FR-TT020 | `P3-tpl-test-render-yaml-round-trips` | `render_yaml_round_trips()` |
| FR-TT021 | `P3-tpl-test-regenerate-yaml-pipeline` | `regenerate_workspace_yaml_produces_valid_yaml()` |
| FR-TT022 | `P3-tpl-test-hlexicon-yaml-matches-markdown` | `hlexicon_yaml_matches_markdown()` |
| FR-TT023 | `P3-tpl-test-yaml-parser-never-panics` | `yaml_parser_never_panics_on_arbitrary_bytes()` |
| FR-TT024 | `P3-tpl-test-template-rendering-correctness` | `all_templates_render()` |
| FR-TT025 | `P3-tpl-test-manifest-schema-validation` | `all_skill_manifests_are_well_formed()` |
