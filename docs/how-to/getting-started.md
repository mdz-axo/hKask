---
title: "Getting Started with hKask — End-to-End Tutorial"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# Getting Started with hKask

**Purpose:** Take a new developer from zero to a working `kask` session in under 15 minutes. By the end, you will have compiled hKask, created a user profile, run a chat session, invoked a skill, and inspected Regulation health.

**Prerequisites:** Rust toolchain (stable, edition 2024), git, a terminal.

---

## 1. Clone and Build

```bash
git clone https://github.com/mdz-axo/hKask.git
cd hKask
cargo build --release
```

The workspace contains 54 crates and 15 MCP servers. The initial build will take several minutes.

Verify the binary:

```bash
./target/release/kask --version
```

Expected: `kask 0.31.0`

---

## 2. Health Check

```bash
./target/release/kask health
```

This checks: database backend (SQLite/SQLCipher by default), keystore availability, Regulation runtime health, and MCP server registrations (15 built-in servers).

---

## 3. Initialize a User Profile

```bash
./target/release/kask init
```

You will be prompted for a master passphrase (encrypts your SQLCipher database — store securely, cannot be recovered) and a display name.

This creates:
- `~/.local/share/hkask/hkask.db` — encrypted personal database
- OS keychain entries for the master passphrase and derived encryption keys
- `~/.config/hkask/settings.json` — configuration

### Loading API Keys (Setup-Time Only)

`.env` is the canonical installation settings file. At setup time (`kask init`),
read `.env` and load all settings (API keys + screening thresholds) into the OS
keychain. After setup, `.env` is deprecated — the CLI reads settings exclusively
from the keychain.

```bash
# Copy the template to .env and fill in your settings (one-time setup)
cp key_load_template.env .env
# Edit .env (add DI_API_KEY, KC_API_KEY, OR_API_KEY, HKASK_OR_MAX_PRICE, etc.)

# Load settings into the OS keychain
./target/release/kask keystore load --path .env --prefix HKASK_ --overwrite --shred
```

The `--shred` option securely deletes `.env` after loading. Once settings are in
the keychain, `.env` is no longer needed — `kask` resolves all settings from the
keychain automatically. See [Key Management](#key-management) below.

---

## 4. Start the Daemon

The hKask daemon is a persistent background process that serves P4 OCAP
gate verification (auth, assignment, capability) to MCP server binaries over
a Unix domain socket. Without it, MCP servers fall back to direct mode and
bypass OCAP verification.

```bash
./target/release/kask daemon start &
```

Verify the daemon is running (this pings the socket, not just checks file
existence):

```bash
./target/release/kask daemon status
```

Expected: `Daemon is running (socket: ...)`

See [ADR-035](../architecture/ADRs/ADR-035-userpod-server-mode.md) for the
full daemon architecture and startup flow.

---

## 5. Authenticate Your UserPod

Create a UserStore session so the daemon recognizes your userpod:

```bash
./target/release/kask userpod login
```

You will be prompted for your userpod name and master passphrase. This
creates a session that the daemon's `check_auth` queries to verify MCP
server bootstrap requests.

---

## 6. Your First Chat Session

```bash
./target/release/kask chat
```

You will see a prompt: `kask> `. Type a message and press Enter. The Curator agent responds.

```
kask> What skills are available?
```

The Curator lists installed skills. To exit: `/quit` or `Ctrl+D`.

> **Note:** As of v0.31.0, `kask chat` auto-starts the daemon if it's not
> already running, and `run_onboarding` creates a UserStore session in
> operating mode. The explicit `kask daemon start` and `kask userpod login`
> steps above are still recommended for first-run clarity and for
> environments where you want the daemon running independently of the chat
> session.

---

## 7. Invoke a Skill

Skills are PDCA loops that compose templates into autonomous cycles. Let us invoke `caveman`:

```bash
# List available skills
./target/release/kask skill list

# Invoke the caveman text compression skill
./target/release/kask skill invoke caveman --input "This is verbose text that could be shorter."
```

The skill plans compression, compresses, checks against its convergence threshold, and returns the result.

---

## 8. Read Regulation Health

```bash
./target/release/kask cns status
```

Displays: set points (target thresholds), current values, algedonic alerts (critical escalations), and variety counters (system complexity vs. regulatory capacity).

---

## 9. View Regulation Spans

```bash
./target/release/kask cns alerts
```

Each span has a namespace (`reg.tool.reserved`, `reg.inference.completed`, `reg.guard.violation`), timestamp, and domain-specific payload.

---

## 10. REPL Slash Commands

| Command | Action |
|---------|--------|
| `/help` | Show available commands |
| `/quit` | Exit session |
| `/skills` | List available skills |
| `/skill <name>` | Invoke a skill |
| `/memory recall <query>` | Search episodic memory |
| `/cns` | Show Regulation health in-chat |
| `/model <name>` | Switch inference model |
| `/condense` | Compress conversation context |
| `/clear` | Clear conversation history |

---

## 11. Next Steps

- **How-To Guides:** [Create an agent pod](../how-to/agents-and-pods.md), [Design a skill](../how-to/skills-and-composition.md), [Bootstrap an MCP server](../how-to/skills-and-composition.md)
- **Reference:** [Crate API reference](../reference/api-reference.md), [Skill registry](../reference/skills/README.md), [Regulation span registry](../reference/regulation-spans.md)
- **Explanation:** [Regulation homeostatic loop](../explanation/cns-and-loops.md), [Hexagonal ports](../explanation/architecture-patterns.md), [OCAP dispatch](../explanation/sovereignty-and-ocap.md)

---

*Verified against commit `3d1a876f` (2026-07-07). If any step fails, file an issue — documentation drift is a bug.*
