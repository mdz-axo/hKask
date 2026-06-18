# Contract Traceability Entity-Relationship Diagram

Rendered from `hkask-types::cns::CnsSpan` variants. All edges are materialized by the contract-audit toolchain.

```mermaid
erDiagram
    SpecRequirement ||--|| GoalPrinciple : "activates (exactly 1)"
    GoalPrinciple ||--|| UserExpectation : "expresses"
    UserExpectation ||--|| Contract : "encoded in expect: field"
    Contract ||--o{ ConstrainingPrinciple : "constrains (1..11)"
    Contract ||--|{ Test : "verified by"
    Test ||--|| Implementation : "exercises"
    Implementation ||--|| CnsSpan : "emits"

    ContractProposal ||--o| ContractAccepted : "may become"
    ContractAccepted ||--|| ContractActive : "transitions to"
    ContractActive ||--o| ContractViolated : "may become"

    Contract {
        string specId PK "P{N}-{domain}-{operation}"
        string expect "User expectation text [P{N}]"
        string pre "Precondition"
        string post "Postcondition"
        string inv "Invariant (optional)"
        string goalPrinciple "Exactly 1 from P1-P12"
    }

    Principle {
        int number PK "1-12"
        string name "Human-readable name"
        string role "goal | constraining"
    }

    GoalPrinciple {
        int principleNumber FK "Exactly 1 per contract"
        string rationale "Why this principle justifies the contract"
    }

    ConstrainingPrinciple {
        int principleNumber FK "1..11 per contract"
        string rationale "What the code cannot do"
    }

    UserExpectation {
        string text "Natural language in user voice"
        string goalPrincipleTag "[P{N}]"
    }

    Test {
        string contractId FK "REQ tag"
        string seam "Public interface"
        string testType "unit|integration|contract|fuzz|system"
    }

    Implementation {
        string filePath
        int line
        string functionName
    }

    CnsSpan {
        string variant "cns.contract.{violated,coverage,proposed,accepted,rejected}"
    }

    ContractProposal {
        string proposer FK "WebID (P12)"
        string proposedAt "Timestamp"
        string contractId "Spec ID"
    }

    ContractAccepted {
        string acceptor FK "WebID (P2 consent)"
        string acceptedAt "Timestamp"
    }

    ContractActive {
        string activatedAt "Timestamp"
    }

    ContractViolated {
        string violationType "missing-user-expectation|missing-goal-principle|missing-constraining-annotation|expectation-postcondition-mismatch"
        string severity "critical|high|medium|low"
    }
```

## Forward Chain (Top-Down Traceability)

```mermaid
flowchart LR
    SR["SpecRequirement<br/>spec/goal/capture"] -->|"activates"| GP["GoalPrinciple<br/>[P{N}]"]
    GP -->|"expresses"| UE["UserExpectation<br/>expect: field"]
    UE -->|"encoded in"| CT["Contract<br/>pre:/post:/inv:"]
    CT -->|"verified by"| TS["Test<br/>proptest"]
    TS -->|"exercises"| IM["Implementation<br/>pub fn"]
    IM -->|"emits"| CNS["CnsSpan<br/>cns.contract.*"]
```

## Reverse Verification Chain (Bottom-Up)

```mermaid
flowchart RL
    IM2["Implementation"] -->|"verify<br/>(Link 1)"| CT2["Contract"]
    CT2 -->|"verify<br/>(Link 2)"| UE2["UserExpectation"]
    UE2 -->|"verify<br/>(Link 3)"| GP2["GoalPrinciple"]
```

## Contract Lifecycle State Machine

```mermaid
stateDiagram-v2
    [*] --> ContractProposed: replicant proposes<br/>cns.contract.proposed
    ContractProposed --> ContractAccepted: human accepts (P2)<br/>cns.contract.accepted
    ContractProposed --> [*]: human rejects (P2)<br/>cns.contract.rejected
    ContractAccepted --> ContractActive: tests pass + merge
    ContractActive --> ContractActive: refactor preserves<br/>contract unchanged
    ContractActive --> ContractViolated: test fails<br/>cns.contract.violated
    ContractViolated --> ContractActive: fix + green tests
    ContractViolated --> [*]: contract deprecated/removed

    note right of ContractProposed
        P12: Replicant Host Mandate
        Anonymous agency prohibited
    end note

    note right of ContractAccepted
        P2: Affirmative Consent
        Human must approve every contract
    end note
```

## Constraining Principle Fan-Out

```mermaid
flowchart TD
    CT["Contract<br/>P9-cns-energy-budget-can-proceed"] -->|"hasGoalPrinciple<br/>(exactly 1)"| P9["P9: Homeostatic<br/>Self-Regulation"]
    CT -->|"constrains"| P1["P1: User Sovereignty"]
    CT -->|"constrains"| P2["P2: Affirmative Consent"]
    CT -->|"constrains"| P4["P4: Clear Boundaries"]
    CT -->|"constrains"| P5["P5: Essentialism"]
    CT -->|"constrains"| P8["P8: Semantic Grounding"]

    style P9 fill:#7fff7f,stroke:#333
    style P1 fill:#87ceeb,stroke:#333
    style P2 fill:#87ceeb,stroke:#333
    style P4 fill:#87ceeb,stroke:#333
    style P5 fill:#ffe4b5,stroke:#333
    style P8 fill:#ffe4b5,stroke:#333
```

## RDF Triple Graph — Gap Detection Model

Every gap is a missing or broken triple in the contract graph:

| Gap | Missing Triple |
|-----|---------------|
| Missing `expect:` field | `Contract → hasUserExpectation → EMPTY` |
| Missing `[P{N}]` tag | `UserExpectation → hasGoalPrinciple → EMPTY` |
| Wrong goal principle | `Contract → hasGoalPrinciple → WRONG_Principle` |
| Missing constraining annotation | `Contract → hasConstrainingPrinciple → MISSING` |
| Unanchored test | `Test → verifiesContract → EMPTY` (no REQ tag) |
| Expectation-postcondition mismatch | `UserExpectation` text ≠ `Postcondition` semantics |

## CNS Span ↔ Template Mapping

| CNS Span | Template(s) That Emit |
|----------|----------------------|
| `cns.contract.violated` | tdd-verify (missing fields, mismatches), tdd-gap-check (constraint gaps), contract-audit.sh --principles |
| `cns.contract.coverage` | tdd-verify (coverage ratio), contract-audit.sh --expect |
| `cns.contract.proposed` | tdd-tracer (new contract), Phase B2 workflow |
| `cns.contract.accepted` | tdd-plan (approval gate), Phase B3 consent workflow |
| `cns.contract.rejected` | tdd-verify (rejected proposals), Phase B3 consent workflow |
