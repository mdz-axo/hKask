# hkask-agents

Agent pods, ACP, and bot/replicant management for hKask.

## Key Concepts

| Concept | Description |
|---------|-------------|
| **Pods** | Agent containers — CuratorPod, TeamPod, ReplicantPod |
| **ACP** | Agent Communication Protocol — agent-to-agent messaging |
| **Bots** | Subsystem-curator bots (auto-spawn at startup) |
| **Replicants** | Authorial style replicas — embed, compose, compare |
| **Curator** | Human governance agent — metacognition, oversight |
| **OCAP** | Object-capability security — delegation with attenuation |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PROVIDER` | Database provider (`sqlite` or `postgres`) |
| `HKASK_DB_PATH` | SQLite database path |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase |

## Pod Lifecycle