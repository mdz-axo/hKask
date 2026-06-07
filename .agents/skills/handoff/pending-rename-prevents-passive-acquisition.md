# Rename `prevents_passive_acquisition()` → `requires_affirmative_consent()`

## Context

The Magna Carta refactoring renamed "Acquisition Resistance" to "Affirmative Consent" throughout hKask. The struct field was renamed (`acquisition_resistance: bool` → `requires_affirmative_consent: bool`), the CLI was updated, the API was updated, docs were rewritten. However, one public accessor method retained the old name:

```rust
// crates/hkask-types/src/sovereignty.rs:176
pub fn prevents_passive_acquisition(&self) -> bool {
    self.requires_affirmative_consent
}
```

The method name `prevents_passive_acquisition` is a "Acquisition Resistance" era artifact. It is semantically accurate (affirmative consent *does* prevent passive acquisition), but it contradicts the renaming principle: **the Magna Carta now describes what the system *does* (require affirmative consent), not what it *resists* (passive acquisition).** The continuation prompt for this work explicitly states: *"'Affirmative Consent' not 'Acquisition Resistance'. The name describes what the system does, not what it resists."*

## Task

Rename the public method `prevents_passive_acquisition()` to `requires_affirmative_consent()` across all crates and docs, keeping the return type and semantics identical.

## Affected Files

### 1. Definition (rename the method)

**`crates/hkask-types/src/sovereignty.rs`** — Line 176

```rust
// BEFORE:
/// Whether this boundary requires affirmative consent
pub fn prevents_passive_acquisition(&self) -> bool {
    self.requires_affirmative_consent
}

// AFTER:
/// Whether this boundary requires affirmative consent (default: true)
pub fn requires_affirmative_consent(&self) -> bool {
    self.requires_affirmative_consent
}
```

Note: The method name now matches the field name. This is intentional — the field is `pub(crate)`, so the public accessor is the only way external crates read this value. The name collision is fine since Rust distinguishes between field access (struct-internal) and method call (external).

### 2. Call site: API route

**`crates/hkask-api/src/routes/sovereignty.rs`** — Lines 21, 106

```rust
// BEFORE (doc comment):
/// Map a boolean `prevents_passive_acquisition` to an affirmative-consent
// AFTER:
/// Map a boolean `requires_affirmative_consent` to an affirmative-consent

// BEFORE (call site):
let requires_affirmative_consent = boundary.prevents_passive_acquisition();
// AFTER:
let requires_affirmative_consent = boundary.requires_affirmative_consent();
```

### 3. Call site: SovereigntyChecker

**`crates/hkask-agents/src/sovereignty.rs`** — Line 107

```rust
// BEFORE:
if operation == "acquisition" {
    return !self.state.boundary.prevents_passive_acquisition();
}
// AFTER:
if operation == "acquisition" {
    return !self.state.boundary.requires_affirmative_consent();
}
```

Also consider renaming `check_operation`'s `"acquisition"` string literal. The operation name "acquisition" is the last remaining "Acquisition Resistance" concept in the call chain. If the operation is being checked for whether affirmative consent is required, the string should probably be `"consent_check"` or similar. However, this is a runtime string that may be used by downstream callers (check `AgentPod::check_sovereignty` in `crates/hkask-agents/src/pod/mod.rs` line ~386). Audit all callers before changing.

### 4. Call site: CLI verifier

**`crates/hkask-cli/src/commands/magna_carta.rs`** — Line 461

```rust
// BEFORE:
if !boundary.prevents_passive_acquisition() {
// AFTER:
if !boundary.requires_affirmative_consent() {
```

### 5. Documentation

**`docs/architecture/magna-carta.md`** — Line 115

```rust
// BEFORE (code block in doc):
pub fn prevents_passive_acquisition(&self) -> bool {
    self.requires_affirmative_consent
}
// AFTER:
pub fn requires_affirmative_consent(&self) -> bool {
    self.requires_affirmative_consent
}
```

## Verification

After renaming, run:

```bash
# 1. Confirm no remaining references to the old name
grep -rn 'prevents_passive_acquisition' crates/ docs/ --include='*.rs' --include='*.md'
# Should return zero hits (excluding archive/)

# 2. Compile all affected crates
cargo check -p hkask-types -p hkask-agents -p hkask-api -p hkask-cli

# 3. Clippy
cargo clippy -p hkask-types -p hkask-agents -p hkask-api -p hkask-cli --no-deps -- -D warnings

# 4. Tests
cargo test -p hkask-types -p hkask-agents -p hkask-api -p hkask-cli

# 5. Run the verifier
cargo build --bin kask && ./target/debug/kask sovereignty verify
```

## Decision Point: The `"acquisition"` Operation String

The `check_operation` method in `SovereigntyChecker` uses the string literal `"acquisition"` to decide whether to check affirmative consent:

```rust
pub fn check_operation(&self, operation: &str, data_category: &DataCategory) -> bool {
    if operation == "acquisition" {
        return !self.state.boundary.requires_affirmative_consent();
    }
    self.can_access(data_category, &self.owner_webid)
}
```

This is called from `AgentPod::check_sovereignty(action, ...)` in `crates/hkask-agents/src/pod/mod.rs`. The `action` parameter is passed as a string from callers.

**Options:**
1. **Rename only the method.** Leave the `"acquisition"` string as-is. It's an internal dispatch key, not a user-facing concept. Low risk.
2. **Rename the method AND the operation string.** Change `"acquisition"` → `"consent_check"` (or `"affirmative_consent"`). This fully removes the "Acquisition Resistance" vocabulary. Requires auditing all callers of `check_sovereignty` / `check_operation` to pass the new string.

Recommend: Start with option 1 (method rename only). Option 2 can be a separate follow-up if desired.

## Constraints

- The return type (`bool`) and semantics (returns `self.requires_affirmative_consent`) do not change.
- This is a public API method on `DataSovereigntyBoundary`. Any external consumers would need updating. hKask currently has no external consumers beyond its own crates.
- Follow the Magna Carta naming principle: describe what the system *does*, not what it *resists*.