-- Human users (contact info for recovery only)
CREATE TABLE IF NOT EXISTS human_users (
    user_id TEXT PRIMARY KEY,
    
    -- Human's contact info (encrypted)
    email_enc BLOB NOT NULL,
    phone_enc BLOB,
    
    -- Auth credentials
    passphrase_hash TEXT NOT NULL,
    salt TEXT NOT NULL,
    master_salt TEXT NOT NULL,
    
    -- Metadata
    created_at TEXT NOT NULL,
    last_active TEXT
);

-- Replicant identities (user logs in AS a replicant)
CREATE TABLE IF NOT EXISTS replicant_identities (
    replicant_name TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    replicant_webid TEXT UNIQUE NOT NULL,
    first_name_enc BLOB NOT NULL,
    last_name_enc BLOB NOT NULL,
    email_enc BLOB NOT NULL,
    phone_enc BLOB,
    persona_yaml TEXT,
    is_primary INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    last_login TEXT,
    FOREIGN KEY (user_id) REFERENCES human_users(user_id)
);

-- Active sessions
CREATE TABLE IF NOT EXISTS user_sessions (
    session_id TEXT PRIMARY KEY,
    replicant_name TEXT NOT NULL,
    replicant_webid TEXT NOT NULL,
    user_id TEXT NOT NULL,
    session_key_salt TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_active TEXT NOT NULL,
    FOREIGN KEY (replicant_name) REFERENCES replicant_identities(replicant_name),
    FOREIGN KEY (user_id) REFERENCES human_users(user_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_replicant_identities_user ON replicant_identities(user_id);
CREATE INDEX IF NOT EXISTS idx_replicant_identities_webid ON replicant_identities(replicant_webid);
CREATE INDEX IF NOT EXISTS idx_user_sessions_user ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_replicant ON user_sessions(replicant_name);
