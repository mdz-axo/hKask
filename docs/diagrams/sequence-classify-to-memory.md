---
title: "Classification-to-Memory Sequence"
diataxis: reference
---

# Classification-to-Memory Sequence

Full flow from source text through dual-model classification, guard scanning,
integration, and shared memory storage. All guard checks are mandatory;
dual-model is mandatory when model B is configured.

Related: `crates/hkask-services-runtime/src/classify_impl.rs`, `crates/hkask-services-runtime/src/dual_classify.rs`

```mermaid
sequenceDiagram
    participant S as Source
    participant G as ContentGuard
    participant MA as Model A (Qwen/KC)
    participant MB as Model B (Gemma/DI)
    participant I as Dual Integrator
    participant CNS as CNS Spans
    participant M as Shared Memory

    S->>G: scan_input(text)
    alt blocked
        G-->>CNS: cns.guard.violation (input_refused)
        G-->>S: Refuse
    else passed
        par Parallel Classification
            MA->>MA: extract_triples_one(text)
            MA-->>I: TripleExtraction A
        and
            MB->>MB: extract_triples_one(text)
            MB-->>I: TripleExtraction B
        end

        I->>I: integrate_dual_triples(A, B)
        I->>I: Jaccard similarity check

        alt agreement >= 0.6
            I->>I: Merge (union, dedup)
        else divergence
            I-->>CNS: cns.classify.dual_fidelity
            I->>I: Merge + annotate [A/B]
        end

        I->>G: scan_output(merged)
        alt secrets detected
            G-->>CNS: cns.guard.violation (output)
            G-->>I: Sanitized output
        else clean
            G-->>I: Pass
        end

        I->>M: store_passage_h_mems()
        I-->>CNS: check_classifier_drift()

        alt drift detected
            CNS-->>CNS: cns.classify.drift
        end
    end
```
