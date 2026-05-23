-- Migration 002: Goals table for user intention tracking
-- Goals are the minimal coordination substrate for multi-agent collaboration
-- Shared language (hLexicon) + Shared goals = productive cooperation

CREATE TABLE IF NOT EXISTS goals (
    id TEXT PRIMARY KEY,
    webid TEXT NOT NULL,
    text TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',
    visibility TEXT NOT NULL DEFAULT 'private',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    parent_goal_id TEXT,
    depth INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (webid) REFERENCES webids(id),
    FOREIGN KEY (parent_goal_id) REFERENCES goals(id)
);

-- Goal criteria: completion conditions (LLM-judged, not deterministic)
CREATE TABLE IF NOT EXISTS goal_criteria (
    id TEXT PRIMARY KEY,
    goal_id TEXT NOT NULL,
    type TEXT NOT NULL,
    description TEXT NOT NULL,
    satisfied INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (goal_id) REFERENCES goals(id) ON DELETE CASCADE
);

-- Goal artifacts: outputs produced while working toward goal
CREATE TABLE IF NOT EXISTS goal_artifacts (
    id TEXT PRIMARY KEY,
    goal_id TEXT NOT NULL,
    artifact_ref TEXT NOT NULL,
    artifact_type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (goal_id) REFERENCES goals(id) ON DELETE CASCADE
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_goals_webid ON goals(webid);
CREATE INDEX IF NOT EXISTS idx_goals_state ON goals(state);
CREATE INDEX IF NOT EXISTS idx_goals_visibility ON goals(visibility);
CREATE INDEX IF NOT EXISTS idx_goals_parent ON goals(parent_goal_id);
CREATE INDEX IF NOT EXISTS idx_goal_criteria_goal ON goal_criteria(goal_id);
CREATE INDEX IF NOT EXISTS idx_goal_artifacts_goal ON goal_artifacts(goal_id);

-- SQLCipher: encrypt goals table (user sovereignty)
-- This is applied at connection time, not in migration
