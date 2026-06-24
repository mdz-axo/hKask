# hkask-mcp-spec

Specification authoring, curation, and validation MCP server.

## Tools (12)

| Tool | Description |
|------|-------------|
| `spec_goal_capture` | Capture a goal as a binding specification requirement. OCAP boundaries are declared inline from context per MDS §3 |
| `spec_goal_decompose` | Decompose a specification goal into ordered sub-goals with dependencies per MDS §3 |
| `spec_require_writing_quality` | Assess a specification's writing quality via the 4-perspective test (Hopper/Lovelace/Schriver/Gentle) per MDS §3. 3/4 = publishable |
| `spec_graph_query` | Query the specification graph by search term with configurable traversal depth per MDS §3 |
| `spec_graph_coherence` | Validate specification collection for internal consistency and coherence per MDS §3 |
| `spec_replica_rewrite` | Rewrite a passage or document using the Gentle Lovelace replica. Optimizes prose for a target quality dimension (Gentle/Schriver/Hopper/Lovelace) using exemplar retrieval and centroid-guided generation |
| `contract_audit` | Discover uncontracted public functions in a crate. Returns coverage percentages and lists of functions lacking REQ contracts for replicant-driven proposals |
| `contract_propose` | Propose a behavioral contract for a public function. Submits for human consent review |
| `contract_accept` | Accept a proposed behavioral contract. Human consent gate per P2 |
| `contract_reject` | Reject a proposed behavioral contract with rationale |
| `contract_list` | List proposed behavioral contracts and their review status |
| `test_run` | Run cargo test on a crate and report REQ-tagged contract violations |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PATH` | SQLite database path |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase |

## Quick Start

```bash
# The server starts automatically with kask
kask chat
# Or standalone:
hkask-mcp-spec
```
