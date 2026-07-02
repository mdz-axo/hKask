---
title: "Authentication Flow — OAuth Sequence Diagram"
audience: [architects, developers, agents]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Composition"
mds_categories: [composition, trust]
diataxis: "sequence-diagram"
source: "crates/hkask-api/src/routes/auth.rs"
---

# OAuth Authentication Flow

## Description

The hKask API authenticates users through GitHub/Google OAuth. The flow follows the standard Authorization Code Grant: the user is redirected to the provider's authorize URL, the provider returns an authorization code, and the server exchanges the code for an access token. The server then creates a session cookie scoped to the authenticated WebID (P12 — no anonymous agency). Every subsequent request carries the session cookie, which the auth middleware validates into a DelegationToken with scoped OCAP permissions (P4).

**Key source:** `crates/hkask-api/src/routes/auth.rs:1-300` (login + callback handlers), `crates/hkask-api/src/middleware/session.rs` (cookie extraction), `crates/hkask-api/src/middleware/auth.rs` (token validation).

**Related:** [MDS.md](../architecture/core/MDS.md) §4.2 (API Surface), [PRINCIPLES.md](../architecture/core/PRINCIPLES.md) P1 (User Sovereignty), P4 (OCAP), P12 (Anonymous Agency)

---

## Authentication Flow Sequence

```mermaid
sequenceDiagram
    actor User
    participant Browser
    participant Axum as hKask API
    participant OAuth as GitHub/Google OAuth
    participant SessionMgr as Auth Session Manager
    participant Keychain as OS Keychain

    User->>Browser: Navigate to /terminal
    Browser->>Axum: GET /api/v1/auth/login?provider=github
    Axum->>Keychain: retrieve OAuth client_id/secret
    Keychain-->>Axum: OAuthConfig
    Axum->>OAuth: Redirect to GitHub authorize URL
    OAuth-->>Browser: GitHub login page
    User->>OAuth: Authorize hKask
    OAuth-->>Browser: Redirect with code+state
    Browser->>Axum: GET /api/v1/auth/callback?provider=github&code=XXX&state=YYY
    Axum->>OAuth: POST /login/oauth/access_token (code)
    OAuth-->>Axum: access_token
    Axum->>OAuth: GET /user (access_token)
    OAuth-->>Axum: GitHub user profile
    Axum->>SessionMgr: Create session (WebID, provider, avatar)
    SessionMgr-->>Axum: Session cookie
    Axum-->>Browser: Set-Cookie: auth-session; Redirect to /terminal
    Browser->>Axum: GET /terminal (with cookie)
    Axum->>SessionMgr: Validate cookie
    SessionMgr-->>Axum: WebID + scoped DelegationToken
    Axum-->>Browser: Terminal UI (authenticated)
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-DA-003
verified_date: 2026-07-01
verified_against: >
  crates/hkask-api/src/routes/auth.rs:1-300
  crates/hkask-api/src/middleware/session.rs
  crates/hkask-api/src/middleware/auth.rs
status: VERIFIED
-->

## Guard Conditions

| Phase | Guard | Failure Mode |
|-------|-------|-------------|
| Login initiation | Provider must be "github" or "google" | 400 Bad Request |
| Callback | CSRF state cookie must match `state` param | 400 Bad Request (CSRF check failed) |
| Code exchange | Valid OAuth authorization code | Provider error (redirect back with error) |
| Session creation | User must exist (auto-created on first sign-in) | 500 Internal Server Error |
| Cookie validation | Session cookie present, not expired | 401 Unauthorized |

## Cross-Reference

- Source: `crates/hkask-api/src/routes/auth.rs`
- Session middleware: `crates/hkask-api/src/middleware/session.rs`
- Auth middleware (token validation): `crates/hkask-api/src/middleware/auth.rs`
- Architecture: `docs/architecture/hKask-architecture-master.md` Pattern C, P4 OCAP
