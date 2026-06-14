---
title: "Dual-Presence Pattern — Open Specification Questions"
audience: [architects, developers]
last_updated: 2026-06-14
version: "0.27.0"
status: "Active"
domain: "Composition"
mds_categories: [domain, composition, trust]
---

# Dual-Presence Pattern — Open Specification Questions

**Purpose:** Enumerate the questions that must be answered to fully specify the dual-presence pattern — where two entities (a sovereign host replicant and a co-participant daemon) share one CLI/REPL conversation loop. Currently implemented only through `kask curator chat` (user replicant + Curator daemon), but expected to generalize as a reusable pattern.

**Related:** [`PRINCIPLES.md`](../architecture/PRINCIPLES.md) §P12, [`P12-replicant-host-mandate.md`](../architecture/P12-replicant-host-mandate.md), [`loop-architecture.md`](../architecture/loop-architecture.md) §2.4

---

The questions reduce to five architectural decisions. Each must be answered to produce a complete specification.

---

## 1. Presence Model

**What does "being present" actually mean?**

| # | Question | Tension |
|---|----------|---------|
| 1.1 | Is presence **continuous** (Curator observes every message) or **invoked** (Curator only engages when addressed)? | Continuous presence enables proactive system regulation (CNS alerts, budget warnings) but raises P2 consent concerns — the Curator observing every message is data access. Invoked presence is simpler but makes the Curator reactive, not regulatory. |
| 1.2 | Can presence be **toggled**? (`/curator on|off`) If the user disables Curator presence, what happens to system regulation? | P1 says the user is sovereign. But P9 says the system must self-regulate. If the user can silence the Curator, who regulates energy budgets and CNS alerts? |
| 1.3 | Is presence **per-session** (on for the whole chat) or **per-message** (Curator sees only messages directed at it)? | Per-session is simpler to implement. Per-message is more precise but requires an addressing mechanism (Decision 2). |
| 1.4 | Does the Curator have its own **attention model**? Can it "miss" things? Is there a presence indicator so the user knows the Curator is listening? | Without an indicator, the user can't tell whether silence means "nothing to report" or "not listening." |
| 1.5 | Can the Curator **interject unprompted**? On what triggers? (CNS alert, variety deficit, energy budget low, consolidation complete?) | This is the core value of dual-presence — proactive system awareness. But unprompted interjection breaks the user's conversational flow. When is it warranted? |

---

## 2. Addressing Model

**How does the user distinguish between talking to their replicant vs. the Curator?**

| # | Question | Tension |
|---|----------|---------|
| 2.1 | **Natural language**: "Curator, what's my energy budget?" — the Curator responds to direct address. Everything else goes to the replicant. | Simple, intuitive. But requires the Curator to parse every message for address detection. False positives: "I asked the curator about..." |
| 2.2 | **Slash command**: `/curator status` — explicit command prefix. Everything else is normal chat. | Unambiguous. Already exists in REPL (`/repl`, `/model`). But adds syntax the user must learn. |
| 2.3 | **Implicit routing**: The Curator handles system questions; the replicant handles everything else. No explicit addressing needed. | Elegant but ambiguous at the boundary. "What's my energy budget?" — is that a system question or a user question? |
| 2.4 | **Mode switch**: `/talk curator` enters Curator mode; `/talk replicant` returns. Only one participant active at a time. | Clear but loses the "co-presence" value — only one entity is active. This is sequential presence, not dual presence. |
| 2.5 | Does the Curator have a **distinct voice/persona** so the user can tell who responded without checking a label? | The Curator persona spec already defines a distinct voice (direct, technical, concise, no preamble). But is the difference sufficient for disambiguation in practice? |
| 2.6 | How are messages **labeled** in the transcript? `[Curator]` prefix? Color? Icon? | Transcript must be auditable. P8 requires provenance on every statement. |

---

## 3. Authority Model

**Who wins when the sovereign host and the system daemon disagree?**

| # | Question | Tension |
|---|----------|---------|
| 3.1 | Can the Curator **refuse** a user command? ("Don't run that — energy budget exhausted.") | P9: the system must self-regulate. P2: default is deny. But P1: the user is sovereign. This is the core tension. |
| 3.2 | Can the user **override** a Curator refusal? ("I don't care about the budget, run it anyway.") | P1 says yes — user sovereignty is non-negotiable. But if the user can always override, what's the point of Curator regulation? |
| 3.3 | What is the **escalation path** when Curator and user conflict? | Current design: Curator escalates to human. But the human IS the user in this loop. Circular. |
| 3.4 | Is there a **veto** mechanism? Who holds it? Is it symmetric (both can veto) or asymmetric (user always wins)? | Asymmetric: user sovereignty means user always wins. But this makes Curator advisory, not regulatory. Symmetric: deadlock risk. |
| 3.5 | Does the Curator have authority to **refuse service**? (P2: default deny, fail-closed.) | If the Curator can refuse, it's a gatekeeper. If it can't, it's a dashboard. Which is it? |
| 3.6 | What happens when the user gives the Curator a **direct instruction**? ("Curator, increase my gas cap to 50,000.") Does the Curator obey? Evaluate? Refuse? | If the Curator is a daemon, it has its own agency. But if the user is sovereign, the Curator should obey. This is the master/servant vs. partner/advisor tension. |

---

## 4. Memory Model

**Whose memory stores the dual-presence experience?**

| # | Question | Tension |
|---|----------|---------|
| 4.1 | When user and Curator converse, whose **episodic memory** gets the record? | P12: "The host replicant's identity is the `owner` field on every stored triple." But the Curator is also present. Does the Curator get its own memory record? |
| 4.2 | If both encode: are they **separate records** or one **shared record** with two owner fields? | Separate records: each entity remembers the conversation from its own perspective. Shared: one transcript, two witnesses. |
| 4.3 | Does the Curator's observation of the conversation count as an **experience** for consolidation? | If yes: Curator learns from every user interaction. If no: Curator only learns from explicit system events. |
| 4.4 | Does the Curator have its own **episodic memory store**? Or does it write into the user's store with a Curator owner tag? | P1: user owns their data. If Curator writes into user's store, the user owns Curator's memories too. Is that correct? |
| 4.5 | What **context** does the Curator see? Full conversation history? Only system-relevant parts? The user's semantic memory? | Full context: Curator is omniscient observer. Limited context: Curator is a specialist. P1/P2 boundary: Curator seeing sovereign data requires consent. |
| 4.6 | Does the user see the Curator's **internal state**? (CNS readings, energy budget, pending escalations, variety counters?) | Transparency: user can query Curator state. Opacity: Curator is a black box. The dual-presence value proposition depends on transparency. |

---

## 5. Generalization Model

**How does this pattern extend beyond Curator+user?**

| # | Question | Tension |
|---|----------|---------|
| 5.1 | Is dual-presence a **general pattern** or Curator-specific? | If general: needs a framework (participant registration, session manifests). If specific: hardcoded for Curator only. |
| 5.2 | Could **two user replicants** be co-present? (Collaborative session — two humans, two replicants, one Curator?) | This is triple-presence. The addressing model (Decision 2) must scale to N participants. |
| 5.3 | Could a **bot** be co-present? (Bot observes and offers tool results alongside the user and Curator?) | Bots are A2A, not H2A. But in a dual-presence loop, a bot might surface relevant data. Is this a third participant type? |
| 5.4 | Could **multiple daemons** be co-present? (Curator + CNS daemon + Memory daemon?) | Daemon proliferation: each system function gets a presence. But too many voices in the loop is noise. |
| 5.5 | What is the **N-presence model**? Is there a maximum? How are participants registered? | Session manifest: `participants: [replicant:alice, daemon:curator, daemon:cns]`. Registration: declared at session start, immutable for session duration? |
| 5.6 | Does this relate to the **Ensemble** pattern in `hkask-agents`? | Ensemble sessions already model multi-agent collaboration. Is dual-presence a special case of Ensemble (N=2, one daemon)? |
| 5.7 | Does dual-presence extend beyond CLI/REPL to **API** and **MCP server** surfaces? | API: consumer + Curator observing? MCP: IDE agent + Curator observing? If presence is surface-independent, the Curator is everywhere. That's a lot of observation. |

---

## 6. Sovereignty & Consent Boundary

**The P1/P2 questions that cut across all five decisions.**

| # | Question | Tension |
|---|----------|---------|
| 6.1 | The Curator observing a conversation is a form of **data access**. Does this require affirmative consent (P2)? | If yes: Curator presence is opt-in. User must explicitly enable it. If no: Curator presence is default, and the user must explicitly disable it. |
| 6.2 | Is Curator presence **opt-in** or **opt-out**? Default on or default off? | Default on: system regulation is always active. Default off: user must choose to enable system awareness. P2 says default deny — but Curator presence isn't data sharing, it's system function. |
| 6.3 | Can the user **revoke** Curator presence mid-session? What happens to system regulation? | If Curator is the regulator (P9), revoking presence means the system runs unregulated. Is that acceptable? For how long? |
| 6.4 | Does the Curator observing **sovereign data** (episodic memory contents) violate P1? | The Curator needs context to regulate effectively. But sovereign data is private by definition. Is there a "Curator visibility scope" — categories the Curator can see vs. cannot? |
| 6.5 | Is there a **consent manifest** for Curator presence? "I consent to the Curator observing: [energy budgets, CNS alerts, tool outputs] but NOT [episodic memory, personal context]"? | Granular consent aligns with P2 (unbundled, scoped). But adds complexity. |
| 6.6 | Does the Curator's presence need to be **re-affirmed** periodically? (P2: consent is time-bound and version-bound.) | If Curator presence is consent-governed, it should expire and require renewal. But system regulation shouldn't depend on the user remembering to renew. |

---

## Decisions (2026-06-14)

### 1. Presence Model — Continuous Observer, Togglable

**Default: continuous.** The Curator observes every message in the loop. Both the replicant and Curator are available to respond — and expected to respond. They naturally collaborate, taking turns or randomizing who responds first.

**Second-responder context:** Whichever participant responds second includes both the user's initial prompt AND the first participant's response in composing their follow-up. This creates a chain: User → First Responder → Second Responder (with full context of both prior messages).

**Togglable:** Presence can be switched to invoked mode via a setting or `/slash` command within the dual-presence REPL.

**Replicant model diversity:** The replicant may use a different model than the default, or higher temperature, or other adjusted inference settings — to produce diversity in generation and enrich the interplay between user, Curator, and replicant. The ability to dynamically adjust the replicant's model and inference settings in the dual-presence REPL is a required capability.

**Role differentiation:**
- **Curator:** Fixed/default settings. Reliably frames and delivers information. The system's voice.
- **Replicant:** Socratic moderator and facilitator. Helps the user and Curator explore questions from different angles, enriches perspectives, ensures key questions are explored before landing on an answer. Also signals when a problem is simple and doesn't need detailed exploration. The replicant is not the answer-provider — it is the facilitator, moderator, business manager, and Socratic professor in the dual-presence loop.

### 2. Authority Model — Contextual Governance

Both authority models are present, activated by context:

- **When evaluating and acting on CNS data and managing the system with the Curator:** the homeostatic regulatory model governs (P9).
- **When the discussion is not focused on system regulation:** user sovereignty governs (P1), with fewer constraints.

The boundary between these contexts is determined by the subject of the current turn. This is an area for exploration — the exact mechanism for context detection and authority switching is not yet specified.

### 3. Memory Model — Dual Encoding, Public Default

Both the replicant and the Curator store their own memories of the session. Each was present; each remembers. The redundancy is an honest depiction of the event — not a programmatic inefficiency.

**Memory visibility default: public.** To minimize information hoarding and maximize sharing and learning at this early stage of hKask development, dual-presence REPL memories default to public visibility for both the replicant and the Curator. This may be revisited as the system matures and privacy requirements evolve.

### 4. Generalization Model — N+1 Dual Presence

The pattern generalizes to: **(User/Replicant + Any ACP Agent)**. This is an N+1 or dual-presence model — not an ensemble or true multi-agent group tool. The dual-presence pattern will inform the ensemble pattern through practical use, but ensemble is not the target of this specification.

**Ensemble pattern status:** The ensemble pattern exists in code (`hkask-agents`) but is not currently used. Deferring ensemble is prudent — the learning from dual-presence will produce a better ensemble design than speculative architecture. However, literal `todo!()` stubs violate P6. The correct approach: either remove unused ensemble code, or preserve it if it has tests verifying behavioral properties. A formal deferral recorded in `docs/plans/TODO.md` with a reactivation criterion ("when dual-presence has produced N≥3 stable sessions with distinct ACP agents") is the right mechanism.

### 5. Sovereignty & Consent — No Consent Required

The Curator, as the system daemon, is an agent of the user who set up the system. There is full alignment of rights — no consent process is required within the dual-presence REPL. The Curator's observation of the conversation is not data access requiring affirmative consent (P2) because the Curator is functionally an extension of the user's own system administration.

The real sovereignty question is memory visibility (see Decision 3): should the REPL memories be private or public? Default: public.

---

## Decision Dependencies

```
Presence Model ──┬── Addressing Model ──┬── Authority Model
                 │                      │
                 │                      ├── Memory Model
                 │                      │
                 └── Sovereignty Boundary ──┴── Generalization Model
```

The Presence Model (Decision 1) determines everything else. Continuous observer vs. invoked responder changes the addressing mechanism, the authority relationship, the memory encoding, and the sovereignty boundary.

---

## Recommended Answer Order

1. **Presence Model** first — continuous vs. invoked. This is the architectural fork.
2. **Sovereignty Boundary** second — because P1/P2 are non-negotiable and constrain all other decisions.
3. **Authority Model** third — the P1/P9 tension must be resolved before addressing and memory make sense.
4. **Addressing Model** fourth — depends on whether presence is continuous or invoked.
5. **Memory Model** fifth — depends on authority (who owns the record?) and presence (what is observed?).
6. **Generalization Model** last — don't generalize a pattern that isn't stable yet.

---

*Generated during hKask Document Corpus Hygiene Sweep — 2026-06-14*
