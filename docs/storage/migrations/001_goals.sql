-- hKask Goal Primitive — Database Schema
-- Migration: 001_goals.sql
-- Purpose: Persistent storage for goals, completion criteria, subgoals, verifications, and audit log

-- Goals table: Core goal entity storage
CREATE TABLE IF NOT EXISTS goals (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL,
    owner_webid     TEXT NOT NULL,
    goal_text       TEXT NOT NULL,
    template_ref    TEXT,
    commitment_level TEXT NOT NULL DEFAULT 'commit',
    flow_json       TEXT,
    state           TEXT NOT NULL DEFAULT 'active',
    turns_used      INTEGER DEFAULT 0,
    energy_budget   INTEGER,
    energy_used     INTEGER DEFAULT 0,
    max_turns       INTEGER DEFAULT 20,
    created_at      INTEGER NOT NULL,
    last_turn_at    INTEGER,
    completed_at    INTEGER,
    blocked_reason  TEXT,
    paused_reason   TEXT,
    visibility      TEXT NOT NULL DEFAULT 'private'
);

CREATE INDEX IF NOT EXISTS idx_goals_session ON goals(session_id);
CREATE INDEX IF NOT EXISTS idx_goals_owner ON goals(owner_webid);
CREATE INDEX IF NOT EXISTS idx_goals_state ON goals(state);

-- Goal completion criteria: Verification conditions
CREATE TABLE IF NOT EXISTS goal_completion_criteria (
    goal_id         TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    ordinal         INTEGER NOT NULL,
    criterion_type  TEXT NOT NULL,
    criterion_data  TEXT NOT NULL,
    satisfied       BOOLEAN DEFAULT FALSE,
    
    PRIMARY KEY (goal_id, ordinal)
);

-- Goal subgoals: User-added criteria mid-loop
CREATE TABLE IF NOT EXISTS goal_subgoals (
    goal_id     TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    ordinal     INTEGER NOT NULL,
    text        TEXT NOT NULL,
    satisfied   BOOLEAN DEFAULT FALSE,
    
    PRIMARY KEY (goal_id, ordinal)
);

-- Goal verifications: Audit trail of verification attempts
CREATE TABLE IF NOT EXISTS goal_verifications (
    id              TEXT PRIMARY KEY,
    goal_id         TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    nu_event_id     TEXT,
    verdict         TEXT NOT NULL,
    reason          TEXT,
    confidence      REAL,
    verified_at     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_verifications_goal ON goal_verifications(goal_id);

-- Goal audit log: Immutable record of all goal operations
CREATE TABLE IF NOT EXISTS goal_audit_log (
    id              TEXT PRIMARY KEY,
    goal_id         TEXT NOT NULL REFERENCES goals(id),
    actor_webid     TEXT NOT NULL,
    action          TEXT NOT NULL,
    old_state       TEXT,
    new_state       TEXT,
    capability_id   TEXT,
    nu_event_id     TEXT,
    created_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_goal ON goal_audit_log(goal_id);
CREATE INDEX IF NOT EXISTS idx_audit_actor ON goal_audit_log(actor_webid);
CREATE INDEX IF NOT EXISTS idx_audit_created ON goal_audit_log(created_at);

-- Goal capabilities: OCAP token storage (optional, can also be in-memory)
CREATE TABLE IF NOT EXISTS goal_capabilities (
    id                  TEXT PRIMARY KEY,
    goal_id             TEXT NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    owner_webid         TEXT NOT NULL,
    holder_webid        TEXT NOT NULL,
    allowed_actions     TEXT NOT NULL,
    attenuation_level   INTEGER NOT NULL DEFAULT 0,
    max_attenuation     INTEGER NOT NULL DEFAULT 7,
    expiration          INTEGER NOT NULL,
    hmac_signature      TEXT NOT NULL,
    created_at          INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_capabilities_goal ON goal_capabilities(goal_id);
CREATE INDEX IF NOT EXISTS idx_capabilities_holder ON goal_capabilities(holder_webid);
