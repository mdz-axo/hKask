# Self-Healing Architecture — hKask Autonomous Error Recovery

## Philosophy

Every fallible operation in hKask should pass through a `SelfHealer`. The healer maps error patterns to recovery strategies, attempts healing, and returns either a healed state (retry), a degraded fallback (continue), or an unhealable report (escalate to Curator via CNS).

The governing principle: **errors are not failures — they are signals that trigger autonomous recovery.** Graceful degradation is the LAST resort, not the first response.

## Architecture

```
Error occurs anywhere in the system
    │
    ▼
SelfHealer::attempt(error, context)
    │
    ├── HealRegistry.find_strategy(error) → HealStrategy
    │       │
    │       ▼
    │   HealAction executed
    │       │
    │       ├── RunCommand        → shell command (diagnostic or fix)
    │       ├── SetEnv            → set environment variable
    │       ├── LoadDotEnv        → load .env from search paths
    │       ├── CreateDefaultFile → create missing config/template
    │       ├── RetryWithBackoff  → retry with exponential delay
    │       ├── ProposeCodeChange → CNS alert for Curator review
    │       └── Sequence          → try multiple actions in order
    │
    ▼
HealOutcome
    ├── Healed { action_taken, modifications }
    │       → retry the operation
    ├── Degraded { reason, fallback_description }
    │       → continue with fallback
    └── Unhealable { reason, suggestion, requires_code_change }
            → escalate to Curator via CNS
```

## What Self-Healing Can Modify

| Target | Runtime? | Mechanism | CNS Path |
|--------|----------|-----------|----------|
| `.env` files | ✅ Yes | `LoadDotEnv` — scans common paths, reloads vars | `cns.heal.dotenv` |
| YAML manifests | ✅ Yes | `CreateDefaultFile` — creates missing configs. Reloaded on next read. | `cns.heal.file_created` |
| Jinja2 templates | ✅ Yes | Same as YAML — file changes take effect on next template render | `cns.heal.file_created` |
| Environment variables | ✅ Yes | `SetEnv` — sets `DEEPINFRA_API_KEY` etc. from file/command | `cns.heal.set_env` |
| Rust source code | ❌ No (compiled) | `ProposeCodeChange` — emits CNS alert with diff suggestion for Curator → human review | `cns.heal.code_change_proposed` |
| File permissions | ⚠️ Advisory | `ProposeCodeChange` — suggests `chmod`/`sudo` commands | `cns.heal.code_change_proposed` |
| Missing binaries | ⚠️ Advisory | `ProposeCodeChange` — suggests installation commands | `cns.heal.code_change_proposed` |

## Built-in Healing Strategies

| # | Strategy | Error Pattern | Heal Action |
|---|----------|--------------|-------------|
| 1 | `missing-api-key` | "No API key" | Load `.env` from 4 search paths, then grep for `DEEPINFRA_API_KEY`/`DI_API_KEY` |
| 2 | `permission-denied` | "Permission denied" | Propose `chmod`/`sudo` via CNS alert |
| 3 | `command-not-found` | "command not found" | Propose installation via CNS alert |
| 4 | `config-file-not-found` | "Failed to read classifier config" | Propose creating missing YAML in `registry/classify/` |
| 5 | `network-error` | "connection refused" | RetryWithBackoff (3 attempts, 2s delay) |
| 6 | `transient-retry` | "timeout\|500\|502\|503\|rate limit" | RetryWithBackoff (3 attempts, 1s delay) |

## Usage in Code

### Basic: Wrap a fallible operation

```rust
use hkask_test_harness::self_heal::{SelfHealer, HealContext};

let healer = SelfHealer::new();
let result = healer.healable(
    || fallible_operation(),
    HealContext {
        operation: "classify".into(),
        error_message: String::new(),
        ..Default::default()
    },
);
```

### Advanced: Manual healing with retry

```rust
let healer = SelfHealer::new();
let context = HealContext {
    operation: "read_config".into(),
    error_message: String::new(),
    config_search_paths: vec![PathBuf::from("registry/"), PathBuf::from("~/.config/hkask/")],
    can_retry: true,
};

match healer.attempt(&error.to_string(), &context) {
    HealOutcome::Healed { action_taken, modifications } => {
        // Retry the operation — environment was fixed
        retry_operation()
    }
    HealOutcome::Degraded { reason, fallback_description } => {
        // Use fallback path
        use_fallback(fallback_description)
    }
    HealOutcome::Unhealable { reason, suggestion, requires_code_change } => {
        // Escalate — CNS already notified via tracing::warn!
        report_to_curator(suggestion)
    }
}
```

## Extending the Registry

Add new strategies at runtime or via YAML configuration:

```rust
let mut healer = SelfHealer::new();
healer.registry_mut().add(HealStrategy {
    name: "disk-full".into(),
    error_pattern: "No space left on device".into(),
    description: "Disk full — suggest cleanup".into(),
    action: HealAction::Sequence(vec![
        HealAction::RunCommand {
            command: "df -h".into(),
            capture_output: true,
        },
        HealAction::ProposeCodeChange {
            file: PathBuf::from("(filesystem)"),
            description: "Disk is full".into(),
            diff_suggestion: "Run `du -sh /tmp/*` and clean up old files, or increase disk space.".into(),
        },
    ]),
    can_modify_files: false,
});
```

## CNS Integration

Every healing attempt emits CNS spans:

| Span Target | When | Content |
|-------------|------|---------|
| `cns.heal.attempt` | Healing starts | operation, error message |
| `cns.heal.strategy` | Strategy selected | strategy name, operation |
| `cns.heal.dotenv` | .env loaded | file path |
| `cns.heal.code_change_proposed` | Code change needed | file, description, diff suggestion |
| `cns.heal.unmatched` | No strategy found | operation, error — needs human attention |

The Curator receives unmatched and code-change alerts via the Curation Loop inbox:
```
QA classify step → RuntimeAlert → alerts_tx channel → CurationInput::Alert
    → Curation Loop inbox → CuratorAgent → human review
```

## Integration Points

| System | Where to Wire | Notes |
|--------|---------------|-------|
| **QA Runner** | `QaScriptRunner::run()` — after each step fails | Wrap `execute_classify`, `execute_command` in `healer.healable()` |
| **MCP Servers** | `ToolSpanGuard::error()` — before returning error to client | Heal config/env issues before reporting failure |
| **Inference Router** | `InferenceRouter::generate()` — on connection/rate-limit errors | Retry with backoff, try alternate providers |
| **Template Renderer** | `ManifestExecutor::render_step_template()` — on missing template | Create default template from registry |
| **CNS Runtime** | `CnsObserver::on_depletion()` — on variety deficit | Self-tune thresholds, escalate if unable |

## File Location

- **Engine:** `crates/hkask-test-harness/src/self_heal.rs`
- **Tests:** `#[cfg(test)] mod tests` in same file
- **Registry:** strategies are code-defined (default) with future YAML-configurable extension path

## Design Constraints

1. **Never modify compiled code at runtime.** Rust source changes are proposed via CNS → Curator → human.
2. **File modifications are idempotent.** `CreateDefaultFile` checks existence before writing.
3. **Environment changes are scoped to the process.** `SetEnv` uses `std::env::set_var` (process-local).
4. **All healing is audited.** Every action logs modifications to CNS spans.
5. **Unhealable errors escalate, never silently ignore.**
