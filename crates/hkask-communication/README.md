# hkask-communication

Core Matrix transport, agent registry, 7R7 listener, and CNS bridge for hKask.

## Architecture

```
Matrix message (Conduit)
       │
       ▼
  7R7 Listener (30s poll)
       │
       ├──► tracing log
       │
       └──► NuEvent::persist() → NuEventStore
               │
               ▼
       CurationLoop.sense() (filters communication.* events)
               │
               ▼
       MetacognitionLoop (CAT evaluate → respond template)
               │
               ▼
       MCP: communication.send_message() → Conduit
```

## Features

- **Matrix transport** — full matrix-sdk integration: login, send/receive messages, create rooms, invite users, list rooms, upload/send files
- **Agent registry** — WebID ↔ Matrix UserId mapping with thread watchlists
- **7R7 listener (r7-1)** — passive Matrix room observer, polls rooms, persists CNS NuEvents for curation awareness
- **7R7 receptors (r7-2 through r7-7)** — six domain-specialized CNS observers that track variety, algedonic, composition, consolidation, cybernetics, and curation activity. All are dumb pipes: observe → emit CNS spans → stop. Zero authority.
- **CNS bridge** — communication events flow into the NuEvent store with `communication.message` and `communication.thread` algedonic categories
- **CAT engagement** — Communication Accommodation Theory gate: `convergence_bias` scalar decides speak/silent per agent

### 7R7 Receptor Inventory

| # | Receptor | Struct | CNS Target | Observes |
|---|----------|--------|------------|----------|
| r7-1 | Observer | `SevenR7Listener` | `cns.communication.message.observed` | Matrix room messages |
| r7-2 | Variety | `VarietyReceptor` | `cns.variety.observed` | System variety balance (Ashby's Law) — alert counts, queue depth |
| r7-3 | Algedonic | `AlgedonicReceptor` | `cns.algedonic.observed` | Pain/pleasure signals — severity distributions, resolution rates |
| r7-4 | Composer | `ComposerReceptor` | `cns.composer.observed` | Skill/template composition health — activations, violations |
| r7-5 | Consolidator | `ConsolidatorReceptor` | `cns.consolidator.observed` | Memory consolidation — episodic (PKO) vs semantic (DC+BIBO) rates |
| r7-6 | Cybernetics | `CyberneticsReceptor` | `cns.cybernetics.observed` | CNS meta-health — circuit breakers, self-heal, energy |
| r7-7 | Curator | `CuratorReceptor` | `cns.curator.observed` | Curator activity — metacognition cycles, CAT decisions, directives |

All receptors follow the same architectural invariant: **observe → emit CNS span → stop**. They never classify, escalate, moderate, or judge. Authority stays in the agent layer (Curator + skills + templates).

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `HKASK_MATRIX_URL` | Matrix homeserver URL | `http://localhost:8008` |
| `HKASK_MATRIX_REGISTRATION_TOKEN` | Registration token | `hkask-dev` |
| `HKASK_MATRIX_AGENT_USERNAME` | Agent Matrix username | (from keychain) |
| `HKASK_MATRIX_AGENT_PASSWORD` | Agent Matrix password | (from keychain) |

## Quick Start

```bash
./scripts/conduit/conduit-docker.sh start
./scripts/conduit/conduit-docker.sh register
HKASK_MATRIX_URL=http://localhost:8008 cargo run -- chat
# In REPL: /mcp start communication
```

## Deferred

- E2EE: deferred to v2 (SQLCipher/SQLite linking conflict with matrix-sdk-sqlite)
- Continuous sync: v1 uses on-demand polling via `get_messages()`
- MatrixTransport integration tests: require running Conduit (tests exist, marked `#[ignore]`)
