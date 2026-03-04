pub const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    name TEXT NOT NULL,
    cwd TEXT NOT NULL,
    shell TEXT NOT NULL,
    status TEXT NOT NULL,
    persist_history INTEGER NOT NULL DEFAULT 0,
    last_seq INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_profile_id ON sessions(profile_id);

CREATE TABLE IF NOT EXISTS scrollback_chunks (
    session_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    line_count INTEGER NOT NULL,
    ts INTEGER NOT NULL,
    PRIMARY KEY(session_id, seq),
    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_scrollback_session_seq ON scrollback_chunks(session_id, seq DESC);

CREATE TABLE IF NOT EXISTS session_explorer_state (
    session_id TEXT PRIMARY KEY,
    root_path TEXT NOT NULL,
    current_dir TEXT NOT NULL DEFAULT '',
    selected_path TEXT,
    open_file_path TEXT,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_session_explorer_updated ON session_explorer_state(updated_at DESC);

CREATE TABLE IF NOT EXISTS app_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
"#;
