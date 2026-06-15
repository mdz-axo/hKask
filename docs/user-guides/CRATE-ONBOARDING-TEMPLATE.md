---
title: "Crate Onboarding Guide Template"
audience: [developers, contributors]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [composition, lifecycle]
---

# Crate Onboarding Guide Template

**Purpose:** Template for crate-specific onboarding guides. Copy this file, replace `{CRATE}` with the crate name, and fill in crate-specific details.

**Usage:** `cp docs/user-guides/CRATE-ONBOARDING-TEMPLATE.md docs/user-guides/{CRATE}-ONBOARDING.md`

---

## 1. What {CRATE} Does

*One-paragraph summary of the crate's purpose in the hKask architecture.*

**Crate:** `{CRATE}`  
**CNS Loop:** {LOOP_NUMBER} — {LOOP_NAME}  
**Public seams:** {COUNT} (see `architecture/PUBLIC_SURFACE-{CRATE}.md` — replace {CRATE} with actual crate name)

---

## 2. Architecture

### 2.1 Module Map

| Module | Purpose |
|--------|---------|
| `{module}` | {purpose} |

### 2.2 Key Types

| Type | Role |
|------|------|
| `{TypeName}` | {what it represents} |

### 2.3 Dependencies

| Depends On | Why |
|------------|-----|
| `hkask-types` | Core types (WebID, DataCategory, NuEvent) |
| `hkask-mcp` | MCP runtime, daemon client, startup gates |

---

## 3. Public Interface

### 3.1 Primary Entry Points

```rust
// {brief description of the main public function}
pub fn {function_name}({params}) -> {return_type}
```

### 3.2 Configuration

| Env Var | Required | Default | Purpose |
|---------|----------|---------|---------|
| `{ENV_VAR}` | {yes/no} | {default} | {purpose} |

---

## 4. Testing

### 4.1 Test Locations

- Unit tests: `crates/{CRATE}/src/` (inline `#[cfg(test)]` modules)
- Integration tests: `crates/{CRATE}/tests/` (if present)

### 4.2 Running Tests

```bash
cargo test -p {CRATE}
cargo test -p {CRATE} -- --list  # List all tests
```

### 4.3 Key Test Scenarios

| Scenario | Test Function | What It Verifies |
|----------|--------------|-----------------|
| {scenario} | `{test_fn}` | {verification} |

---

## 5. Common Tasks

### 5.1 {Task Name}

```rust
// Example code showing a common usage pattern
```

### 5.2 {Task Name}

```rust
// Example code
```

---

## 6. Error Handling

| Error Variant | When It Occurs | Recovery |
|---------------|---------------|----------|
| `{ErrorVariant}` | {condition} | {recovery action} |

---

## 7. Related Documents

| Document | Relevance |
|----------|-----------|
| [`architecture/core/PRINCIPLES.md`](../architecture/core/PRINCIPLES.md) | Governing principles |
| [`specifications/specs/REQUIREMENTS.md`](../specifications/specs/REQUIREMENTS.md) | Functional requirements |
| [`specifications/specs/TRACEABILITY_MATRIX.md`](../specifications/specs/TRACEABILITY_MATRIX.md) | Spec → code → test traceability |

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
