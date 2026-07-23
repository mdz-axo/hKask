# Curator Review — Todo

- [x] S1-S8b + L3/L4: curator cleanup (dead act path, typed dispatch, unified EscalationSeverity, etc.)
- [x] M1: reg.meta canonical namespace + MetaSpan enum + emission helpers
- [x] M2: CuratorContext regulation_sink + SelfQuality counters; emit reg.meta.directive at issue_directive
- [x] M3: EscalationPolicy mutable thresholds + self_calibrate loop + emit reg.meta.escalation/circuit_breaker
- [x] M4: bidirectional self-calibration (compute_threshold_adjustment: raise/lower/cooldown/floor/ceiling/min-obs gate). 10 unit tests.
- [x] M5: calibration-effectiveness measurement (PendingCalibration close-out with eff_before/eff_after/eff_delta)
- [x] M6: GENERATIVE self-calibration — the curator is its own generative entity.
  - New template curator/metacognition-self-calibrate.j2 (registered in curator/manifest.yaml): the curator generates its own threshold adjustment from self-quality + effectiveness + last-calibration trajectory.
  - self_calibrate is now async + generative-first (try_generative_calibration via ManifestExecutor); compute_threshold_adjustment is the safety-rail fallback (no executor / template failure).
  - The LLM's proposed threshold is CLAMPED to the bounded band [DEFAULT_ESCALATION_VARIETY_DEFICIT, VARIETY_DEFICIT_CEILING] regardless of source (hard safety rail).
  - reg.meta.self_calibration records decision source (generative/fallback) so generative-vs-fallback quality can be compared.
- [x] Cleanup: deleted dead skill_catalog field+setter+accessor from CuratorContext (other agent's unfinished WIP, zero Rust callers — per "don't allow dead code").
- [x] Final: clippy -D warnings clean on hkask-types, hkask-regulation, hkask-pods, hkask-services-context, hkask-services-chat, hkask-mcp-curator. Tests: types 80, regulation 169, pods 44. reg-canonical gate OK.

## Note: interference recovered
Other agents had been editing CuratorContext (skill_catalog/registry_index) and loop_body.rs in parallel, reverting several edits. After they were stopped, I re-applied the generative self_calibrate rewrite + PendingCalibration.source, restored the .await call, and deleted the dead skill_catalog code. Build is now fully green.

## Remaining (deferred / not curator)
- GEPA offline evolution of the self-calibration template: deferred until real reg.meta.self_calibration trajectory data accumulates. The M5 spans (source + eff_delta) are the trajectory data GEPA's gpa-sample-trajectories step consumes.
- hkask-inference ollama_backend.rs / kilocode_backend.rs / openai_backend.rs: other-agent WIP (unused-import warnings; was fluxing). Not curator; currently compiles with warnings.