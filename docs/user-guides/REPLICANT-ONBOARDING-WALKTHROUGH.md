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

**Purpose:** End-to-end guide from your first sign-in through your first conversation with a replicant. Covers the browser-based OAuth flow, automatic onboarding, and your first chat session.

**Governing Principles:** P1 (User Sovereignty), P2 (Affirmative Consent), P12 (Replicant Host Mandate)

---

## 1. Prerequisites

- **A modern web browser** (Chrome, Firefox, Safari, or Edge).
- **A GitHub or Google account** for sign-in.
- That's it. No local installation, no command-line tools, no API keys.

---

## 2. Sign In

Open your browser and visit your hKask instance:

```
https://hkask.your-domain.com
```

Click **"Sign in with GitHub"** (or **"Sign in with Google"** if your instance supports it). You'll be redirected through an OAuth consent screen — grant access, and you'll land in your terminal.

If this is your first sign-in, the onboarding flow starts automatically (see §3). If you've signed in before, you go straight to your terminal.

---

## 3. First Sign-In — The Onboarding Flow

On your very first sign-in, hKask provisions your account automatically. You don't need to run any commands — it all happens as soon as you land.

### 3.1 What Happens

1. **OAuth handshake.** hKask exchanges your GitHub (or Google) OAuth token for an identity. This is your root credential — no separate passphrase to remember.

2. **WebID provisioned.** A WebID is derived from your OAuth identity (e.g., `webid://alice-smith`). This is your permanent identifier in the hKask sovereign namespace.

3. **Default replicant created.** A replicant is automatically created for you with a default name derived from your account. You don't choose a name during onboarding — one is assigned. You can customize it later (see §6.3).

4. **Wallet assigned.** A sovereign wallet is generated for your replicant, holding capability tokens and consent grants.

5. **Terminal appears.** The browser loads a terminal emulator connected to your replicant. The Curator greets you with a welcome message — you can start chatting immediately.

### 3.2 Returning Users

If you've signed in before, you skip onboarding and go straight to your terminal. Your replicant, memory, and pods are exactly as you left them.

### 3.3 Customizing Your Replicant Name

The default name is functional but not personal. To rename your replicant at any time:

```bash
kask replicant rename "Alice Smith"
```

(All commands in this guide are typed directly into the browser terminal — you're already connected.)

---

## 4. Verify Your Replicant

After onboarding, your terminal is already connected. Type these commands to confirm everything is set up:

```bash
# List your replicants
kask pod list

# Verify CNS health
kask cns health
```

Expected output:
```
Replicants:
  alice-smith (webid://alice-smith) — Active

CNS: 5/5 loops healthy, 0 alerts
```

---

## 5. First Chat Session

You're already in the terminal after sign-in — no need to start anything. Just type and your replicant responds.

### 5.1 What You'll See

```
ℏKask v0.27.0 — chat session
Replicant: alice-smith
Type /help for commands, /exit to end session.

You: Hello! Who are you?
```

Your replicant responds using the inference model configured by your instance administrator. The Curator (system persona) mediates the conversation — it routes messages, enforces sovereignty boundaries, and logs interactions to episodic memory.

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

- **Authentication:** Your OAuth session token is verified on every request — no separate passphrase needed.
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

### 6.3 Customize Your Replicant

Rename your replicant if you haven't already:

```bash
kask replicant rename "Alice Smith"
```

Create additional replicants:

```bash
kask replicant create --name "Bob Jones"
kask pod activate bob-jones
```

### 6.4 Read the Guides

| Guide | Location |
|-------|----------|
| Agent pod creation | [`user-guides/AGENT-POD-CREATION-GUIDE.md`](../user-guides/AGENT-POD-CREATION-GUIDE.md) |
| Operations runbook | [`guides/OPERATIONS_RUNBOOK.md`](../guides/OPERATIONS_RUNBOOK.md) |
| Kata user guide | [`guides/kata-user-guide.md`](../guides/kata-user-guide.md) |

---

## 7. Troubleshooting

### "Replicant not authenticated"

Your OAuth session may have expired. Sign out and sign in again — your replicant and data are preserved.

### "Permission denied" on memory access

You haven't granted consent yet. See §6.1 above.

### Connection lost

If your terminal disconnects, refresh the page. Your session state is preserved on the server — you'll pick up where you left off.

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
