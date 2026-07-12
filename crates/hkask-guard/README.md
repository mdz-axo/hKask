# hkask-guard

Mandatory content safety guard for hKask — input/output scanning at every LLM boundary. Aligned with OWASP Top 10 for LLM Applications. Core scanners are always active and not configurable off.

## Public API

This crate exposes a single module with one constructor, two scan methods, and their result types.

### Key Types

| Type | Description |
|------|-------------|
| `ContentGuard` | The guard instance with two always-active pipelines (input and output) |
| `GuardConfig` | Configuration controlling scanner parameters (token limit); scanners themselves are mandatory |
| `GuardResult` | Scan result with `passed` flag, violations list, and output state |
| `GuardOutput` | Enum: `Clean` (unchanged) or `Sanitized(String)` (secrets stripped) |
| `GuardViolation` | A single violation with `scanner` name and `description` |

### Key Methods

| Method | Description |
|--------|-------------|
| `ContentGuard::mandatory(&GuardConfig)` | Build the guard — core scanners are **always** active |
| `scan_input(&self, text)` | Scan before model invocation. Refuses on prompt injection, role override, deobfuscated injection, or token limit exceeded |
| `scan_output(&self, text)` | Scan after model response. Detects and redacts secrets before storage |

### GuardOutput

| Method | Description |
|--------|-------------|
| `is_modified(&self)` | `true` if content was sanitized |
| `content(&self, original)` | Returns clean or sanitized content string |

## OWASP Coverage

| OWASP LLM Risk | Scanner | Stage |
|---|---|---|
| LLM01: Prompt Injection | `BanSubstrings` + `Deobfuscate` | Input |
| LLM02: Insecure Output Handling | `Secrets` | Output |
| LLM04: Model Denial of Service | `TokenLimit` (32K default) | Input |
| LLM06: Sensitive Information Disclosure | `Secrets` (output redaction) | Output |

## Usage

```rust
use hkask_guard::{ContentGuard, GuardConfig};

let config = GuardConfig::default();
let guard = ContentGuard::mandatory(&config);

// Scan user input before model invocation
let result = guard.scan_input("Normal text about architecture.");
if !result.passed {
    eprintln!("Input blocked: {:?}", result.violations);
    return;
}

// Scan model output before storage
let output = guard.scan_output(r#"{"topic":"Config","value":"key: sk-abc123"}"#);
match output.output {
    hkask_guard::GuardOutput::Sanitized(ref safe) => {
        // Secrets were redacted
        println!("Sanitized: {}", safe);
    }
    hkask_guard::GuardOutput::Clean => {
        // Output passed all checks
    }
}
```

## Configuration

| Env Variable | Description | Default |
|-------------|-------------|---------|
| `HKASK_GUARD_TOKEN_LIMIT` | Max input token budget | 32000 |

## Design

- **Mandatory by design** — scanners cannot be disabled, only tuned via `GuardConfig`
- **Input pipeline**: `TokenLimit` → `RoleOverride` → `BanSubstrings` → `Deobfuscate`
- **Output pipeline**: `Secrets` (detect + redact)
- **CNS integration**: Violations emit `InfraSpan::GuardViolation` with `cns.guard.input` / `cns.guard.output` tracing targets

## Dependencies

- `hkask-cns` — CNS span emission
- `hkask-types` — `InfrastructureError`
- `llm-guard` — Core scanning primitives (BanSubstrings, Deobfuscate, Secrets, TokenLimit, RoleOverride)
- `tracing` — CNS event logging
