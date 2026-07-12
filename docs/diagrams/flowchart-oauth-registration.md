# OAuth Registration & Onboarding Flow

Decision flowchart for hKask's OAuth sign-in pipeline, covering open/closed registration modes, invite validation, and Matrix onboarding.

## Diagram

```mermaid
flowchart TD
    A([User visits hKask server]) --> B{Has session?}
    B -->|Yes| T[/Redirect to /terminal]
    B -->|No| C{OAuth sign-in}
    C --> D[GitHub OAuth flow]
    D --> E[OAuth callback received]
    
    E --> F{Invite cookie present?}
    F -->|Yes| G[Extract invite code]
    F -->|No| H[Verify CSRF state]
    G --> H
    
    H --> I{CSRF valid?}
    I -->|No| J([403 Forbidden])
    I -->|Yes| K[Exchange OAuth code for token]
    
    K --> L[Fetch GitHub user info]
    L --> M{ServerConfig loaded?}
    M -->|No| N[Skip guard: allow]
    M -->|Yes| O{Registration mode?}
    
    O -->|Open| N
    O -->|Closed| P{Invite code valid?}
    P -->|No| Q([403: Invite required])
    P -->|Yes| N
    
    N --> R[find_or_create_oauth_user]
    R --> S{Is invite flow?}
    S -->|Yes| U[accept_invite in DB]
    S -->|No| V[Create session]
    U --> V
    
    V --> W[Set session cookie]
    W --> X[Fire-and-forget: Matrix onboarding]
    X --> Y{Invite flow?}
    Y -->|Yes| Z[/Redirect to /onboarding]
    Y -->|No| T
    
    subgraph "Matrix Onboarding (tokio::spawn)"
        M1[Register human Matrix account] --> M2[Register replicant Matrix account]
        M2 --> M3{Accounts created?}
        M3 -->|Yes| M4[Ensure chat room exists]
        M4 --> M5[Invite users to room]
        M3 -->|No| M6[Log warning: Conduit offline]
    end
    
    X -.-> M1
```

## Key Decision Points

| Node | Decision | Outcome |
|------|----------|---------|
| **F** | Invite cookie? | Bypasses CSRF check — the invite code is the anti-forgery token |
| **M** | Config loaded? | If corrupt/missing, registration guard is skipped (open access fallback) |
| **O** | Open vs Closed | Closed servers require a valid invite code |
| **P** | Invite valid? | Validated against `invites` table (pending + unexpired) |

## Non-Blocking Design

Matrix onboarding runs as a `tokio::spawn` fire-and-forget task. If Conduit is unreachable, the user still gets their session and can use the terminal. Matrix failures are logged but never block sign-in.

## Cross-References

- Server config: `crates/hkask-types/src/server_config.rs`
- Registration guard: `crates/hkask-api/src/routes/auth.rs` (callback handler, lines 281-316)
- Matrix onboarding: `crates/hkask-api/src/routes/auth.rs` (onboard_matrix, lines 728-778)
- Invite lifecycle: `docs/diagrams/state-invite-lifecycle.md`
- ERD: `docs/diagrams/erd-multi-user.md`
