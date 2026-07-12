# TUI Bridge Wiring Architecture

**Type:** flowchart | **Target:** Bridge injection from CLI into TUI windows | **Diataxis quadrant:** Reference

The `hkask-tui` crate defines bridge traits; `hkask-repl` implements them on `TuiReplBridge`. The `hkask-cli` crate wires them via the `with_bridges!` macro-generated builder methods. This separation enforces the dependency rule: TUI never depends on CLI.

```mermaid
flowchart TD
    subgraph "hkask-cli (entry point)"
        A([main]) --> B[Create AgentService context]
        B --> C[Create TuiReplBridge from ReplState]
        C --> D[TuiSession::new bridge]
        D --> E{Wire optional bridges}
    end

    subgraph "Bridge Injection via with_bridges! macro"
        E1["with_wallet_bridge - optional"]
        E2["with_config_bridge - optional"]
        E3["with_backup_bridge - optional"]
        E4["with_registry_bridge - optional"]
        E5["with_memory_bridge - optional"]
        E6["with_kanban_bridge - optional"]
        E7["with_matrix_bridge - optional"]
        E8["with_media_bridge - optional"]
        E9["with_training_bridge - optional"]
        E10["with_companies_bridge - optional"]
        E11["with_research_bridge - optional"]
        E12["with_docproc_bridge - optional"]
        E13["with_replica_bridge - optional"]
        E14["with_skills_bridge - optional"]
        E15["with_scenarios_bridge - optional"]
    end

    E --> E1
    E --> E2
    E --> E3
    E --> E4
    E --> E5
    E --> E6
    E --> E7
    E --> E8
    E --> E9
    E --> E10
    E --> E11
    E --> E12
    E --> E13
    E --> E14
    E --> E15

    subgraph "TuiSession (lib.rs)"
        S["TuiSession.bridges: WorkspaceBridges"]
        S --> S1[Stored as Option Arc dyn Trait]
    end

    subgraph "Window Factory (window_catalog.rs)"
        WF[create_window] --> WF1{match WindowKind}
        WF1 --> WF2[mk_bridge! macro: conditionally wire]
        WF2 --> WF3[Box dyn Window]
    end

    subgraph "Window Implementation"
        W[Window receives Option Arc dyn Trait]
        W --> W1[Method checks is_some at render time]
        W1 --> W2{Has bridge?}
        W2 -->|Yes| W3[Render live data]
        W2 -->|No| W4[Render placeholder / empty state]
    end

    E1 --> S
    E2 --> S
    E3 --> S
    E4 --> S
    E5 --> S
    E6 --> S
    E7 --> S
    E8 --> S
    E9 --> S
    E10 --> S
    E11 --> S
    E12 --> S
    E13 --> S
    E14 --> S
    E15 --> S

    S --> WF
    WF3 --> W

    subgraph "Bridge Impl Location: hkask-repl/src/tui_bridges.rs"
        IMPL["impl Trait for TuiReplBridge"]
        IMPL --> IMPL1[ConfigDataBridge: reads ReplSettings]
        IMPL --> IMPL2[WalletDataBridge: delegates to WalletService]
        IMPL --> IMPL3[KanbanDataBridge: delegates to KanbanState]
        IMPL --> IMPL4[... 12 more impls ...]
    end

    C -.-> IMPL
```

## Dependency Rule

```
hkask-cli ──→ hkask-tui (uses traits + TuiSession)
hkask-cli ──→ hkask-repl (implements bridges)
hkask-repl ──→ hkask-tui (implements traits)
hkask-tui ──✗→ hkask-cli (PROHIBITED — would be circular)
```

## Bridge Lifecycle

| Phase | Action | Location |
|-------|--------|----------|
| 1. Trait definition | `trait WalletDataBridge { ... }` | `hkask-tui/src/bridges/wallet.rs` |
| 2. Mock implementation | `impl WalletDataBridge for MockWalletBridge` | Same file (tests + dev) |
| 3. Live implementation | `impl WalletDataBridge for TuiReplBridge` | `hkask-repl/src/tui_bridges.rs` |
| 4. Injection | `session.with_wallet_bridge(bridge)` | `hkask-cli` (builder pattern) |
| 5. Window wiring | `mk_bridge!(WalletWindow, ctx.wallet_bridge, ...)` | `window_catalog.rs` |
| 6. Consumption | `self.wallet.as_ref().map(|w| w.wallet_balance())` | Window `render()` |

## Key Architectural Decision

Bridges are `Option<Arc<dyn Trait>>` — windows gracefully degrade when a bridge isn't wired. This means:
- The TUI works in test mode (no services) with mock bridges
- Missing bridges show "No data" / placeholder content, never panic
- Adding a new service requires: bridge trait + mock + live impl + `with_bridges!` macro entry + `create_window` match arm (5 sites)

---

*Generated from `crates/hkask-tui/src/bridges/mod.rs`, `window_catalog.rs`, `crates/hkask-repl/src/tui_bridges.rs` — v0.31.0*
