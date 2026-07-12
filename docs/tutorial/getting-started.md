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

**Purpose:** Take a new developer from zero to a working `kask` session in under 15 minutes. By the end, you will have compiled hKask, created a user profile, run a chat session, invoked a skill, and inspected CNS health.

**Prerequisites:** Rust toolchain (stable, edition 2024), git, a terminal.

---

## 1. Clone and Build

```bash
git clone https://github.com/mdz-axo/hKask.git
cd hKask
cargo build --release
```

The workspace contains 40 crates and 15 MCP servers. The initial build will take several minutes.

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

This checks: database backend (SQLite/SQLCipher by default), keystore availability, CNS runtime health, and MCP server registrations (16 built-in servers).

---

## 3. Initialize a User Profile

```bash
./target/release/kask init
```

You will be prompted for a master passphrase (encrypts your SQLCipher database — store securely, cannot be recovered) and a display name.

This creates:
- `~/.hkask/db.sqlcipher` — encrypted personal database
- `~/.hkask/keystore/` — OS keychain entries for derived encryption keys
- `~/.hkask/settings.yaml` — configuration

---

## 4. Your First Chat Session

```bash
./target/release/kask chat
```

You will see a prompt: `kask> `. Type a message and press Enter. The Curator agent responds.

```
kask> What skills are available?
```

The Curator lists installed skills. To exit: `/quit` or `Ctrl+D`.

---

## 5. Invoke a Skill

Skills are PDCA loops that compose templates into autonomous cycles. Let us invoke `caveman`:

```bash
# List available skills
./target/release/kask skill list

# Invoke the caveman text compression skill
./target/release/kask skill invoke caveman --input "This is verbose text that could be shorter."
```

The skill plans compression, compresses, checks against its convergence threshold, and returns the result.

---

## 6. Read CNS Health

```bash
./target/release/kask cns status
```

Displays: set points (target thresholds), current values, algedonic alerts (critical escalations), and variety counters (system complexity vs. regulatory capacity).

---

## 7. View CNS Spans

```bash
./target/release/kask cns spans --recent 10
```

Each span has a namespace (`cns.tool.reserved`, `cns.inference.completed`, `cns.guard.violation`), timestamp, and domain-specific payload.

---

## 8. REPL Slash Commands

| Command | Action |
|---------|--------|
| `/help` | Show available commands |
| `/quit` | Exit session |
| `/skills` | List available skills |
| `/skill <name>` | Invoke a skill |
| `/memory recall <query>` | Search episodic memory |
| `/cns` | Show CNS health in-chat |
| `/model <name>` | Switch inference model |
| `/condense` | Compress conversation context |
| `/clear` | Clear conversation history |

---

## 9. Next Steps

- **How-To Guides:** [Create an agent pod](../how-to/create-agent-pod.md), [Design a skill](../how-to/design-a-skill.md), [Bootstrap an MCP server](../how-to/bootstrap-mcp-server.md)
- **Reference:** [Crate API reference](../reference/api/), [Skill registry](../reference/skills/README.md), [CNS span registry](../reference/cns-spans.md)
- **Explanation:** [CNS homeostatic loop](../explanation/cns-homeostatic-loop.md), [Hexagonal ports](../explanation/hexagonal-ports.md), [OCAP dispatch](../explanation/ocap-mcp-dispatch.md)

---

*Verified against commit `3d1a876f` (2026-07-07). If any step fails, file an issue — documentation drift is a bug.*
