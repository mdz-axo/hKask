# hkask-mcp-curator

Curator MCP server — the programmatic interface to the Curator's regulatory surface. Provides system health observability, escalation management, cross-pod semantic search, memory recall, spec drift detection, and algedonic event history.

## Configuration

| Variable | Required | Description |
|----------|----------|-------------|
| `HKASK_CURATOR_DB` | No | Path to Curator's SQLCipher database (defaults to `~/.config/hkask/agents/curator.db`) |
| `HKASK_DB_PASSPHRASE` | No | SQLCipher encryption passphrase |

## Tools (11)

### Liveness
| Tool | Description |
|------|-------------|
| `curator_ping` | Health check — are the curator daemon and stores available? |

### Escalation Management (DB-backed)
| Tool | Description |
|------|-------------|
| `curator_escalations` | List all pending escalations requiring review |
| `curator_escalation_resolve` | Resolve an escalation by ID |
| `curator_escalation_dismiss` | Dismiss an escalation as not actionable |

### System Health (daemon-backed)
| Tool | Description |
|------|-------------|
| `curator_health` | Metacognition cycle — CNS alert counts, overall health |
| `curator_cns_status` | Live CNS variety counters per domain |
| `curator_bot_status` | Per-bot gas consumption vs. energy budget |

### Memory & Learning (DB-backed)
| Tool | Description |
|------|-------------|
| `curator_semantic_search` | Query the Curator's semantic memory by entity name |
| `curator_memory_recall` | Recall episodic and semantic memory about an entity |

### Specification Curation (daemon-backed)
| Tool | Description |
|------|-------------|
| `curator_spec_drift` | Check specs for coherence, drift from registered verbs |

### History (DB-backed)
| Tool | Description |
|------|-------------|
| `curator_algedonic_log` | Read algedonic event history for a time window |

## Operating Modes

- **Daemon mode:** Connects to a running `kask daemon` for live CNS data (health, CNS status, bot status, spec drift). Escalation management, memory recall, and algedonic history work from the Curator's SQLCipher database.
- **Standalone mode:** Opens Curator SQLCipher databases directly. DB-backed tools work; daemon-backed tools return degraded status.

## CNS Spans

All curator tools emit `cns.tool.curator.*` spans with the Curator WebID as the replicant host (P12 compliance).

## Zed Configuration

```json
{
  "context_servers": {
    "hkask-curator": {
      "command": {
        "path": "/path/to/hkask/target/release/hkask-mcp-curator"
      },
      "settings": {
        "HKASK_CURATOR_REPLICANT": "curator",
        "HKASK_DB_PASSPHRASE": "<from-keystore>"
      }
    }
  }
}
```

## See Also

- [`architecture/reference/hKask-Curator-persona.md`](../../docs/architecture/reference/hKask-Curator-persona.md) — Curator persona specification
- [`PRINCIPLES.md`](../../docs/architecture/core/PRINCIPLES.md) §P5, P9, P12
