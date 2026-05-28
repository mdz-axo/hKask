---
title: "hKask Architecture Principles"
audience: [architects, developers, agents]
last_updated: 2026-05-24
version: "1.0.0"
status: "Active"
domain: "Cross-cutting"
---


# hKask Architecture Principles

**Purpose:** Foundational principles governing hKask architecture, derived from cybernetic first principles and constraint-driven design.

**Related:** [`AGENTS.md`](../../AGENTS.md), [`hKask-architecture-master.md`](hKask-architecture-master.md)  
**Verification:** `cargo check --workspace`

---

## 1. Five Anchor Capabilities

hKask is built on five non-negotiable anchor capabilities that define the system's boundaries and purpose.[^cybernetics]

```mermaid
graph TD
    subgraph Anchors[Five Anchor Capabilities]
        A1[1. Agent Enablement<br/>Bots + Replicants in pods]
        A2[2. Essential Tools<br/>15 MCP servers + Okapi]
        A3[3. User Sovereignty<br/>OCAP, SQLCipher, gating]
        A4[4. CNS<br/>cns.* spans, variety counters]
        A5[5. Composition<br/>Unified registry, hLexicon]
    end
    
    subgraph Outcomes[Capability Outcomes]
        O1[Sovereign agents with WebID, ACP]
        O2[LLM inference, embeddings, web]
        O3[Privacy, encryption, ownership]
        O4[Monitoring, algedonic alerts]
        O5[Template-driven wiring]
    end
    
    Anchors --> Outcomes
    
    style Anchors fill:#e1f5ff
    style Outcomes fill:#fff3e1
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-PRIN-001
verified_date: 2026-05-20
verified_against: AGENTS.md; crates/hkask-agents/src/pod.rs; crates/hkask-agents/src/bot.rs; crates/hkask-agents/src/replicant.rs
status: VERIFIED
-->

### 1.1 Agent Enablement

**Principle:** Every agent (bot or replicant) is a sovereign entity with WebID, UCAN capabilities, and ACP communication.[^webid][^ucan][^acp]

**Implementation:**
- Bot/Replicant taxonomy in `hkask-agents` crate
- Agent pods with isolated execution
- A2A (machine-to-machine) and H2A (human-to-agent) interaction modes

**Constraint:** No escalation primitive between bots and replicants. Algedonic alerts handle severity escalation to human.

### 1.2 Essential Tools

**Principle:** Fifteen MCP servers provide all external tooling — no direct HTTP calls from agents.[^mcp]

**Implementation (15 Total):**

**Enabled (15):**
- `hkask-mcp-inference` — Okapi LLM inference
- `hkask-mcp-condenser` — General-purpose context reranking and condensation
- `hkask-mcp-web` — Search, scrape, extract
- `hkask-mcp-ocap` — Capability management
- `hkask-mcp-keystore` — OS keychain
- `hkask-mcp-cns` — CNS operations
- `hkask-mcp-git` — Git CAS
- `hkask-mcp-registry` — Registry operations
- `hkask-mcp-gml` — GML allosteric engine
- `hkask-mcp-spec` — DDMVSS spec capture
- `hkask-mcp-github` — GitHub integration
- `hkask-mcp-fmp` — FMP integration
- `hkask-mcp-telnyx` — Telnyx integration
- `hkask-mcp-fal` — FAL integration
- `hkask-mcp-rss-reader` — RSS feeds

**Converted to Templates (per AGENTS.md):**
- `hkask-mcp-spandrel` → Graph analysis templates
- `hkask-mcp-doc-knowledge` → Document extraction templates

**Constraint:** All MCP servers are `hkask-*` crates — no external MCP dependencies.

### 1.3 User Sovereignty

**Principle:** Users own their data, control delegation, and enforce privacy through OCAP capability attenuation.[^ocap]

**Implementation:**
- SQLCipher encryption with passphrase-derived keys
- Visibility gating (private/public/semantic/episodic)
- Capability tokens attenuate on each recursive delegation

**Constraint:** No cross-machine sync. Git handles backup. Local-first architecture.

### 1.4 Cybernetic Nervous System (CNS)

**Principle:** All system telemetry flows through CNS spans with variety counters and algedonic alerts.[^beer-cybernetics]

**Implementation:**
- Namespace: `cns.*` (replaces deprecated `okh.*`)
- Spans: `cns.tool.*`, `cns.prompt.*`, `cns.agent_pod.*`, `cns.connector.*`
- Algedonic Alert: Variety deficit >100 → escalate to Curator/human

**Constraint:** CNS monitors production system health. Tests verify correctness. Separate concerns.

### 1.5 Composition

**Principle:** Unified registry with `template_type` discriminator enables self-wiring templates.[^jinja2]

**Implementation:**
- Single registry (not three separate)
- Template types: `Prompt`, `Process`, `Cognition`
- hLexicon grounding (75 terms allocated across 3 domains)
- Jinja2 rendering with LLM-based selection

**Constraint:** Selection intelligence in Jinja2/LLM, not Rust code.

---

## 2. Constraint-Driven Design (P1–P7, C1–C7)

**Purpose:** Tailoring rules that prevent architectural decay and maintain minimal viable complexity.[^constraints]

### 2.1 Process Constraints (P1–P7)

| # | Constraint | Enforcement |
|---|------------|-------------|
| **P1** | No trait without two consumers | Compiler error if unused |
| **P2** | No generic without two instantiations | Dead code warning |
| **P3** | No module directory without encapsulation | Architecture review |
| **P4** | No builder without fallibility or complexity | Lint rule |
| **P5** | No feature flag without an activator | `cargo deny` check |
| **P6** | Delete stubs, don't publish them | PR review gate |
| **P7** | Prefer deletion over deprecation | Migration strategy |

### 2.2 Conceptual Constraints (C1–C7)

| # | Constraint | Enforcement |
|---|------------|-------------|
| **C1** | A type must be worn before it's tailored | Use before abstract |
| **C2** | Distinguish dead from unwired | Dead code = removed; Unwired = shelf life |
| **C3** | Unwired code has a shelf life | 30-day limit |
| **C4** | Repetition is a missing primitive | DRY violation → extract |
| **C5** | Every error variant is a unique recovery path | No catch-all variants |
| **C6** | A stub is a debt receipt | Track in OPEN_QUESTIONS.md |
| **C7** | When implementations diverge, one must yield | Consolidation required |

**Verification Command:**
```bash
# Check for unused traits (P1)
cargo check --workspace 2>&1 | grep "never used"

# Check for stubs (P6)
grep -r "todo!\|unimplemented!\|FIXME" crates/ --include="*.rs"

# Check for deprecations (P7)
grep -r "#\[deprecated\]" crates/ --include="*.rs"
```

---

## 3. Hexagonal Boundaries

**Principle:** hKask uses ports and adapters pattern to isolate domain logic from external systems.[^cockburn-hexagonal]

```mermaid
graph TD
    subgraph External[External Systems]
        LLM[Okapi LLM]
        MCP_EXT[MCP Servers]
        GIT[Git CAS]
        KEYCHAIN[OS Keychain]
        WEB[Web Search]
    end
    
    subgraph Adapters[Adapters Layer]
        LLM_ADAPTER[LLM Adapter<br/>OkapiConnector]
        MCP_ADAPTER[MCP Adapter<br/>DispatchRuntime]
        GIT_ADAPTER[Git Adapter<br/>ArtifactStore]
        KEY_ADAPTER[Key Adapter<br/>KeystoreService]
        WEB_ADAPTER[Web Adapter<br/>SearchConnector]
    end
    
    subgraph Ports[Ports Layer]
        INFERENCE_PORT[Inference Port<br/>InferenceProvider]
        STORAGE_PORT[Storage Port<br/>StorageProvider]
        MEMORY_PORT[Memory Port<br/>MemoryProvider]
        SECURITY_PORT[Security Port<br/>SecurityAdapter]
    end
    
    subgraph Domain[Domain Layer — hKask Kernel]
        TEMPLATES[Template Registry<br/>Cascade Engine]
        CNS[CNS<br/>Variety Counters]
        AGENTS[Agent Pods<br/>ACP]
        CAPABILITIES[Capability Model<br/>OCAP]
    end
    
    External --> Adapters
    Adapters --> Ports
    Ports --> Domain
    
    style Domain fill:#e1f5ff
    style Ports fill:#fff3e1
    style Adapters fill:#f3e1ff
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-PRIN-002
verified_date: 2026-05-20
verified_against: crates/hkask-agents/src/adapters/mod.rs; crates/hkask-mcp/src/runtime.rs; crates/hkask-templates/src/ports.rs
status: VERIFIED
-->

### 3.1 What Crosses the Boundary

| crosses | Type | Direction | Example |
|---------|------|-----------|---------|
| Templates | Inbound | External → Domain | `.j2`, `.yaml` files |
| Capabilities | Outbound | Domain → External | OCAP token delegation |
| ν-events | Outbound | Domain → CNS | `cns.tool.*` spans |
| Embeddings | Bidirectional | Both | Vector storage/retrieval |

### 3.2 What Does NOT Cross the Boundary

| Does Not Cross | Reason |
|----------------|--------|
| Direct HTTP calls | All external I/O via MCP |
| Global state | OCAP discipline |
| Ambient authority | Capabilities required |
| Raw SQL | Storage port abstraction |

---

## 4. Stewardship Principles

**Purpose:** Principles for documentation and collaboration stewardship, derived from the Peripheral project pattern.[^peripheral]

| # | Principle | Statement |
|---|-----------|-----------|
| **PS-01** | Declare Shared Goal | Every collaboration context states its purpose |
| **PS-02** | Document Bounded Lexicon | Domain terms defined in hLexicon |
| **PS-03** | Name Mode of Play | Interaction mode (A2A, H2A) explicit |
| **PS-04** | Prefer Invitational Voice | "Consider" over "must" for human-facing |
| **PS-05** | Procedural Rhetoric in ADRs | Decision consequences articulated |
| **PS-06** | Living Documentation | Docs share code lifecycle (Gentle) |
| **PS-07** | Sourced Ideas | Every ## section has external citation |
| **PS-08** | Mermaid-First | Diagrams inline, not external links |
| **PS-09** | DIAGRAM_ALIGNMENT | Every diagram verified with metadata |
| **PS-10** | Writing Excellence | 3 of 4 dimensions pass (Hopper/Lovelace/Schriver/Gentle) |
| **PS-11** | TOGAF Alignment | Every document classified by phase |
| **PS-12** | Git is Archive | Retired docs recoverable via `git show` |

**Verification Command:**
```bash
# Check PS-07: Citation density
for f in docs/architecture/*.md docs/specifications/*.md; do
  citations=$(grep -c '\[\^' "$f")
  sections=$(grep -c '^## ' "$f")
  [ "$citations" -lt "$sections" ] && echo "MISSING CITATIONS: $f"
done

# Check PS-09: DIAGRAM_ALIGNMENT
for f in docs/**/*.md; do
  if grep -q '```mermaid' "$f"; then
    grep -A5 '```mermaid' "$f" | grep -q 'DIAGRAM_ALIGNMENT' || echo "MISSING: $f"
  fi
done
```

---

## 5. Anti-Patterns (Hallucinations)

**Purpose:** Explicitly excluded patterns that violate hKask minimal design.[^minimalism]

| Anti-Pattern | Status | Rationale |
|--------------|--------|-----------|
| Bot reputation systems | ❌ Excluded | Not MVP |
| Bot swarms / consensus | ❌ Excluded | NO swarms per spec |
| Cross-machine sync | ❌ Excluded | Local-first, Git backup |
| Bot marketplace | ❌ Excluded | Not MVP |
| Curator customization | ❌ Excluded | Single system persona |
| SemVer versioning | ❌ Excluded | Git-only (SHA-based) |
| Separate feedback crate | ❌ Excluded | CNS handles all |
| Promotion pipeline | ❌ Excluded | Episodic/semantic categorical |
| Escalation primitive | ❌ Excluded | Algedonic alerts only |
| Visibility type system | ❌ Excluded | OCAP-enforced |
| OCT-H currency | ❌ Excluded | Not implemented |
| Fine-tuning (axolotl) | ❌ Excluded | Out of scope |
| OpenCode/OpenHands condenser | ❌ Excluded | Out of scope |
| UCAN for h-bar | ❌ Excluded | OCAP-only for v0.21.0 |
| Three separate registries | ❌ Excluded | Unified registry |
| Rust-based template selection | ❌ Excluded | Jinja2/LLM selection |

**Verification Command:**
```bash
# Check for anti-pattern implementation
grep -r "reputation\|swarm\|marketplace\|OCT-H\|axolotl" crates/ --include="*.rs"
```

---

## 7. References

[^cybernetics]: Wiener, N. (1948). *Cybernetics: Or Control and Communication in the Animal and the Machine*. MIT Press.
[^webid]: Berners-Lee, T. (2009). *WebID: Secure, decentralized, human-friendly identification*. W3C. <https://www.w3.org/2005/Incubator/webid/>.
[^ucan]: Dialo, D. (2021). *UCAN: User-Controlled Authorization Networks*. Protocol Labs. <https://github.com/ucan-wg/spec>.
[^acp]: ACP Runtime. (2026). *Agent Communication Protocol Specification*. <https://github.com/acp-runtime/acp>.
[^mcp]: Model Context Protocol. (2026). *MCP Specification*. <https://modelcontextprotocol.io/>.
[^ocap]: Miller, M. S. (2006). *Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control*. Johns Hopkins University.
[^beer-cybernetics]: Beer, S. (1972). *Brain of the Firm*. Penguin Books. Algedonic alerts defined in Chapter 12.
[^jinja2]: Jinja2 Developers. (2026). *Jinja Template Designer Reference*. <https://jinja.palletsprojects.com/>.
[^constraints]: Gabriel, R. P. (1991). *The Rise of "Worse is Better"*. Lisp Pointers.
[^cockburn-hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture*. <https://alistair.cockburn.us/hexagonal-architecture/>.
[^peripheral]: Peripheral Project. (2026). *Stewardship Principles*. Documented in `docs/standards/STEWARDSHIP.md`.
[^minimalism]: Raymond, E. S. (2001). *The Art of Unix Programming*. Addison-Wesley. Rule: "When in doubt, cut."
[^budget]: hKask Project. (2026). *AGENTS.md*. `/home/mdz-axolotl/Clones/hKask/AGENTS.md`.
[^testing]: hKask Project. (2026). *AGENTS.md §Workspace Integrity*. `/home/mdz-axolotl/Clones/hKask/AGENTS.md`.

---

*These principles are the foundation for all hKask architecture decisions. Deviations require ADR with procedural rhetoric.*
