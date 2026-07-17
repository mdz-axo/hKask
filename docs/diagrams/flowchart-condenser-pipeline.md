# Condenser Pipeline — Architecture Flowchart

**Diataxis type:** Reference
**Status:** Current (v0.31.0)

This diagram traces the condenser MCP server's tool dispatch and compression pipeline. The `CondenserServer` (thin MCP wrapper) delegates to `CondenserEngine` (pure domain logic), which dispatches to one of three compression algorithms based on the classified `ContextCategory`. The ontology anchor — derived from the tool name via bridge crates — feeds domain-aware saliency scoring into `word_rank` and `rtk_style`.

Cross-links:
- [MCP Server Registry](../reference/mcp-servers/README.md) — all built-in MCP servers
- [API Reference: hkask-condenser](../reference/api-reference.md) — full module and type listing
- [Architecture Patterns](../explanation/architecture-patterns.md) — MCP bootstrap and tool dispatch sequence

```mermaid
flowchart TD
    Client["MCP Client\n(kask / external)"]
    
    subgraph Wrapper["hkask-mcp-condenser (thin wrapper)"]
        Server["CondenserServer\nMCP tool router"]
        Ping["condenser_ping"]
        Compress["condenser_compress"]
        Classify["condenser_classify"]
        SetProfile["condenser_set_profile"]
        Stats["condenser_stats"]
        Persist["condenser_persist"]
        ThreadSummary["condenser_thread_summary"]
        ScoreSaliency["condenser_score_saliency"]
    end
    
    subgraph Domain["hkask-condenser (pure domain)"]
        Engine["CondenserEngine\nprofile + stats state"]
        Registry["AlgorithmRegistry"]
        ClassifyFn["classify_tool\ntool_name → category"]
        AnchorFn["derive_ontology_anchor\ntool_name → OntologyAnchor"]
        SaliencyFn["domain_saliency\nline + anchor → f64"]
        OntologyGraph["OntologyGraph\nFIBO/CogAT/GOLEM/ML-Schema/OMC/PKO/DC+BIBO"]
    end
    
    subgraph Algos["Compression Algorithms"]
        Rtk["rtk_style\nhead/tail + density factor"]
        WordRank["word_rank\nTF-IDF + structural + saliency"]
        Flashrank["flashrank\ngreedy marginal utility"]
    end
    
    subgraph Infra["Infrastructure"]
        InferencePort["InferencePort\n(centralized router)"]
        Episodic["EpisodicMemory\n(optional, SQLite-backed)"]
    end
    
    Client -->|"tool call"| Server
    Server --> Ping
    Server --> Compress
    Server --> Classify
    Server --> SetProfile
    Server --> Stats
    Server --> Persist
    Server --> ThreadSummary
    Server --> ScoreSaliency
    
    Ping --> Engine
    Compress --> Engine
    Classify --> Engine
    SetProfile --> Engine
    Stats --> Engine
    
    Engine --> Registry
    Engine --> ClassifyFn
    Engine --> AnchorFn
    Engine --> SaliencyFn
    SaliencyFn --> OntologyGraph
    
    Registry -->|"ShellCommand\nTestOutput\nBuildOutput"| Rtk
    Registry -->|"ConversationHistory\nLogOutput"| WordRank
    Registry -->|"FileContents\nStructuredData\nUnknown"| Flashrank
    
    Rtk -->|"density_factor"| AnchorFn
    WordRank -->|"line_score"| SaliencyFn
    
    Persist --> Episodic
    ThreadSummary --> InferencePort
    ScoreSaliency -->|"against=memory"| Episodic
    ScoreSaliency -->|"against=persona"| SaliencyFn
```