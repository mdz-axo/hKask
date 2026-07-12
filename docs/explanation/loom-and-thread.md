---
title: "The Loom and the Thread — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# The Loom and the Thread

## A Design Philosophy in Two Words

hKask's README opens with a design philosophy: "Austere and efficient recombinatorial system. Rust is the loom (fixed logic). YAML/Jinja2 is the thread (mutable content)." This is not a metaphor for poetic effect — it is the structural premise of the entire system.

The loom-and-thread separation resolves a fundamental tension in agent platforms: behavior must be both stable enough to trust and flexible enough to evolve. A pure-code system locks behavior at compile time, making it rigid. A pure-configuration system puts behavior in mutable files, making it brittle and unverifiable. hKask splits the difference: the loom constrains what the thread can express, and the thread gives the loom something to weave.

## The Loom: Rust as Invariant Logic

The loom is everything compiled. It is the `kask` binary — 40 core crates, 15 MCP servers, ~192,700 lines of Rust. It is:

- **The CNS** (`hkask-cns`). The cybernetic loop: sense, compare, compute, act, verify. This loop does not change based on configuration. It is structural — a `Loop` trait with fixed semantics, `LoopAction` types with fixed authority hierarchy.

- **The Energy Layer** (`GasBudget`, `Well`, `WalletManager`). The hold-settle pattern, stale reservation detection, hard limits, invariance enforcement (`remaining + reserved ≤ cap`). These invariants are compile-time guarantees via private fields and constructor assertions.

- **The Database Driver** (`DatabaseDriver` trait, `SqliteDriver`, `PostgresDriver`). The abstraction is fixed — stores code against `&dyn DatabaseDriver`, not raw connections. New providers can be added, but the interface is invariant.

- **The GovernedTool Membrane** (`GovernedTool<P: ToolPort>`). Every tool invocation passes through this single membrane: OCAP check → gas reserve → tool execute → gas settle → span emit. The sequence is structural. Configuration cannot reorder it.

- **The Template Engine** (`ManifestExecutor`). It interprets YAML manifests, but the interpretation itself is Rust. The executor walks steps, evaluates conditions, checks convergence, enforces gas. The interpreter is the loom — it cannot be changed by a manifest.

The loom is not configurable. It is code. It is compiled, tested, fuzzed, verified by CI (format → clippy → build → test → doc → invariants). It is the safety boundary.

## The Thread: YAML/Jinja2 as Variant Content

The thread is everything authored. It lives in files that are read at runtime, not compiled in. It is:

- **Skill Manifests** (`registry/templates/*/manifest.yaml`, 64 files). These declare the structure of a skill: its steps, convergence criteria, gas budget, error handling. They do not contain logic — they contain declarative configuration that the Rust executor interprets.

- **Jinja2 Templates** (`registry/templates/*/*.j2`, 273 files). These are the prompts, the tool invocations, the output schemas. They are raw material for the skill execution engine. A template runs once; the skill wraps it in a PDCA loop.

- **SKILL.md Files** (`.agents/skills/*/SKILL.md`, 39 files). These are documentation — the human-facing description of what a skill does. The YAML front matter declares metadata (name, namespace, visibility); the Markdown body is the explanation. The `SkillLoader` parses these at startup.

- **Agent Definitions**. Pod configurations, agent WebIDs, skill assignments, capability grants. These declare what exists, not how it works.

The thread is mutable. An author can create a new skill by writing a `manifest.yaml` and a few `.j2` templates — no recompilation, no redeployment. An existing skill can be tuned: tighten the convergence threshold, increase the gas cap, add a pre-condition to a step. The thread evolves without touching the loom.

## Why This Separation Enables Safe Composition

The key property is that the loom constrains what the thread can express. A YAML manifest cannot:

- Create new action types. The executor only understands `render`, `tool_invoke`, `choice`, `populate`, `select`, `abort`, `escalate`. A manifest that declares `action: "rm -rf /"` is a parse error.

- Bypass gas enforcement. `gas.cap`, `gas.cost_per_iteration`, `gas.hard_limit` are fields the executor reads and enforces. A manifest cannot declare `"gas_bypass": true`.

- Violate convergence invariants. The executor enforces `min_iterations`, `max_iterations`, and `improvement_gate`. A manifest can configure these values but cannot override the check logic.

- Access the file system, network, or cryptographic keys. Templates invoke tools through the MCP protocol; tools are registered Rust implementations behind the `GovernedTool` membrane. A template cannot call `std::fs::remove_dir_all`.

The thread is powerful within its domain — it can compose skills, define workflows, set quality thresholds, tune iteration parameters — but it cannot escape the loom's constraints. This is the same security model as a web browser: JavaScript (thread) can manipulate the DOM, but it cannot access the file system. The browser (loom) provides a sandbox.

## Concrete Examples

### Skill Manifest → Template Engine

A skill manifest at `registry/templates/diagnose/manifest.yaml` declares:

```yaml
manifest:
  id: diagnose
  version: "0.31.0"
steps:
  - ordinal: 1
    action: render
    template_ref: plan.j2
  - ordinal: 2
    action: tool_invoke
    mcp: condenser
  - ordinal: 3
    action: render
    template_ref: evaluate.j2
convergence:
  threshold: 0.15
  max_iterations: 5
  convergence_field: composite
gas:
  cap: 5000
  cost_per_iteration: 1000
```

The Rust `ManifestExecutor` reads this, constructs a `BundleManifest`, and drives the PDCA loop. The YAML describes WHAT to do (render this template, invoke that tool, check convergence at this threshold). The Rust enforces HOW it's done (step ordering, gas tracking, timeout enforcement, OCAP checks).

### CNS Loops → FlowDef

The CNS `Loop` trait is the loom. It defines the five-phase cycle: sense, compare, compute, act, verify. Domain-specific loops (Cybernetics, Snapshot, StorageGuard) implement this trait with domain-specific signal processing. But the cycle itself is invariant — a loop always senses before comparing, always verifies after acting. The thread (FlowDef in YAML) can declare what signals to monitor and what thresholds to enforce, but it cannot change the cycle structure.

## The Boundary

The boundary is clean: Rust never interprets YAML structurally — YAML describes, Rust enforces. The `manifest_loader` (`crates/hkask-templates/src/manifest_loader.rs`) reads YAML files, deserializes them into strongly typed `BundleManifest` structs via `serde_yaml_neo`, and passes the typed structures to the executor. The YAML's structure is validated at parse time: missing required fields produce errors, unknown fields are ignored or rejected, type mismatches fail immediately.

This is fundamentally different from a system where configuration is arbitrary JSON parsed into `serde_json::Value` and interpreted at runtime. In hKask, there is no runtime YAML traversal. The loom has already cast the thread into its fixed mold before any step executes.

## The Tooling Policy as Loom Hygiene

The AGENTS.md tooling policy reinforces this separation: "hKask is a Rust project. Python is not an acceptable project dependency." This is loom purity. Adding a Python dependency would introduce a second loom — a second interpreter, a second type system, a second security boundary. The hKask project instead favors shell scripts under `scripts/` and Rust binaries for auxiliary tooling. The loom is one language, one compiler, one set of invariants.
