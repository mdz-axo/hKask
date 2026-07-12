# Multi-User Data Model (ERD)

Entity-relationship diagram for hKask's multi-user schema (`crates/hkask-storage/src/sql/users.sql`).

## Diagram

```mermaid
erDiagram
    human_users ||--o{ replicant_identities : "user_id"
    human_users ||--o{ user_sessions : "user_id"
    human_users ||--o{ invites : "created_by"
    human_users ||--o{ invites : "accepted_user_id"
    replicant_identities ||--o{ user_sessions : "replicant_name"

    human_users {
        TEXT user_id PK "UUID"
        BLOB email_enc "Encrypted email"
        BLOB phone_enc "Encrypted phone"
        TEXT passphrase_hash "Argon2id hash"
        TEXT salt "Password salt"
        TEXT master_salt "PII key derivation salt"
        INTEGER created_at "Unix timestamp"
        INTEGER last_active "Nullable"
        INTEGER passphrase_set_at "Last passphrase change"
        TEXT role "admin | member (default: member)"
        TEXT oauth_provider "Nullable: github | google"
        TEXT oauth_provider_user_id "Nullable: external ID"
        TEXT oauth_display_name "Nullable: display name from provider"
    }

    replicant_identities {
        TEXT replicant_name PK "Human-readable name"
        TEXT user_id FK "References human_users"
        TEXT replicant_webid UK "WebID URI"
        TEXT wallet_id "Nullable: linked wallet"
        BLOB first_name_enc "Encrypted first name"
        BLOB last_name_enc "Encrypted last name"
        TEXT persona_yaml "Nullable: persona definition"
        INTEGER is_primary "0 or 1"
        INTEGER created_at "Unix timestamp"
        INTEGER last_login "Nullable"
    }

    user_sessions {
        TEXT session_id PK "UUID"
        TEXT replicant_name FK "References replicant_identities"
        TEXT replicant_webid "WebID copy"
        TEXT user_id FK "References human_users"
        TEXT session_key_salt "Key derivation salt"
        INTEGER expires_at "Unix timestamp"
        INTEGER last_active "Unix timestamp"
    }

    invites {
        TEXT invite_id PK "UUID"
        TEXT created_by FK "Admin user_id"
        TEXT code UK "12-char invite code"
        TEXT status "pending | accepted"
        INTEGER created_at "Unix timestamp"
        INTEGER expires_at "7 days from creation"
        INTEGER accepted_at "Nullable: acceptance timestamp"
        TEXT accepted_user_id FK "Nullable: accepting user_id"
    }
```

## Cardinality Notes

- **human_users → replicant_identities:** One-to-many. A human can own multiple replicants.
- **human_users → user_sessions:** One-to-many. A human can have multiple active sessions across replicants.
- **human_users → invites (created_by):** One-to-many. An admin can issue many invites.
- **human_users → invites (accepted_user_id):** One-to-many (nullable). A user can accept multiple invites (though normally one).
- **replicant_identities → user_sessions:** One-to-many. A replicant can have multiple sessions.

## Notable Indexes

| Index | Table | Columns | Purpose |
|-------|-------|---------|---------|
| `idx_replicant_identities_user` | replicant_identities | user_id | Lookup replicants by human |
| `idx_replicant_identities_webid` | replicant_identities | replicant_webid | Lookup by WebID |
| `idx_user_sessions_user` | user_sessions | user_id | Session listing by user |
| `idx_user_sessions_replicant` | user_sessions | replicant_name | Session listing by replicant |
| `idx_user_sessions_expiry` | user_sessions | expires_at | Expired session cleanup |
| `idx_invites_code` | invites | code | Invite lookup by code |
| `idx_invites_created_by` | invites | created_by | Admin's invite listing |

## Cross-References

- Functional spec: `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` §3.16
- Server config: `crates/hkask-types/src/server_config.rs`
- Invite flow: `docs/diagrams/flowchart-oauth-registration.md`
- Invite lifecycle: `docs/diagrams/state-invite-lifecycle.md`
