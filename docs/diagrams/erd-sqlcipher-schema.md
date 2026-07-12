---
title: "SQLCipher Schema — ERD"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, trust]
last-verified-against: "3d1a876f"
diataxis: reference
---

```mermaid
erDiagram
    human_users {
        TEXT user_id PK
        BLOB email_enc
        BLOB phone_enc
        TEXT passphrase_hash
        TEXT salt
        TEXT master_salt
        INTEGER created_at
        INTEGER last_active
        INTEGER passphrase_set_at
        TEXT role
        TEXT oauth_provider
        TEXT oauth_provider_user_id
        TEXT oauth_display_name
    }

    replicant_identities {
        TEXT replicant_name PK
        TEXT user_id FK
        TEXT replicant_webid UK
        TEXT wallet_id
        BLOB first_name_enc
        BLOB last_name_enc
        TEXT persona_yaml
        INTEGER is_primary
        INTEGER created_at
        INTEGER last_login
    }

    user_sessions {
        TEXT session_id PK
        TEXT replicant_name FK
        TEXT replicant_webid
        TEXT user_id FK
        TEXT session_key_salt
        INTEGER expires_at
        INTEGER last_active
    }

    invites {
        TEXT invite_id PK
        TEXT created_by FK
        TEXT code UK
        TEXT status
        INTEGER created_at
        INTEGER expires_at
        INTEGER accepted_at
        TEXT accepted_user_id FK
    }

    agent_registry {
        TEXT name PK
        TEXT agent_kind
        TEXT definition_json
        TEXT token_hash
        TEXT registered_at
        TEXT source_yaml
    }

    nu_events {
        TEXT id PK
        TEXT timestamp
        TEXT observer_webid
        TEXT span_category
        TEXT span_path
        TEXT phase
        TEXT observation
        TEXT regulation
        TEXT outcome
        INTEGER recursion_depth
        TEXT parent_event
        TEXT visibility
    }

    hmems {
        TEXT id PK
        TEXT entity
        TEXT attribute
        TEXT value
        TEXT valid_from
        TEXT valid_to
        TEXT recalled_at
        TEXT transaction_at
        REAL confidence
        TEXT perspective
        TEXT visibility
        TEXT owner_webid
        TEXT dimension
    }

    embeddings {
        TEXT id PK
        TEXT entity_ref FK
        BLOB vector
        INTEGER dimensions
        TEXT model
        TEXT created_at
    }

    vec_embeddings {
        TEXT id PK
        REAL_ARRAY embedding
    }

    goals {
        TEXT id PK
        TEXT webid
        TEXT text
        TEXT state
        TEXT visibility
        TEXT created_at
        TEXT completed_at
        TEXT parent_goal_id
        INTEGER depth
        TEXT display_name
    }

    goal_criteria {
        TEXT id PK
        TEXT goal_id FK
        TEXT type
        TEXT description
        INTEGER satisfied
    }

    goal_artifacts {
        TEXT id PK
        TEXT goal_id FK
        TEXT artifact_ref
        TEXT artifact_type
        TEXT created_at
    }

    quarantined_goals {
        TEXT id PK
        TEXT original_data
        TEXT quarantine_reason
        TEXT quarantined_at
        INTEGER repair_attempts
        INTEGER repaired
    }

    consent_records {
        TEXT id PK
        TEXT webid UK
        TEXT granted_categories
        INTEGER granted_at
        INTEGER revoked_at
        INTEGER active
    }

    wallet_balances {
        TEXT wallet_id PK
        INTEGER balance_rj
        INTEGER usdc_equivalent_micro
        TEXT created_at
        TEXT updated_at
    }

    wallet_transactions {
        INTEGER id PK
        TEXT wallet_id FK
        TEXT tx_type
        TEXT tx_subtype
        TEXT chain
        TEXT on_chain_tx_hash
        INTEGER amount_rj
        INTEGER balance_after_rj
        TEXT key_id
        TEXT tool_name
        INTEGER gas_units
        TEXT created_at
    }

    api_keys {
        TEXT key_id PK
        TEXT wallet_id FK
        BLOB public_key
        INTEGER spending_limit_rj
        INTEGER spent_rj
        TEXT scope
        TEXT purpose
        TEXT rate_limit_json
        TEXT privacy_mode
        TEXT preferred_chain
        TEXT expires_at
        TEXT issued_at
        TEXT revoked_at
        TEXT created_at
    }

    encumbrances {
        TEXT key_id PK_FK
        TEXT wallet_id FK
        INTEGER amount_rj
        INTEGER consumed_rj
        TEXT status
        TEXT created_at
        TEXT released_at
    }

    deposit_addresses {
        TEXT wallet_id PK_FK
        TEXT chain PK
        TEXT address UK
        INTEGER derivation_index PK
        TEXT privacy_mode
        TEXT created_at
    }

    deposit_references {
        TEXT reference PK
        TEXT wallet_id FK
        TEXT chain
        TEXT expires_at
        INTEGER spent
        TEXT created_at
    }

    audit_log {
        TEXT id PK
        TEXT timestamp
        TEXT actor_webid
        TEXT action
        TEXT resource
        TEXT outcome
        TEXT details
        TEXT ip_address
        TEXT created_at
    }

    cns_variety_checkpoint {
        TEXT domain PK
        INTEGER variety_count
        TEXT last_updated
        INTEGER threshold
    }

    cns_alerts {
        TEXT id PK
        TEXT timestamp
        TEXT alert_type
        TEXT severity
        TEXT domain
        TEXT message
        INTEGER resolved
        TEXT resolved_at
    }

    loop_cursors {
        TEXT key PK
        INTEGER value
        TEXT updated_at
    }

    kata_history {
        INTEGER id PK
        TEXT agent_name
        TEXT date
        TEXT kata_type
        TEXT practice_name
        INTEGER steps_completed
        INTEGER gas_consumed
        TEXT created_at
    }

    pod_meta {
        TEXT key PK
        TEXT value
    }

    human_users ||--o{ replicant_identities : "user_id"
    human_users ||--o{ user_sessions : "user_id"
    human_users ||--o{ invites : "created_by / accepted_user_id"
    replicant_identities ||--o{ user_sessions : "replicant_name"
    goals ||--o{ goal_criteria : "goal_id"
    goals ||--o{ goal_artifacts : "goal_id"
    wallet_balances ||--o{ wallet_transactions : "wallet_id"
    wallet_balances ||--o{ api_keys : "wallet_id"
    wallet_balances ||--o{ deposit_addresses : "wallet_id"
    wallet_balances ||--o{ deposit_references : "wallet_id"
    api_keys ||--|| encumbrances : "key_id"
    encumbrances }o--|| wallet_balances : "wallet_id"
```

## Node-to-Code Mapping

| Table | Crate | Source File |
|-------|-------|-------------|
| `hmems` | `hkask-storage-core` | `src/sql/schema.sql` |
| `embeddings` | `hkask-storage-core` | `src/sql/schema.sql` |
| `vec_embeddings` | `hkask-storage-core` | `src/sql/schema.sql` |
| `nu_events` | `hkask-storage-core` | `src/sql/schema.sql` |
| `audit_log` | `hkask-storage-core` | `src/sql/schema.sql` |
| `cns_variety_checkpoint` | `hkask-storage-core` | `src/sql/schema.sql` |
| `cns_alerts` | `hkask-storage-core` | `src/sql/schema.sql` |
| `agent_registry` | `hkask-storage` | `src/agent_registry.rs` |
| `goals` | `hkask-storage` | `src/goals.rs` |
| `goal_criteria` | `hkask-storage` | `src/sql/schema.sql` |
| `goal_artifacts` | `hkask-storage` | `src/sql/schema.sql` |
| `consent_records` | `hkask-storage::consent_store` | `src/consent_store.rs` |
| `quarantined_goals` | `hkask-storage` | `src/goals.rs` |
| `loop_cursors` | `hkask-storage-core` | `src/sql/schema.sql` |
| `human_users` | `hkask-storage-core` | `src/sql/users.sql` |
| `replicant_identities` | `hkask-storage-core` | `src/sql/users.sql` |
| `user_sessions` | `hkask-storage-core` | `src/sql/users.sql` |
| `invites` | `hkask-storage-core` | `src/sql/users.sql` |
| `wallet_balances` | `hkask-storage` | `src/wallet/mod.rs` |
| `wallet_transactions` | `hkask-storage` | `src/wallet/mod.rs` |
| `api_keys` | `hkask-storage` | `src/wallet/mod.rs` |
| `encumbrances` | `hkask-storage` | `src/wallet/mod.rs` |
| `deposit_addresses` | `hkask-storage` | `src/wallet/mod.rs` |
| `deposit_references` | `hkask-storage` | `src/wallet/mod.rs` |
| `kata_history` | `hkask-storage::kata` | `src/kata.rs` |
| `pod_meta` | `hkask-storage-core` | `src/sql/schema.sql` |

### Relationships

| Relationship | Cardinality | On |
|-------------|-------------|-----|
| `human_users` → `replicant_identities` | 1:N | `user_id` |
| `human_users` → `user_sessions` | 1:N | `user_id` |
| `human_users` → `invites` | 1:N | `created_by`, `accepted_user_id` |
| `replicant_identities` → `user_sessions` | 1:N | `replicant_name` |
| `goals` → `goal_criteria` | 1:N | `goal_id` |
| `goals` → `goal_artifacts` | 1:N | `goal_id` |
| `wallet_balances` → `wallet_transactions` | 1:N | `wallet_id` |
| `wallet_balances` → `api_keys` | 1:N | `wallet_id` |
| `wallet_balances` → `deposit_addresses` | 1:N | `wallet_id` |
| `wallet_balances` → `deposit_references` | 1:N | `wallet_id` |
| `api_keys` → `encumbrances` | 1:1 | `key_id` |
| `encumbrances` → `wallet_balances` | N:1 | `wallet_id` |
