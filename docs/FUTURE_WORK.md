# Future Work Encapsulation

## Open Questions & Underspecified Aspects

### F6.1: hkask-mcp-gml Purpose

**Question:** Should GML/allosteric thinking be:
- A) Standalone MCP server (current)
- B) Library imported by hkask-ensemble
- C) Template-based cognition (migrate to templates)

**Decision Criteria:**
- If MWC computations are reused → Extract to library
- If GML is agent-specific → Keep as MCP server
- If GML is prompt-based → Migrate to templates

**Timeline:** Decision by 2026-06-22 (90 days)

---

### F6.2: Broadcast/Discovery Mechanism

**Question:** How do agents discover other agent pods?

**Options:**
- A) `tokio-util` watch/broadcast channels
- B) Registry lookup via `hkask-mcp-registry`
- C) ACP runtime discovery protocol

**Current State:** `tokio-util` removed (not yet needed)

**Trigger for Implementation:** When multi-agent discovery is required

---

### F6.3: URL Parsing Requirements

**Question:** When does template registry need URI validation?

**Current State:** `url` crate removed

**Trigger for Re-addition:**
- Template registry accepts remote Git URLs
- Need to validate/normalize URL format
- Cross-repository template references

---

### F6.4: Base64 Encoding for Macaroons

**Question:** Will macaroon issuer be implemented?

**Current State:** `base64` removed, hex encoding used

**Alternatives:**
- Keep hex encoding (simpler, already works)
- Add base64 only if macaroons are implemented
- Use `data-encoding` for multiple formats

**Decision Point:** When capability token format is finalized

---

### F6.5: nalgebra/ndarray for Embeddings

**Question:** Are advanced vector operations needed beyond sqlite-vec?

**Current State:** `nalgebra`/`ndarray` removed

**Triggers for Re-addition:**
- Need matrix operations (not just cosine similarity)
- Embedding transformations (PCA, clustering)
- Custom vector algebra in Rust (not SQL)

**Current Implementation:** `hkask-storage::EmbeddingStore::similarity_search` uses sqlite-vec

---

## Deferred Technical Debt

| TODO | Location | Priority | Deferred Reason |
|------|----------|----------|-----------------|
| Load key from secure keystore | `hkask-ensemble/src/okapi_integration.rs` | Medium | Requires keystore integration |
| Get WebID from Git config | `hkask-templates/src/registry_git.rs` | Low | Nice-to-have, not critical |
| Handle error properly | `hkask-templates/src/registry_sqlite.rs` (2x) | Medium | **Blocked by compilation** |
| Implement template processing | `hkask-cli/src/main.rs` | Medium | **Blocked by compilation** |

---

## Review Cadence

| Review | Frequency | Owner |
|--------|-----------|-------|
| **Open questions** | Monthly | Human |
| **Deferred TODOs** | Bi-weekly | Agent |
| **Trigger events** | As they occur | Automated |

---

*Document generated: 2026-05-22*
*Part of hKask Future Work Encapsulation (Phase 6)*