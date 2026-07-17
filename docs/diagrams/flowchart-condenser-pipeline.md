# Condenser Pipeline — Architecture Flowchart

**Diataxis type:** Reference
**Status:** Current (v0.31.0)

This diagram traces the condenser MCP server's tool dispatch, compression pipeline, and learning loop. The `CondenserServer` (thin MCP wrapper) delegates to `CondenserEngine` (pure domain logic), which dispatches to one of three compression algorithms based on the classified `ContextCategory`. The engine records each compression in a bounded history ring buffer; after 10+ observations per category, it auto-selects the best-performing algorithm (learning). The ChatService's `condense_history` uses two-phase condensation: CPU pre-compress (Phase 1) then LLM summarize (Phase 2).

Cross-links:
- [MCP Server Registry](../reference/mcp-servers/README.md) — all built-in MCP servers
- [API Reference: hkask-condenser](../reference/api-reference.md) — full module and type listing
- [Architecture Patterns](../explanation/architecture-patterns.md) — MCP bootstrap and tool dispatch sequence

```mermaid
flowchart TD
    Client["MCP Client\n(kask / external)"]
    
    subgraph Wrapper["hkask-mcp-condenser (thin wrapper)"]
        Server["CondenserServer\nMCP tool router"]
        Ping["condenser_ping\n+suggested_profile\n+history_stats"]
        Compress["condenser_compress\n+auto-select algorithm"]
        Classify["condenser_classify"]
        SetProfile["condenser_set_profile"]
        Stats["condenser_stats"]
        Persist["condenser_persist"]
        ThreadSummary["condenser_thread_summary"]
        ScoreSaliency["condenser_score_saliency"]
    end
    
    subgraph Domain["hkask-condenser (pure domain)"]
        Engine["CondenserEngine\nprofile + stats + history"]
        Registry["AlgorithmRegistry\nselect + select_by_name"]
        ClassifyFn["classify_tool\ntool_name to category"]
        AnchorFn["derive_ontology_anchor\ntool_name to OntologyAnchor"]
        SaliencyFn["domain_saliency\nline + anchor to f64"]
        SaliencyModule["saliency module\nscore_against_persona\nextract_query_words\nscore_memory_results\nword_frequencies shared"]
        OntologyGraph["OntologyGraph\nFIBO/CogAT/GOLEM/ML-Schema/OMC/PKO/DC+BIBO"]
        History["CompressionRecord ring buffer\n200 max observations"]
        Learning["recommend_algorithm\nsuggest_profile\ncompression_stats"]
    end
    
    subgraph Algos["Compression Algorithms"]
        Rtk["rtk_style\nhead/tail + density factor"]
        WordRank["word_rank\nTF-IDF + structural + saliency"]
        Flashrank["flashrank\ngreedy marginal utility"]
    end
    
    subgraph ChatSvc["hkask-services-chat"]
        CondenseHistory["condense_history\n2-phase: CPU then LLM"]
        Phase1["Phase 1: CPU pre-compress\nCondenserEngine Heavy profile"]
        Phase2["Phase 2: LLM summarize\nInferencePort call"]
    end
    
    subgraph Infra["Infrastructure"]
        InferencePort["InferencePort\n(centralized router)"]
        Episodic["EpisodicMemory\n(optional, SQLite-backed)"]
        Semantic["SemanticMemory\n(optional, SQLite + embeddings)"]
        EmbeddingStore["EmbeddingStore\n1024-dim KNN search"]
        Daemon["Daemon\nstore_experience\n(quality-enriched)"]
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
    Engine --> History
    Engine --> Learning
    Learning -->|"reads"| History
    SaliencyFn --> OntologyGraph
    
    Registry -->|"static default_for"| Rtk
    Registry -->|"static default_for"| WordRank
    Registry -->|"static default_for"| Flashrank
    Learning -->|"learned override"| Registry
    
    Rtk -->|"density_factor"| AnchorFn
    WordRank -->|"line_score"| SaliencyFn
    
    Persist --> Episodic
    ThreadSummary --> InferencePort
    ScoreSaliency -->|"against=persona"| SaliencyModule
    ScoreSaliency -->|"against=memory"| Semantic
    ScoreSaliency -->|"against=memory fallback"| Episodic
    ScoreSaliency -->|"score result count"| SaliencyModule
    Semantic --> EmbeddingStore
    
    Compress -.->|"record_experience\nquality data"| Daemon
    ThreadSummary -.->|"record_experience"| Daemon
    
    CondenseHistory --> Phase1
    Phase1 -->|"CondenserEngine\nProfile::Heavy"| Engine
    Phase1 -->|"compressed text"| Phase2
    Phase2 --> InferencePort
    CondenseHistory -->|"format + estimate"| SaliencyModule
```