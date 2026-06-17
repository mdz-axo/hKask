---
title: "Replicant Onboarding Walkthrough"
audience: [new users, replicant owners, project maintainers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Draft"
domain: "Cross-cutting"
mds_categories: [domain, lifecycle]
---

# Replicant Onboarding Walkthrough

**Purpose:** End-to-end guide from installing hKask through your first conversation with a named replicant. Covers `kask onboard`, passphrase creation, and the first `kask chat` session.

**Governing Principles:** P1 (User Sovereignty), P2 (Affirmative Consent), P12 (Replicant Host Mandate)

---

## 1. Prerequisites

- **Rust toolchain** (stable, via `rustup`): hKask builds from source.
- **Ollama** (optional but recommended): local inference engine. Install from [ollama.com](https://ollama.com). Pull at least one model: `ollama pull qwen3:8b`.
- **SQLite** (system library): `apt install libsqlite3-dev` (Debian/Ubuntu) or `brew install sqlite` (macOS).
- **Git**: hKask uses git for template archival (CAS).

```bash
# Verify prerequisites
rustc --version   # ≥1.80
ollama --version  # optional
sqlite3 --version
git --version
```

---

## 2. Build and Install

```bash
git clone https://github.com/your-org/hKask.git
cd hKask
cargo build --release
# Binary at target/release/kask
```

Optionally, add to PATH:
```bash
export PATH="$PWD/target/release:$PATH"
# Or symlink: ln -s "$PWD/target/release/kask" ~/.local/bin/kask
```

---

## 3. First Launch — The Onboarding Flow

Run `kask` for the first time. If no configuration exists, the onboarding flow starts automatically:

```bash
kask
```

### 3.1 What Happens

1. **Keystore creation.** hKask generates a master key via Argon2id key derivation. This key encrypts all secrets (API keys, passphrases, wallet keys). You'll be prompted for a master passphrase — **write this down and store it securely.** It cannot be recovered.

2. **Replicant creation.** You'll be asked for:
   - **Full name** (first and last, e.g., "Alice Smith"). This becomes your replicant's identity. It's used for WebID derivation and Matrix integration.
   - **Passphrase.** Your replicant's authentication credential. Different from the master passphrase. This is what you'll use to sign in.

3. **API key configuration (optional).** If you have API keys for cloud inference providers (DeepInfra, Fireworks, Together AI), you can enter them now or later via `kask settings set`.

4. **Model selection.** Choose a default inference model. If Ollama is running, local models are detected automatically.

### 3.2 Manual Onboarding

If you skipped onboarding or need to add a replicant to an existing installation:

```bash
kask onboard
```

This runs the same flow interactively. Use `kask onboard --name "Alice Smith"` to pre-fill the name.

---

## 4. Verify Your Replicant

After onboarding, verify everything is set up:

```bash
# List your replicants
kask pod list

# Check sovereignty status
kask sovereignty status

# Verify CNS health
kask cns health
```

Expected output:
```
Replicants:
  alice-smith (webid://alice-smith) — Active

Sovereignty: Maximum (default-deny, explicit consent required)
CNS: 5/5 loops healthy, 0 alerts
```

---

## 5. First Chat Session

Start your first conversation with your replicant:

```bash
kask chat
```

### 5.1 What You'll See

```
ℏKask v0.27.0 — chat session
Replicant: alice-smith
Type /help for commands, /exit to end session.

You: Hello! Who are you?
```

Your replicant responds using the default inference model. The Curator (system persona) mediates the conversation — it routes messages, enforces sovereignty boundaries, and logs interactions to episodic memory.

### 5.2 Key Commands

| Command | What It Does |
|---------|-------------|
| `/help` | List all REPL commands |
| `/improv` | Switch to improv mode (Yes And, Plussing, etc.) |
| `/feedback` | Record feedback about the conversation |
| `/memory` | View recent episodic memories |
| `/model <name>` | Switch inference model mid-session |
| `/exit` | End the session |

### 5.3 What's Happening Under the Hood

- **Authentication:** Your replicant's passphrase is verified against the keystore.
- **OCAP gates:** The daemon verifies your replicant is authenticated, assigned to the chat role, and holds capability tokens for the tools being used.
- **Dual memory:** Every exchange is encoded to episodic memory (personal, sovereign) and semantic memory (shared, consent-gated).
- **CNS monitoring:** The Cybernetic Nervous System tracks variety, algedonic signals, and loop health throughout the session.

---

## 6. Next Steps

### 6.1 Grant Consent for Memory Access

By default, all data access is denied (Magna Carta "Maximum" default). To let your replicant access episodic memory:

```bash
kask sovereignty grant --category episodic_memory --webid webid://alice-smith
```

### 6.2 Explore MCP Servers

hKask ships with 10 MCP servers providing tools for web search, document processing, media analysis, and more:

```bash
kask mcp list-servers
kask mcp list-tools --server research
```

### 6.3 Create Additional Replicants

```bash
kask onboard --name "Bob Jones"
kask pod activate bob-jones
kask chat --replicant bob-jones
```

### 6.4 Read the Guides

| Guide | Location |
|-------|----------|
| Agent pod creation | [`user-guides/AGENT-POD-CREATION-GUIDE.md`](../user-guides/AGENT-POD-CREATION-GUIDE.md) |
| Operations runbook | [`guides/OPERATIONS_RUNBOOK.md`](../guides/OPERATIONS_RUNBOOK.md) |
| Kata user guide | [`guides/kata-user-guide.md`](../guides/kata-user-guide.md) |

---

## 7. Troubleshooting

### "Daemon unavailable"

The hKask daemon runs on a Unix socket at `~/.config/hkask/daemon.sock`. If it's not running, start it via the serve command:

```bash
kask serve
```

### "Replicant not authenticated"

Your replicant's passphrase may have expired or been revoked. Re-authenticate:

```bash
kask replicant login alice-smith
```

### "Model not found"

If using Ollama, ensure the model is pulled:
```bash
ollama pull qwen3:8b
```

If using a cloud provider, verify your API key:
```bash
kask settings show INFERENCE_MODEL
kask settings show DI_API_KEY   # DeepInfra
kask settings show FW_API_KEY   # Fireworks
```

### "Permission denied" on memory access

You haven't granted consent yet. See §6.1 above.

---

## 8. Reference

| Concept | Document |
|---------|----------|
| Magna Carta (P1–P4) | [`architecture/core/magna-carta.md`](../architecture/core/magna-carta.md) |
| Architecture principles (P1–P12) | [`architecture/core/PRINCIPLES.md`](../architecture/core/PRINCIPLES.md) |
| MDS specification framework | [`architecture/core/MDS.md`](../architecture/core/MDS.md) |
| REPL specification | [`specifications/specs/REPL-specification.md`](../specifications/specs/REPL-specification.md) |
| AgentService specification | [`specifications/specs/MDS-agent-service.md`](../specifications/specs/MDS-agent-service.md) |

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
