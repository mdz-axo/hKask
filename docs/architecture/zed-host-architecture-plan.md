---
title: zed-kask — Minimal-Divergence Fork Architecture & Migration Plan
audience: hKask architects / zed-kask integrators
last_updated: 2026-07-23
version: 0.31.0
status: proposal
domain: architecture
mds_categories: [composition, trust, lifecycle]
---

# zed-kask — Minimal-Divergence Fork Architecture & Migration Plan

> **One-line frame:** `Clones/zed-kask` is a **fork of Zed** that tracks `upstream` (`zed/zed`) and diverges in **exactly three places**: (1) the **skill module** (skill execution → hKask's `ManifestExecutor`), (2) the **Curator agent** (a new native agent backed by hKask), and (3) the **hKask tool-processing code** (compiled-in hKask crates + in-process tool hosting). Everything else stays byte-identical to upstream and is re-merged regularly. hKask (`Clones/hKask`) is trimmed to **only** the Curator + user sovereignty + the tools. **No backward compatibility.** Principle: *as simple and minimal as possible — and the fork's divergence surface is itself minimal.*

Reasoning chain: `pragmatic-semantics` → `pragmatic-cybernetics` → `falsifiability` → `sequential-inquiry` → `kata-improvement` → `improve-codebase-architecture` → `essentialist` → `skill-router` → `task-breakdown` → `grill-me` → `self-critique-revision`, grounded by reading the actual `zed-kask` crate tree.

---

## 0. Fork Location & Upstream-Sync Strategy (load-bearing)

- **Fork:** `Clones/zed-kask` — `origin` = `github.com/mdz-axo/zed-kask.git`, `upstream` = `github.com/zed/zed.git`, on `main`, currently **in sync** with upstream.
- **Divergence policy:** keep `main` a near-clone of `upstream/main`. All hKask integration is isolated to a **small, named set of crates/files** (§3) so `git fetch upstream && git merge upstream/main` stays low-conflict. No scattered edits across Zed's tree.
- **hKask wiring:** hKask's *keep*-crates (§2.2) are added to `zed-kask`'s Cargo workspace as **path dependencies** (`../../Clones/hKask/crates/...`) for development. For shipping, vendor as a git submodule or subtree so the fork is self-contained. (Decision in T0.6.)
- **Sync cadence (ongoing, Phase 7):** rebase/merge `upstream/main` regularly; resolve conflicts only in the divergent crates; run the hKask integration tests after each sync. The whole point of the fork is to *inherit Zed's improvements for free* — divergence is the cost, so minimize it.

---

## 1. The Enhanced Prompt (minimal-divergence fork)

> Fork Zed into **`zed-kask`** (`Clones/zed-kask`), tracking `upstream` and diverging only in three areas. Trim hKask (`Clones/hKask`) to the Curator + sovereignty + tools, compiled into zed-kask. No backward compatibility.
>
> 1. **zed-kask owns the generic surface and infra** (unchanged from upstream): chat (Agent Panel), GitHub, editor UI, comms/voip/CRDT (replacing Matrix entirely), the **inference router** (`crates/language_model*`), the **provider keystore** (`crates/credentials_provider`), thread storage. These stay byte-identical to upstream.
> 2. **Divergence #1 — skill execution:** change `crates/agent_skills` + `crates/agent/src/tools/skill_tool.rs` so a skill activation runs hKask's **manifest model** — `manifest.yaml` + Jinja2 templates driving a WordAct/FlowDef/KnowAct/RenderAct cascade with PDCA loops, gas/rjoule, OCAP gating — via the compiled-in `ManifestExecutor`, instead of `render_skill_envelope()` injecting the `SKILL.md` body.
> 3. **Divergence #2 — Curator agent:** add the Curator (VSM S4) as a native in-process zed-kask agent (singleton; `CuratorHandle` mpsc authority never crosses a process boundary), selectable in the Agent Panel. ACP is optional (only for external-agent interop).
> 4. **Divergence #3 — hKask tool processing:** compile hKask's keep-crates into zed-kask; host the 15 MCP tools **in-process** (new transport alongside `context_server`'s `StdioTransport`); emit `reg.*` spans directly.
> 5. **Thread → memory:** zed-kask threads parsed into UserPod / Curator episodic + semantic memory (extends the existing ACP per-turn encoding).
> 6. **Remove everything redundant from hKask:** inference router, daemon, ACP seam, MCP-stdio, REPL, chat service, Matrix (all of it), communication MCP, backward-compat shims. **Nothing is removed from zed-kask** — it tracks upstream.
> 7. **Magnac Carta P1–P4, P12 non-negotiable.** `hkask-guard` becomes a layer in zed-kask's inference path so **every** LLM boundary (direct chat + skill cascade + Curator) is guarded — coverage *improves*.

---

## 2. The Essentialist Split (what zed-kask owns vs what hKask keeps)

### 2.1 zed-kask owns (generic — inherited from upstream, NOT modified except integration seams)

Inference routing (`crates/language_model`, `language_model_core`, `language_models`, `language_models_cloud`), provider keystore (`crates/credentials_provider`, `zed_credentials_provider`), chat/Agent Panel (`crates/agent`, `agent_ui`), editor/GitHub/comms/voip/CRDT (`crates/workspace`, `project`, etc.), thread storage (`crates/agent/src/thread_store.rs`), MCP stdio hosting (`crates/context_server`). These stay upstream-identical; we only *add seams* (guard layer, in-process transport) where hKask plugs in.

### 2.2 hKask keeps (unique: curator + sovereignty + tools) — compiled into zed-kask

| Crate | Why irreducible |
|---|---|
| `hkask-types` | Foundation: IDs, `InferencePort` trait, `RegulationSpan`, vocab. |
| `hkask-storage` | **Sovereignty:** per-pod SQLCipher encrypted private sphere (P11.1). |
| `hkask-memory` | Unique semantic/episodic memory + consolidation. |
| `hkask-regulation` | Cybernetic nervous system (`reg.*`, variety, algedonic, set-points). |
| `hkask-templates` | **The tools/skills:** `ManifestExecutor` + registry + cascade + PDCA. |
| `hkask-pods` | **Curator + UserPod** + deployment (sovereignty + curator). |
| `hkask-guard` | **Magna Carta floor (P3.1)** — becomes a layer in zed-kask's inference path. |
| `hkask-capability` | **OCAP** — sovereignty enforcement. |
| `hkask-identity` | **WebID** — sovereignty identity. |
| `hkask-keystore` (trimmed) | **Sovereignty crypto only:** OCAP signing, DB passphrase, internal-secret derivation w/ versioning. *Storage* backend → zed-kask keystore. |
| `hkask-wallet`, `hkask-ledger` | rJoule energy budget + hMem accounting. |
| 15 MCP servers | **The tools** — hosted in-process in zed-kask. |
| `hkask-mcp-server` (framework) | Trim if zed-kask's context_server hosts them natively; keep the `reg.tool.*`+OCAP gating. |

### 2.3 hKask deletes (redundant; jobs move to zed-kask)

`hkask-inference` (router/providers/config — keep only the `InferencePort` *trait* in `hkask-types`), `hkask-acp` (no cross-process), `hkask-repl`, `hkask-services-chat`, `hkask-communication`, `mcp-servers/hkask-mcp-communication`, the daemon/`kask serve`, Matrix sidecar + all Matrix refs, cloud/Hetzner deploy, `hkask-api` chat/chat_ws, backward-compat shims. `hkask-cli` → slim to backup/wallet/repair/admin (links hKask crates). Deletion-test candidates (decide T0.5): `hkask-condenser`, `hkask-git-cas`, `hkask-services-*`, `hkask-mcp-filesystem`.

---

## 3. The Minimal Divergence Map (exact zed-kask touch points)

Every hKask integration maps to a **named, isolated** change in zed-kask. This is the entire divergence surface; everything else tracks upstream.

| # | Divergence | zed-kask crate / file | Change |
|---|---|---|---|
| D1 | Skill execution | `crates/agent_skills` (discovery keeps `SKILL.md` companions) + `crates/agent/src/tools/skill_tool.rs` (`render_skill_envelope`, used by both the `skill` tool and slash commands) | Replace body-injection with: resolve the skill's hKask `manifest.yaml`+templates and run the compiled-in `ManifestExecutor` cascade (PDCA + gas/rjoule + OCAP); return structured result. `SKILL.md` stays the **discovery-only** catalog entry (frontmatter). |
| D2 | Curator agent | `crates/agent/src/agent.rs` + `native_agent_server.rs` + `crates/agent_servers` | Register the Curator as a native in-process agent (singleton); route Curator turns to the in-process `CuratorAgent`. ACP variant optional. |
| D3 | hKask tools in-process | new workspace members (path-deps to `Clones/hKask` keep-crates) + `crates/context_server/src/client.rs` + `transport/` | Add an **in-process transport** alongside `StdioTransport`; the 15 hKask tools register as in-process context servers; emit `reg.*` directly into the ledger. |
| D4 | Guard layer | `crates/language_model_core`/`language_model` (the `LanguageModel` provider abstraction — exact seam verified in T1.2) | Wrap the model with `hkask_guard::GuardedInferencePort` so scan_input/scan_output run on **every** inference call (direct chat + cascade + Curator). |
| D5 | Sovereignty keys | `crates/credentials_provider` / `zed_credentials_provider` | Store hKask sovereignty keys (OCAP signing, DB passphrase, internal secrets) alongside Zed's provider keys. |
| D6 | Thread → memory | `crates/agent/src/thread.rs` / `thread_store.rs` | Hook thread completion → hKask memory ingestion (episodic + semantic). |
| D7 | **App-identity separation** (§7) | `crates/paths/src/paths.rs`, `crates/release_channel/src/lib.rs`, `crates/zed/src/zed/mac_only_instance.rs`, `crates/zed/Cargo.toml`, `script/install.sh`/`uninstall.sh`/`bundle-linux` | Rename the **local footprint** (APP_NAME, app_identifier, app_id, display_name, single-instance port, remote-server dirs, binary) so zed-kask coexists with an upstream zed install; **keep** the shared `*.zed.dev` account/collab endpoints so the user logs into their existing Zed account. |

**Discipline:** D1–D6 are the *only* edits to zed-kask. Any hKask behavior that would require touching other Zed crates is a smell — push the logic into an hKask crate behind one of these seams instead.

---

## 4. Decisive Reasoning (condensed)

- **Pragmatic Semantics:** the fork re-admits the strong claim "change Zed's skill execution." Corrected frame: zed-kask = host + generic infra (upstream-identical); hKask = compiled-in unique crates. One process.
- **Falsifiability:** "embed the ManifestExecutor in Zed" (E2) was *falsified* under "no extension hook / two runtimes." The fork **dissolves both falsifiers** (one process ⇒ one runtime ⇒ one registry ⇒ P5.1 intact; cascade runs in-process ⇒ no OCAP/gas escape). E2 is the corroborated, most-minimal realization. The Curator counterfactual (*do(not in-process Curator)*) still holds — and is now trivially satisfied (one process).
- **Pragmatic Cybernetics:** regulation reads `reg.*` spans + ledger, never the UI ⇒ surface-agnostic ⇒ preserved. In-process tools emit spans directly (fidelity improves). **Guard coverage improves:** the guard layer reaches zed-kask's *direct-chat* inference, which the old daemon model couldn't — strengthening P3.1.
- **Essentialist (crate-level deletion test):** the daemon, ACP, inference router, provider keystore, MCP-stdio, chat/REPL/Matrix all **vanish**; complexity does not reappear (the ManifestExecutor's own loop is skill-execution, not chat surface; the guard moves into zed-kask's path).
- **Essentialist (fork-level):** divergence is itself minimized to D1–D6 so upstream merges stay cheap — the cost of a fork is the maintenance of divergence, so the divergence surface must be minimal and localized.
- **Skill-Router top-5:** `essentialist` (0.92), `improve-codebase-architecture` (0.88), `pragmatic-cybernetics` (0.86), `deep-module` (0.80), `falsifiability` (0.78).

---

## 5. "What Else Are We Forgetting?" (grok findings)

| # | Item | Consequence |
|---|---|---|
| F1 | TTS/voice in `hkask-mcp-communication` + `TranscriptViewer` audio | → zed-kask voip |
| F2 | Curator Matrix posting via the communication MCP (`loop_body.rs` L901) | → post to a zed-kask Curator thread (in-process) |
| F3 | 7R7 passive listener (Matrix rooms → `reg.*`) | → zed-kask thread-watcher background task |
| F4 | Onboarding creates Matrix creds + userpod | → zed-kask first-launch (create UserPod, register agents, no Matrix) |
| F5 | Federation CRDT transport depends on Matrix | → defer for local MVP; intra-process A2A already in-process |
| F6 | `hkask-api` HTTP server (chat, chat_ws, episodic, consolidation, sovereignty, admin) | deletion-test; keep sovereignty/consolidation/admin only if no in-process path |
| F7 | Model providers — zed-kask owns router; hKask keeps guard + `InferencePort` trait | resolved by the fork (D4) |
| F8 | `kask` CLI subcommands | delete matrix/deploy/serve; keep wallet/repair/admin (thin) |
| F9 | Backward-compat shims (pod-kind alias, `persona_yaml` two-source, pre-v0.31 migration, `kask tui -f`) | delete |
| F10 | Double-gate — zed-kask `tool_permissions` (UI pre-filter) × hKask `GovernedTool` (OCAP+gas, final) | define fail-fast → Curator escalation |
| F11 | Always-on Curator — no daemon ⇒ Curator runs only while zed-kask runs | acceptable for local MVP; background/federation deferred |
| F12 | `hkask-mcp-filesystem` overlaps zed-kask file access | deletion-test |

---

## 6. The Plan (phased; edits live in `Clones/zed-kask`; no backward compat; build-then-delete)

> High-risk = the skill-execution change (D1). Foundational = the crate boundary + guarded inference seam. The fork's upstream-sync is an ongoing phase (Phase 7).

### Phase 0 — Decisions (no code)
- **T0.1** ADR: *zed-kask minimal-divergence fork; hKask = compiled-in curator+sovereignty+tools; no backward compat.* XS.
- **T0.2** ADR: *Skill execution = compiled-in `ManifestExecutor` (E2, single runtime); guard = layer in zed-kask inference path.* XS.
- **T0.3** ADR: *Curator = native in-process agent (singleton, in-process `CuratorHandle`); ACP optional.* XS.
- **T0.4** ADR: *Matrix + communication MCP removed; comms/voip/CRDT via zed-kask; federation deferred.* XS.
- **T0.5** Deletion-test verdicts for §2.3 candidates. XS.
- **T0.6** hKask wiring: path-deps (`../../Clones/hKask/...`) for dev vs git-submodule/vendor for shipping. XS.
- **Checkpoint 0:** ADRs + verdicts merged.

### Phase 1 — The crate boundary + guarded inference seam (in zed-kask)
- **T1.1** Add hKask keep-crates (§2.2) as workspace path-deps in `Clones/zed-kask/Cargo.toml`; get them compiling against zed-kask types at the seams. M.
- **T1.2 (D4)** Find the exact `LanguageModel` provider seam in `crates/language_model_core`/`language_model`; implement zed-kask's model behind `hkask_types::InferencePort` and wrap with `hkask_guard::GuardedInferencePort`. AC: a guarded inference call works in-process; `reg.inference` span emitted; **all** inference (chat+cascade+Curator) guarded. M.
- **T1.3 (D5)** Trim `hkask-keystore` to sovereignty crypto; store keys via `crates/credentials_provider`. S.
- **Checkpoint 1:** hKask unique crates compile inside zed-kask; inference guarded in-process.

### Phase 2 — Skill execution (D1, the biggest)
- **T2.1a** In `crates/agent_skills`: keep `SKILL.md` frontmatter as discovery-only catalog metadata; add resolution from a skill to its hKask `manifest.yaml`+templates (registry source of truth). S.
- **T2.1b** In `crates/agent/src/tools/skill_tool.rs`: replace `render_skill_envelope()` body-injection with a call to the compiled-in `ManifestExecutor` cascade (WordAct/FlowDef/KnowAct/RenderAct + PDCA + gas/rjoule + OCAP). Verify the 50KB catalog budget empirically. L.
- **T2.1c** Wire `reg.skill.*` span emission + OCAP/gas gating into the new path. S.
- **T2.2** End-to-end: `/grill-me` in a zed-kask thread runs the KnowAct cascade, returns the assessment, <10s, `reg.skill.activate`+`reg.skill.*` present. S.
- **Checkpoint 2:** skills execute via the hKask cascade, single source of truth, in-process.

### Phase 3 — Agents + tools in-process (D2, D3)
- **T3.1 (D2)** Register the **UserPod** as a native in-process zed-kask agent (`crates/agent/src/agent.rs`/`native_agent_server.rs`); chat turn → guarded inference + OCAP. M.
- **T3.2 (D2)** Register the **Curator** as a native in-process agent (singleton; `CuratorHandle` mpsc in-process; addressable in the Agent Panel). M.
- **T3.3 (D3)** Add an **in-process transport** in `crates/context_server/src/client.rs`+`transport/` alongside `StdioTransport`; host the 15 hKask tools in-process; `reg.tool.*` + OCAP-gated. AC: a tool call runs in-process; span present; `VarietyTracker` shows tool domains. M.
- **T3.4 (F10)** Double-gate reconciliation: zed-kask approval = UI pre-filter; `GovernedTool` = final gate; fail-fast → Curator escalation. S.
- **Checkpoint 3:** UserPod + Curator selectable; tools callable in-process with full regulation observability.

### Phase 4 — Thread → memory + thread watcher (D6)
- **T4.1 (D6)** Thread→memory ingestion: parse zed-kask thread transcripts into episodic h_mems (extend the existing ACP per-turn encoding to full transcripts). M.
- **T4.2** Curator threads → Curator episodic + semantic publish (P11). S.
- **T4.3 (F3)** zed-kask thread-watcher (replaces 7R7): background task observes threads, emits `reg.*` for the conversation surface. S.
- **Checkpoint 4:** zed-kask threads become memory; conversation surface observed.

### Phase 5 — Eager deletion from hKask (build-then-delete; depends on 2,3,4)
- **T5.1** Delete `hkask-inference` (keep `InferencePort` trait). M.
- **T5.2** Delete `hkask-acp` + daemon/`kask serve`; Curator+regulation = zed-kask background tasks. M.
- **T5.3** Delete `hkask-repl`, `hkask-services-chat`, `hkask-cli` chat/tui/transcript_viewer, `kask tui`/`matrix`/`deploy`/`serve`/`doctor`-providers. M.
- **T5.4** Delete `hkask-communication` + `mcp-servers/hkask-mcp-communication`; re-point Curator posting → zed-kask thread (F2); TTS → zed-kask voip (F1); drop from `BUILTIN_SERVERS`+gas table. M.
- **T5.5** Delete Matrix sidecar + `hkask-api` Matrix refs + cloud/Hetzner `matrix_url` + `deploy/k8s/conduit`. M.
- **T5.6** Deletion-test `hkask-api` routes (F6); keep sovereignty/consolidation/admin only if no in-process path. S.
- **T5.7** Delete backward-compat shims (F9) + per T0.5 verdicts (condenser/git-cas/services-*/filesystem-MCP). M.
- **T5.8** Trim `hkask-cli` to backup/wallet/repair/admin. S.
- **Checkpoint 5:** minimal hKask; zed-kask owns all generic infra; CI green.

### Phase 6 — Local install (no daemon)
- **T6.1** zed-kask first-launch onboarding: create UserPod, write `agent.yaml`, register UserPod+Curator, no Matrix. AC: fresh-machine install → both agents in the Panel <5 min. M.
- **T6.2** Verify sovereignty invariants (P1/P4/P11/P12): per-pod SQLCipher, OCAP gating, WebID, consent. S.
- **Checkpoint 6:** end-to-end local install verified on a clean machine.

### Phase 7 — Upstream sync (ongoing)
- **T7.1** Regular `git fetch upstream && git merge upstream/main` in `Clones/zed-kask`; resolve conflicts only in D1–D6 crates; run hKask integration tests + zed-kask build after each sync. Ongoing.
- **T7.2** Keep a `DIVERGENCE.md` in `Clones/zed-kask` listing D1–D6 + the hKask workspace members, so every sync knows exactly what's ours vs upstream's. S.

### Quality-gate (task-breakdown self-evaluation)
- Red flag: **T2.1b is L** (touches Zed's skill path + wires the cascade) — split further if it exceeds one focused session.
- Bias: deletion optimism is real — the guard-seam (T1.2) and 50KB budget (T2.1b) are the genuine unknowns.
- Parallelism: Phase 1 → Phase 2 → (3 ∥ 4) → Phase 5 → Phase 6; Phase 7 ongoing throughout.

---

## 7. App-Identity Separation (zed-kask ↔ zed coexistence)

**Principle (deep-module):** separate the **local filesystem footprint** so `zed-kask` and an upstream `zed` install coexist on the same machine without conflict, while **sharing the Zed account** — the user logs into their existing Zed account and uses zed-kask *as Zed*, with the minimal kask enhancements. Two deep modules own the footprint; a few hardcoded, non-derived points need separate renames (bug-hunt findings).

### 7.1 The two deep modules (single knobs)

| Module | Knob | Today | zed-kask | What it renames |
|---|---|---|---|---|
| `crates/paths/src/paths.rs` | `APP_NAME: &str` (+ derived `APP_NAME_LOWERCASE`) | `"Zed"` | `"Zed-Kask"` / `"zed-kask"` | config/data/state/temp/logs dirs on all OSes; `Zed-Kask.log`; db/extensions/themes/snippets/prompts/settings/keymap/AGENTS.md; macOS `~/Library/Application Support/Zed-Kask` + `~/Library/Logs/Zed-Kask` + `~/.local/state/Zed-Kask`; Linux `$XDG_*_HOME/zed-kask`; Windows `%APPDATA%\Zed-Kask` + `%LOCALAPPDATA%\Zed-Kask`. **The file itself comments: "Forks should change this to avoid colliding with Zed's user data."** |
| `crates/release_channel/src/lib.rs` | `app_identifier()` / `app_id()` / `display_name()` | `"Zed-Editor-Stable"` / `"dev.zed.Zed-Stable"` / `"Zed"` | `"Zed-Kask-Editor"` / `"dev.zed-kask.Zed-Kask"` / `"Zed-Kask"` | Windows single-instance mutex `{id}-Instance-Mutex` + named pipe `\\.\pipe\{id}-Named-Pipe`; macOS bundle id (`~/Library/Preferences/dev.zed-kask.Zed-Kask.plist`, LaunchServices identity); Dock/menu display name. |

**Deletion test:** inlining `APP_NAME`/`app_identifier` at every call site would reappear the platform-path logic everywhere → the modules earn their keep; change the constants, the whole footprint renames. ≤3 public items each, every consumer reads them, nothing writes back → **deep**.

### 7.2 Non-derived collision points (bug-hunt — APP_NAME alone does NOT fix these)

| # | Point | File | Risk | Fix |
|---|---|---|---|---|
| C1 | **macOS single-instance TCP port** | `crates/zed/src/zed/mac_only_instance.rs` `address()` | Port = `43737 + (channel×100) + uid` — keyed on **release channel + uid only**, NOT on APP_NAME. zed-kask and zed-stable (same channel, same uid) → **same port → the second app sees the "Zed Editor Stable Instance Running" handshake and silently exits.** | Distinct port block (fixed offset, e.g. `+500`, or a `Kask` release-channel arm) + change `instance_handshake()` to "Zed-Kask …". |
| C2 | **Remote SSH/WSL server dirs** | `crates/paths/src/paths.rs` `remote_server_dir_relative()`/`remote_wsl_server_dir_relative()` + `crates/util/src/shell.rs` | Hardcoded `.zed_server` / `.zed_wsl_server` on the REMOTE host. SSH to a host where zed also runs → collision + version mismatch. | `.zed-kask_server` / `.zed-kask_wsl_server` (2 path fns + shell.rs). |
| C3 | **Binary name** | `crates/zed/Cargo.toml` `[[bin]] name = "zed"` | Same `zed` binary on PATH → shadows/conflicts. | `[[bin]] name = "zed-kask"` (keep package name `zed` to minimize diff). |
| C4 | **macOS bundle display names** | `crates/zed/Cargo.toml` L281–305 (`"Zed Dev"`…`"Zed"`) | Indistinguishable from zed in Dock/Launchpad. | `"Zed-Kask …"` (via `display_name()`). |
| C5 | **URL scheme `zed://`** | `crates/zed/src/zed/open_listener.rs` + `assets/settings/default.json` `$schema` + `zed://skill` share links | Internal `zed://` prefixes are just strings (safe); the OS-level handler is bundle-id-registered (macOS: only one app owns `zed://`). | **Decision:** keep `zed://` internally (minimal divergence — don't touch open_listener) and accept the macOS handler conflict, OR rename to `zed-kask://` (full isolation, but diverges `default.json` `$schema` + skill-share links). Lean: keep `zed://`; revisit. |

### 7.3 RENAME vs KEEP (the account-sharing constraint)

| RENAME (local footprint — isolated) | KEEP (shared — user logs into their Zed account) |
|---|---|
| `APP_NAME`, `app_identifier`, `app_id`, `display_name` | `default.json` `"server_url": "https://zed.dev"` (collab) |
| config/data/state/cache/logs/db/extensions dirs | `"provider": "zed.dev"`, `"zed.dev": {}` (LLM provider/account) |
| `Zed-Kask.log`, settings/keymap/AGENTS.md paths | `cloud_api_client` `cloud.zed.dev` (account API) |
| Windows mutex/pipe, macOS bundle id + plist | `release_channel::ZED_DOCS_URL` `https://zed.dev/docs` (docs) |
| macOS single-instance port + handshake | `staging-collab.zed.dev` / `collab.zed.dev` (collab relay) |
| `.zed-kask_server` / `.zed-kask_wsl_server` remote dirs | telemetry endpoint (zed.dev) — optional disable |
| binary `zed-kask` | extension marketplace URL (shared; extensions re-installed in the isolated dir) |

**Key invariant:** account/auth/collab traffic goes to `*.zed.dev` keyed on the user's Zed credentials, NOT on bundle id or APP_NAME. Renaming the local identity does **not** affect login — the user signs into the same Zed account and zed-kask behaves as Zed with a separate local footprint.

### 7.4 grill-me challenges (what breaks?)

- **Does renaming the bundle id break Zed account login?** No — auth is to `cloud.zed.dev` keyed on credentials, not bundle id. (Verified: account endpoints live in `default.json`/`cloud_api_client`, independent of `app_id`.)
- **Does renaming APP_NAME orphan existing Zed settings?** It *isolates* them — zed-kask starts fresh (re-onboard); the user's zed settings stay untouched in the old `zed` dirs. Intended.
- **C1 is the silent killer:** an APP_NAME rename does NOT prevent the macOS single-instance collision — verified `address()` keys on channel+uid. Must fix C1 explicitly or zed-kask silently exits whenever zed is running.
- **Extensions:** isolated dir = re-install. Minor cost; benefit = no version conflicts with zed's extensions.
- **Telemetry:** distinct install id (renamed data_dir) → zed-kask reports under a different install id to the same endpoint. Acceptable, or disable.

### 7.5 Tasks (foundational — run in Phase 1, parallel with the crate boundary; pure fork-renaming, no hKask dependency)

- **T-A1** `crates/paths/src/paths.rs`: `APP_NAME = "Zed-Kask"`. AC: `config_dir()`/`data_dir()`/`logs_dir()`/`log_file()` resolve under `zed-kask`/`Zed-Kask` on all OSes. S.
- **T-A2** `crates/release_channel/src/lib.rs`: `app_identifier()` → `Zed-Kask-Editor`; `app_id()` → `dev.zed-kask.Zed-Kask`; `display_name()` Stable → `Zed-Kask`. AC: Windows mutex/pipe + macOS bundle id distinct. S.
- **T-A3 (C1)** `crates/zed/src/zed/mac_only_instance.rs`: distinct port block (offset or `Kask` channel) + `instance_handshake()` "Zed-Kask …". AC: zed-kask runs while zed-stable is running. S.
- **T-A4 (C2)** `crates/paths/src/paths.rs` + `crates/util/src/shell.rs`: `.zed-kask_server` / `.zed-kask_wsl_server`. S.
- **T-A5 (C3/C4)** `crates/zed/Cargo.toml`: `[[bin]] name = "zed-kask"`; macOS display names `Zed-Kask …`. S.
- **T-A6** `script/install.sh`/`uninstall.sh`/`bundle-linux`: `appid`/`app_id` → `dev.zed-kask.Zed-Kask`. S.
- **T-A7** Decision record (C5): keep `zed://` vs rename to `zed-kask://`. XS.
- **T-A8** Verify: with both `zed` and `zed-kask` installed, both launch independently, separate settings, **same Zed account login**. AC: both run concurrently; account works in both. S.

## 8. Open Questions (honestly carried)

1. **ACP vs native** — native in-process recommended (minimal); keep ACP only for external-agent interop (T0.3).
2. **`hkask-keystore` storage backend** — share `crates/credentials_provider`, or thin hKask keychain wrapper? (T1.3).
3. **Exact `LanguageModel` provider seam** for the guard layer — verified in T1.2.
4. **50KB catalog budget** — empirical (T2.1b).
5. **Condenser / git-cas / services-\* / filesystem-MCP** — deletion-test verdicts (T0.5).
6. **Always-on Curator** — runs only while zed-kask runs; background/federation deferred (F11). Acceptable for local MVP?
7. **Double-gate** (F10) — fail-fast behavior (T3.4).
8. **`hkask-api` fate** (F6) — keep sovereignty/consolidation/admin or dissolve into in-process paths? (T5.6).
9. **hKask wiring** — path-deps (dev) vs git-submodule/vendor (shipping) (T0.6).
10. **URL scheme (C5)** — keep `zed://` (minimal divergence, macOS handler conflict) or rename `zed-kask://` (full isolation, diverges settings `$schema` + skill-share links) (T-A7).
11. **macOS single-instance port (C1)** — fixed offset vs a new `Kask` release-channel arm (T-A3).
12. **Extensions** — isolated dir (re-install) vs sharing zed's extensions dir (T-A1 decision).
13. **Telemetry** — distinct install id to shared endpoint vs disable for zed-kask.

---

## 9. Self-Critique-Revision (convergence)

- **Fork grounding strengthens the proposal:** the divergence is now mapped to *exact* zed-kask crates (`agent_skills`, `agent/tools/skill_tool.rs`, `agent`/`agent_servers`, `context_server/client.rs`+`transport/`, `language_model*`, `credentials_provider`), so the "minimal divergence" claim is verifiable, not aspirational.
- **Over-caution corrected:** the fork dissolves the E2 P5/OCAP falsifiers and the ACP/daemon/MCP-stdio seams — the architecture is more minimal than the prior daemon version, and the guard-coverage gap *closes*.
- **New risk honestly added:** upstream-sync conflict cost (Phase 7) is the price of the fork; mitigated by isolating divergence to D1–D6 + a `DIVERGENCE.md`.
- **Calibration:** 0.80 on the compiled-in architecture; 0.6 on T2.1b magnitude; 0.6 on the 50KB budget; 0.7 on low-conflict upstream merges (depends on keeping D1–D6 tightly localized). Honest.
- **Convergence:** quality improved; no criterion regressed; residual is genuine irreducible uncertainty (always-on Curator, keystore backend, 50KB budget, sync-conflict rate), correctly reported rather than iterated past.