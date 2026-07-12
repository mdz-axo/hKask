---
title: "Guard Violation Lifecycle"
diataxis: reference
---

# Guard Violation Lifecycle

States and transitions for content safety guard violations. Aligned with
OWASP LLM Top 10 risk categories (LLM01, LLM02, LLM04, LLM06).

Related: `crates/hkask-guard/src/pipeline.rs`, `crates/hkask-types/src/cns.rs`

```mermaid
stateDiagram-v2
    [*] --> Scanning

    state Scanning {
        [*] --> InputCheck
        InputCheck --> ModelCall : pass
        InputCheck --> InputRefused : prompt_injection
        InputCheck --> InputRefused : role_override
        InputCheck --> InputRefused : token_limit_exceeded

        ModelCall --> OutputCheck
        OutputCheck --> Store : pass
        OutputCheck --> SecretStripped : api_key_leak
        OutputCheck --> SecretStripped : jwt_leak
        OutputCheck --> SecretStripped : pem_leak
    }

    InputRefused --> CNSLog
    SecretStripped --> CNSLog

    CNSLog --> [*]

    note left of InputRefused
        CNS: cns.guard.violation
        OWASP LLM01 (Prompt Injection)
        OWASP LLM04 (Model DoS)
    end note

    note right of SecretStripped
        CNS: cns.guard.violation
        Secrets redacted before storage
        OWASP LLM02 (Insecure Output)
        OWASP LLM06 (Info Disclosure)
    end note

    note left of CNSLog
        Violation recorded
        Content refused (input)
        or sanitized (output)
        Drift detection monitors
        pattern over time
    end note
```
