# Consumer Trace (T2.0) — `ReplicantIdentity` / `AgentKind` / `is_primary` / `list_replicants`

> Historical trace. The consolidation is complete — zero `replicant`/`Replicant`/`AgentKind` references remain in Rust code.
> This document is retained as a record of the work done.

## A. `ReplicantIdentity` / `replicant_name` / `replicant_webid` consumers

| Crate | File | Usage | 1:1 impact |
|---|---|---|---|
| hkask-identity | `src/lib.rs` | **DEFINITION** (now `UserPod` + alias) | done (T1.1a) |
| hkask-api | `routes/auth.rs` | constructs on OAuth sign-in; logs; `onboard_matrix`; `accept_invite` (`:707`) | keep (1:1 → always one) |
| hkask-api | `routes/replicant.rs` | `ReplicantInfo`, `list_replicants`, rename, delete | **remove list + is_primary** |
| hkask-api | `routes/terminal.rs` | switcher dropdown JS (`:263`) | **remove switcher** |
| hkask-api | `routes/chat_ws.rs` | `session.replicant_webid` (`:191`) | rename field (later) |
| hkask-api | `routes/onboarding.rs` | `q.replicant`, `replicant_name`, `REPLICANT_NAME` template | rename (later) |
| hkask-api | `middleware/session.rs` | `session.replicant_webid` (`:89`) | rename field (later) |
| hkask-api | `middleware/admin.rs` | `get_replicant_by_webid` (`:37`) | rename method (later) |
| hkask-api | `routes/admin.rs` | `get_replicant_by_webid` ×3 (invites) | rename method (later) |
| hkask-cli | `commands/user.rs` | `get_replicant`, `get_replicants`, `login_replicant`, `show_replicant` (prints "Primary: yes"), `list_replicants` (prints primary/secondary), `create_invite` | **remove multi-persona prints; rename later** |
| hkask-storage / hkask-database | (UserStore impl) | `list_replicants`, `get_replicant`, `get_replicant_by_webid`, `rename_replicant`, `delete_replicant` + row→struct mapping with `is_primary` column | **DB schema + methods** |

## B. `AgentKind` consumers

| Crate | File | Usage |
|---|---|---|
| hkask-types | `src/agent/mod.rs` | **DEFINITION** (enum Bot/Replicant) — DELETE |
| hkask-agents | `a2a/mod.rs` | `A2AAgent.agent_type: AgentKind`, `register_agent(webid, agent_type, ...)`, tests use `AgentKind::Bot`/`::Replicant` (~10 sites) |
| hkask-agents | `pod/mod.rs` | `AgentPod.agent_type: AgentKind` (re-export `:88`) |
| hkask-agents | `pod/types.rs` | `AgentIdentity.agent_type`, `validate_fields` enforces `["bot","replicant"]` (`:349`) |
| hkask-agents | `pod/active_pods.rs` | `PodStatusInfo.agent_type`, `create_curator_pod` uses `AgentKind::Bot` (`:376`) |
| hkask-agents | `pod/deployment.rs` | (indirect via persona) |
| hkask-agents | tests ×4 | `agent_pod_integration.rs`, `integration_depth.rs`, `pod_portability.rs`, `a2a` tests |

## C. Sub-slicing proposal (each ≤5 files, build stays green)

The 1:1 simplification and `AgentKind` deletion are each ~8-12 files if done atomically. Break into:

**1:1 multi-persona removal (build-green, alias still present):**
- **S-1to1-a** `hkask-identity`: drop `is_primary` field from `UserPod` + `new()`; update `derive_webid`/`new` callers in identity. — ≤2 files. *Build RED for api/cli until S-1to1-b.*
- **S-1to1-b** `hkask-api/routes/replicant.rs` + `terminal.rs` + storage: remove `list_replicants` endpoint + `is_primary` from `ReplicantInfo`; remove switcher; storage drops `is_primary` column read (keep column for migration safety). — ≤4 files. *Build green.*
- **S-1to1-c** `hkask-cli/commands/user.rs`: drop "primary/secondary" prints; `list_replicants` → single. — ≤1 file. *Build green. Checkpoint.*

**`AgentKind` deletion (strangler-fig: stop using → then delete):**
- **S-AK-a** `hkask-agents/a2a`: drop `agent_type` from `A2AAgent` + `register_agent` signature; update a2a tests. — ≤2 files.
- **S-AK-b** `hkask-agents/pod`: drop `agent_type` from `AgentPod`/`AgentIdentity`; `validate_fields` drops the `["bot","replicant"]` check; `PodStatusInfo` drops `agent_type`. — ≤3 files.
- **S-AK-c** `hkask-types`: delete `AgentKind` enum + `agent/mod.rs` (or empty it). — ≤2 files. *Build green. Checkpoint.*

## D. Open questions surfaced by the trace
- D1: `is_primary` DB column — drop the column (migration) or leave for safety? (lean: leave in this pass; drop in a later migration slice)
- D2: `hkask-storage` `list_replicants`/`get_replicant*` method names — rename to `list_userpod*`/`get_userpod*` now or in Phase 6? (lean: Phase 6, keep build green now)
- D3: `AgentPersona::system("curator", AgentKind::Bot)` at `active_pods.rs:376` — curator persona construction references `AgentKind::Bot`; when AgentKind deletes, curator persona builder needs a Bot-free path. (handled in S-AK-b / T2.2)