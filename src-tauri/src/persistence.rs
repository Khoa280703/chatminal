use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use crate::models::SessionStatus;

const DEFAULT_PROFILE_NAME: &str = "Default";
const ACTIVE_PROFILE_KEY: &str = "active_profile_id";
const LEGACY_ACTIVE_SESSION_KEY: &str = "active_session_id";

#[derive(Debug, Clone)]
pub struct PersistedProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct PersistedSession {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub shell: String,
    pub persist_history: bool,
    pub last_seq: u64,
    pub preview: String,
}

#[derive(Debug, Clone)]
pub struct PersistedSessionExplorerState {
    pub session_id: String,
    pub root_path: String,
    pub current_dir: String,
    pub selected_path: Option<String>,
    pub open_file_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PersistedWorkspace {
    pub active_profile_id: Option<String>,
    pub sessions: Vec<PersistedSession>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub shell: String,
    pub status: SessionStatus,
    pub persist_history: bool,
    pub last_seq: u64,
}

#[derive(Debug, Clone)]
pub struct HistoryChunk {
    pub session_id: String,
    pub seq: u64,
    pub chunk_text: String,
    pub line_count: u64,
    pub ts: u64,
}

#[derive(Debug, Clone)]
pub struct Persistence {
    db_path: PathBuf,
}

impl Persistence {
    pub fn initialize() -> Result<Self, String> {
        let db_path = db_path()?;
        let persistence = Self { db_path };
        persistence.init_schema()?;
        Ok(persistence)
    }

    pub fn list_profiles(&self) -> Result<Vec<PersistedProfile>, String> {
        let conn = self.open_connection()?;
        load_profiles(&conn)
    }

    pub fn create_profile(&self, raw_name: &str) -> Result<PersistedProfile, String> {
        let conn = self.open_connection()?;
        let name = validate_profile_name(raw_name)?;
        let profile_id = Uuid::new_v4().to_string();
        let now = now_ts_millis() as i64;

        conn.execute(
            r#"
            INSERT INTO profiles (id, name, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![profile_id, name, now, now],
        )
        .map_err(|err| format!("create profile failed: {err}"))?;

        Ok(PersistedProfile {
            id: profile_id,
            name,
        })
    }

    pub fn rename_profile(&self, profile_id: &str, raw_name: &str) -> Result<(), String> {
        let conn = self.open_connection()?;
        let name = validate_profile_name(raw_name)?;
        let affected = conn
            .execute(
                "UPDATE profiles SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now_ts_millis() as i64, profile_id],
            )
            .map_err(|err| format!("rename profile failed: {err}"))?;

        if affected == 0 {
            return Err("profile not found".to_string());
        }

        Ok(())
    }

    pub fn delete_profile(&self, profile_id: &str) -> Result<(), String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open delete profile transaction failed: {err}"))?;

        let profile_exists = tx
            .query_row(
                "SELECT COUNT(1) FROM profiles WHERE id = ?1",
                params![profile_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|err| format!("validate profile failed: {err}"))?;
        if profile_exists == 0 {
            return Err("profile not found".to_string());
        }

        let profile_count = tx
            .query_row("SELECT COUNT(1) FROM profiles", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|err| format!("count profiles failed: {err}"))?;
        if profile_count <= 1 {
            return Err("cannot delete the last profile".to_string());
        }

        let replacement_profile_id = tx
            .query_row(
                "SELECT id FROM profiles WHERE id <> ?1 ORDER BY updated_at DESC, created_at ASC, id ASC LIMIT 1",
                params![profile_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| format!("find replacement profile failed: {err}"))?
            .ok_or_else(|| "cannot resolve replacement profile".to_string())?;

        tx.execute(
            "DELETE FROM app_state WHERE key = ?1",
            params![active_session_key(profile_id)],
        )
        .map_err(|err| format!("clear deleted profile active session failed: {err}"))?;

        let active_profile_id = tx
            .query_row(
                "SELECT value FROM app_state WHERE key = ?1",
                params![ACTIVE_PROFILE_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| format!("load active profile before delete failed: {err}"))?;

        if active_profile_id.as_deref() == Some(profile_id) {
            tx.execute(
                r#"
                INSERT INTO app_state (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                "#,
                params![ACTIVE_PROFILE_KEY, replacement_profile_id],
            )
            .map_err(|err| format!("switch active profile during delete failed: {err}"))?;
        }

        tx.execute(
            "DELETE FROM scrollback WHERE session_id IN (SELECT id FROM sessions WHERE profile_id = ?1)",
            params![profile_id],
        )
        .map_err(|err| format!("delete profile scrollback failed: {err}"))?;

        tx.execute(
            "DELETE FROM sessions WHERE profile_id = ?1",
            params![profile_id],
        )
        .map_err(|err| format!("delete profile sessions failed: {err}"))?;

        tx.execute("DELETE FROM profiles WHERE id = ?1", params![profile_id])
            .map_err(|err| format!("delete profile failed: {err}"))?;

        tx.commit()
            .map_err(|err| format!("commit delete profile failed: {err}"))?;
        Ok(())
    }

    pub fn latest_persisted_seq(&self, session_id: &str) -> Result<Option<u64>, String> {
        let conn = self.open_connection()?;
        conn.query_row(
            "SELECT MAX(seq) FROM scrollback WHERE session_id = ?1",
            params![session_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .map(|value| value.map(|seq| seq.max(0) as u64))
        .map_err(|err| format!("query latest persisted seq failed: {err}"))
    }

    pub fn load_session_preview(
        &self,
        session_id: &str,
        preview_lines: usize,
    ) -> Result<String, String> {
        let conn = self.open_connection()?;
        load_preview_for_session(&conn, session_id, preview_lines)
    }

    pub fn set_active_profile(&self, profile_id: Option<&str>) -> Result<(), String> {
        let conn = self.open_connection()?;
        match profile_id {
            Some(value) => {
                conn.execute(
                    r#"
                    INSERT INTO app_state (key, value)
                    VALUES (?1, ?2)
                    ON CONFLICT(key) DO UPDATE SET value = excluded.value
                    "#,
                    params![ACTIVE_PROFILE_KEY, value],
                )
                .map_err(|err| format!("set active profile failed: {err}"))?;
            }
            None => {
                conn.execute(
                    "DELETE FROM app_state WHERE key = ?1",
                    params![ACTIVE_PROFILE_KEY],
                )
                .map_err(|err| format!("clear active profile failed: {err}"))?;
            }
        }
        Ok(())
    }

    pub fn get_bool_state(&self, key: &str, default: bool) -> Result<bool, String> {
        let conn = self.open_connection()?;
        let value = conn
            .query_row(
                "SELECT value FROM app_state WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| format!("load app state failed: {err}"))?;

        let Some(value) = value else {
            return Ok(default);
        };

        let normalized = value.trim().to_ascii_lowercase();
        let parsed = match normalized.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        };

        Ok(parsed.unwrap_or(default))
    }

    pub fn set_bool_state(&self, key: &str, value: bool) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            r#"
            INSERT INTO app_state (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![key, if value { "1" } else { "0" }],
        )
        .map_err(|err| format!("set app state failed: {err}"))?;
        Ok(())
    }

    pub fn upsert_session(&self, record: &SessionRecord) -> Result<(), String> {
        let conn = self.open_connection()?;
        let now = now_ts_millis() as i64;

        conn.execute(
            r#"
            INSERT INTO sessions (
                id, profile_id, name, cwd, shell, status, persist_history, last_seq, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                profile_id = excluded.profile_id,
                name = excluded.name,
                cwd = excluded.cwd,
                shell = excluded.shell,
                status = excluded.status,
                persist_history = excluded.persist_history,
                last_seq = excluded.last_seq,
                updated_at = excluded.updated_at
            "#,
            params![
                &record.id,
                &record.profile_id,
                &record.name,
                &record.cwd,
                &record.shell,
                status_to_db(&record.status),
                bool_to_db(record.persist_history),
                record.last_seq as i64,
                now
            ],
        )
        .map_err(|err| format!("upsert session failed: {err}"))?;

        Ok(())
    }

    pub fn set_session_status(
        &self,
        session_id: &str,
        status: SessionStatus,
    ) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status_to_db(&status), now_ts_millis() as i64, session_id],
        )
        .map_err(|err| format!("set session status failed: {err}"))?;
        Ok(())
    }

    pub fn set_session_cwd(&self, session_id: &str, cwd: &str) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE sessions SET cwd = ?1, updated_at = ?2 WHERE id = ?3",
            params![cwd, now_ts_millis() as i64, session_id],
        )
        .map_err(|err| format!("set session cwd failed: {err}"))?;
        Ok(())
    }

    pub fn set_active_session(
        &self,
        profile_id: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.open_connection()?;
        let key = active_session_key(profile_id);
        match session_id {
            Some(value) => {
                conn.execute(
                    r#"
                    INSERT INTO app_state (key, value)
                    VALUES (?1, ?2)
                    ON CONFLICT(key) DO UPDATE SET value = excluded.value
                    "#,
                    params![key, value],
                )
                .map_err(|err| format!("set active session failed: {err}"))?;
            }
            None => {
                conn.execute("DELETE FROM app_state WHERE key = ?1", params![key])
                    .map_err(|err| format!("clear active session failed: {err}"))?;
            }
        }
        Ok(())
    }

    pub fn load_workspace(&self, preview_lines: usize) -> Result<PersistedWorkspace, String> {
        let conn = self.open_connection()?;

        let profiles = load_profiles(&conn)?;
        if profiles.is_empty() {
            return Ok(PersistedWorkspace {
                active_profile_id: None,
                sessions: Vec::new(),
                active_session_id: None,
            });
        }

        let active_profile_id = select_active_profile_id(&conn, &profiles)?;
        let sessions = load_sessions_for_profile(&conn, &active_profile_id, preview_lines)?;

        let mut active_session_id = load_active_session_for_profile(&conn, &active_profile_id)?;
        if let Some(candidate) = active_session_id.as_deref()
            && !sessions.iter().any(|session| session.id == candidate)
        {
            active_session_id = None;
        }

        if active_session_id.is_none() {
            active_session_id = sessions.first().map(|session| session.id.clone());
        }

        self.set_active_profile(Some(&active_profile_id))?;
        self.set_active_session(&active_profile_id, active_session_id.as_deref())?;

        Ok(PersistedWorkspace {
            active_profile_id: Some(active_profile_id),
            sessions,
            active_session_id,
        })
    }

    pub fn append_history_batch(
        &self,
        chunks: &[HistoryChunk],
        max_lines_per_session: usize,
        auto_delete_after_days: u32,
    ) -> Result<(), String> {
        if chunks.is_empty() {
            return Ok(());
        }

        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open history transaction failed: {err}"))?;

        let mut touched = HashSet::new();
        let mut last_seq_by_session: HashMap<String, u64> = HashMap::new();

        {
            let mut insert_stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO scrollback (session_id, seq, chunk_text, line_count, ts)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )
                .map_err(|err| format!("prepare history insert failed: {err}"))?;

            for chunk in chunks {
                insert_stmt
                    .execute(params![
                        &chunk.session_id,
                        chunk.seq as i64,
                        &chunk.chunk_text,
                        chunk.line_count as i64,
                        chunk.ts as i64
                    ])
                    .map_err(|err| format!("insert history chunk failed: {err}"))?;

                touched.insert(chunk.session_id.clone());
                last_seq_by_session
                    .entry(chunk.session_id.clone())
                    .and_modify(|current| *current = (*current).max(chunk.seq))
                    .or_insert(chunk.seq);
            }
        }

        {
            let mut update_stmt = tx
                .prepare("UPDATE sessions SET last_seq = ?1, updated_at = ?2 WHERE id = ?3")
                .map_err(|err| format!("prepare session seq update failed: {err}"))?;
            let now = now_ts_millis() as i64;
            for (session_id, seq) in &last_seq_by_session {
                update_stmt
                    .execute(params![*seq as i64, now, session_id])
                    .map_err(|err| format!("update last_seq failed: {err}"))?;
            }
        }

        for session_id in &touched {
            trim_session_lines(&tx, session_id, max_lines_per_session)?;
        }
        trim_ttl(&tx, auto_delete_after_days)?;

        tx.commit()
            .map_err(|err| format!("commit history transaction failed: {err}"))?;

        Ok(())
    }

    pub fn clear_session_history(&self, session_id: &str) -> Result<(), String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open clear session history transaction failed: {err}"))?;

        tx.execute(
            "DELETE FROM scrollback WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|err| format!("delete session history failed: {err}"))?;
        tx.execute(
            "UPDATE sessions SET last_seq = 0, updated_at = ?1 WHERE id = ?2",
            params![now_ts_millis() as i64, session_id],
        )
        .map_err(|err| format!("reset session seq failed: {err}"))?;

        tx.commit()
            .map_err(|err| format!("commit clear session history failed: {err}"))?;
        Ok(())
    }

    pub fn clear_all_history(&self) -> Result<(), String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open clear all history transaction failed: {err}"))?;
        tx.execute("DELETE FROM scrollback", [])
            .map_err(|err| format!("delete all history failed: {err}"))?;
        tx.execute(
            "UPDATE sessions SET last_seq = 0, updated_at = ?1",
            params![now_ts_millis() as i64],
        )
        .map_err(|err| format!("reset all session seq failed: {err}"))?;
        tx.commit()
            .map_err(|err| format!("commit clear all history failed: {err}"))?;
        Ok(())
    }

    pub fn get_session_explorer_state(
        &self,
        session_id: &str,
    ) -> Result<Option<PersistedSessionExplorerState>, String> {
        let conn = self.open_connection()?;
        conn.query_row(
            r#"
            SELECT session_id, root_path, current_dir, selected_path, open_file_path
            FROM session_explorer_state
            WHERE session_id = ?1
            "#,
            params![session_id],
            |row| {
                Ok(PersistedSessionExplorerState {
                    session_id: row.get::<_, String>(0)?,
                    root_path: row.get::<_, String>(1)?,
                    current_dir: row.get::<_, String>(2)?,
                    selected_path: row.get::<_, Option<String>>(3)?,
                    open_file_path: row.get::<_, Option<String>>(4)?,
                })
            },
        )
        .optional()
        .map_err(|err| format!("load session explorer state failed: {err}"))
    }

    pub fn set_session_explorer_root(
        &self,
        session_id: &str,
        root_path: &str,
    ) -> Result<PersistedSessionExplorerState, String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open session explorer root transaction failed: {err}"))?;

        let exists = tx
            .query_row(
                "SELECT COUNT(1) FROM sessions WHERE id = ?1",
                params![session_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|err| format!("validate session explorer root failed: {err}"))?;

        if exists == 0 {
            return Err("session not found".to_string());
        }

        tx.execute(
            r#"
            INSERT INTO session_explorer_state (
                session_id, root_path, current_dir, selected_path, open_file_path, updated_at
            ) VALUES (?1, ?2, '', NULL, NULL, ?3)
            ON CONFLICT(session_id) DO UPDATE SET
                root_path = excluded.root_path,
                current_dir = '',
                selected_path = NULL,
                open_file_path = NULL,
                updated_at = excluded.updated_at
            "#,
            params![session_id, root_path, now_ts_millis() as i64],
        )
        .map_err(|err| format!("set session explorer root failed: {err}"))?;

        let state = tx
            .query_row(
                r#"
                SELECT session_id, root_path, current_dir, selected_path, open_file_path
                FROM session_explorer_state
                WHERE session_id = ?1
                "#,
                params![session_id],
                |row| {
                    Ok(PersistedSessionExplorerState {
                        session_id: row.get::<_, String>(0)?,
                        root_path: row.get::<_, String>(1)?,
                        current_dir: row.get::<_, String>(2)?,
                        selected_path: row.get::<_, Option<String>>(3)?,
                        open_file_path: row.get::<_, Option<String>>(4)?,
                    })
                },
            )
            .map_err(|err| format!("reload session explorer state failed: {err}"))?;

        tx.commit()
            .map_err(|err| format!("commit session explorer root failed: {err}"))?;
        Ok(state)
    }

    pub fn update_session_explorer_state(
        &self,
        session_id: &str,
        current_dir: &str,
        selected_path: Option<&str>,
        open_file_path: Option<&str>,
    ) -> Result<PersistedSessionExplorerState, String> {
        let conn = self.open_connection()?;
        let affected = conn
            .execute(
                r#"
                UPDATE session_explorer_state
                SET current_dir = ?1,
                    selected_path = ?2,
                    open_file_path = ?3,
                    updated_at = ?4
                WHERE session_id = ?5
                "#,
                params![
                    current_dir,
                    selected_path,
                    open_file_path,
                    now_ts_millis() as i64,
                    session_id
                ],
            )
            .map_err(|err| format!("update session explorer state failed: {err}"))?;

        if affected == 0 {
            return Err("session explorer root is not set".to_string());
        }

        self.get_session_explorer_state(session_id)?
            .ok_or_else(|| "session explorer state disappeared".to_string())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
            .map_err(|err| format!("delete session failed: {err}"))?;
        Ok(())
    }

    fn init_schema(&self) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute_batch(
            r#"
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
                updated_at INTEGER NOT NULL,
                FOREIGN KEY(profile_id) REFERENCES profiles(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS scrollback (
                session_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                chunk_text TEXT NOT NULL,
                line_count INTEGER NOT NULL,
                ts INTEGER NOT NULL,
                PRIMARY KEY (session_id, seq),
                FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS app_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS session_explorer_state (
                session_id TEXT PRIMARY KEY,
                root_path TEXT NOT NULL,
                current_dir TEXT NOT NULL DEFAULT '',
                selected_path TEXT,
                open_file_path TEXT,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_profiles_updated_at
              ON profiles(updated_at DESC, created_at ASC);

            CREATE INDEX IF NOT EXISTS idx_scrollback_session_seq
              ON scrollback(session_id, seq DESC);

            CREATE INDEX IF NOT EXISTS idx_scrollback_session_ts
              ON scrollback(session_id, ts);

            CREATE INDEX IF NOT EXISTS idx_session_explorer_updated
              ON session_explorer_state(updated_at DESC);
            "#,
        )
        .map_err(|err| format!("initialize schema failed: {err}"))?;

        ensure_profile_column(&conn)?;
        let default_profile_id = ensure_default_profile(&conn)?;
        backfill_profile_id(&conn, &default_profile_id)?;
        ensure_session_profile_index(&conn)?;
        migrate_legacy_active_session_key(&conn, &default_profile_id)?;
        ensure_active_profile_key(&conn, &default_profile_id)?;

        Ok(())
    }

    fn open_connection(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|err| format!("open database failed: {err}"))?;

        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA busy_timeout = 5000;
            "#,
        )
        .map_err(|err| format!("configure database failed: {err}"))?;

        Ok(conn)
    }
}

fn load_profiles(conn: &Connection) -> Result<Vec<PersistedProfile>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM profiles ORDER BY updated_at DESC, created_at ASC, id ASC")
        .map_err(|err| format!("prepare profiles query failed: {err}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(PersistedProfile {
                id: row.get::<_, String>(0)?,
                name: row.get::<_, String>(1)?,
            })
        })
        .map_err(|err| format!("query profiles failed: {err}"))?;

    let mut profiles = Vec::new();
    for row in rows {
        profiles.push(row.map_err(|err| format!("read profile row failed: {err}"))?);
    }

    Ok(profiles)
}

fn load_sessions_for_profile(
    conn: &Connection,
    profile_id: &str,
    preview_lines: usize,
) -> Result<Vec<PersistedSession>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, profile_id, name, cwd, shell, persist_history, last_seq
             FROM sessions
             WHERE profile_id = ?1
             ORDER BY updated_at DESC, id ASC",
        )
        .map_err(|err| format!("prepare workspace query failed: {err}"))?;

    let rows = stmt
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|err| format!("query workspace failed: {err}"))?;

    let mut sessions = Vec::new();
    for row in rows {
        let (id, record_profile_id, name, cwd, shell, persist_history, last_seq) =
            row.map_err(|err| format!("read workspace row failed: {err}"))?;

        let preview = load_preview_for_session(conn, &id, preview_lines)?;
        sessions.push(PersistedSession {
            id,
            profile_id: record_profile_id,
            name,
            cwd,
            shell,
            persist_history: persist_history != 0,
            last_seq: last_seq.max(0) as u64,
            preview,
        });
    }

    Ok(sessions)
}

fn load_preview_for_session(
    conn: &Connection,
    session_id: &str,
    preview_lines: usize,
) -> Result<String, String> {
    let limit_chunks = (preview_lines.max(10) * 3).min(10_000);
    let mut stmt = conn
        .prepare(
            "SELECT chunk_text
             FROM scrollback
             WHERE session_id = ?1
             ORDER BY seq DESC
             LIMIT ?2",
        )
        .map_err(|err| format!("prepare session preview query failed: {err}"))?;

    let rows = stmt
        .query_map(params![session_id, limit_chunks as i64], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|err| format!("query session preview failed: {err}"))?;

    let mut chunks = Vec::new();
    for row in rows {
        chunks.push(row.map_err(|err| format!("read preview row failed: {err}"))?);
    }

    chunks.reverse();
    let content = chunks.join("");
    Ok(tail_lines(&content, preview_lines.max(1)))
}

fn select_active_profile_id(
    conn: &Connection,
    profiles: &[PersistedProfile],
) -> Result<String, String> {
    let stored = conn
        .query_row(
            "SELECT value FROM app_state WHERE key = ?1",
            params![ACTIVE_PROFILE_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("load active profile failed: {err}"))?;

    if let Some(profile_id) = stored
        && profiles.iter().any(|profile| profile.id == profile_id)
    {
        return Ok(profile_id);
    }

    profiles
        .first()
        .map(|profile| profile.id.clone())
        .ok_or_else(|| "no profile found".to_string())
}

fn load_active_session_for_profile(
    conn: &Connection,
    profile_id: &str,
) -> Result<Option<String>, String> {
    let profile_key = active_session_key(profile_id);
    let stored = conn
        .query_row(
            "SELECT value FROM app_state WHERE key = ?1",
            params![profile_key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("load active session failed: {err}"))?;

    if stored.is_some() {
        return Ok(stored);
    }

    let legacy = conn
        .query_row(
            "SELECT value FROM app_state WHERE key = ?1",
            params![LEGACY_ACTIVE_SESSION_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("load legacy active session failed: {err}"))?;

    let Some(session_id) = legacy else {
        return Ok(None);
    };

    if !session_exists_in_profile(conn, profile_id, &session_id)? {
        return Ok(None);
    }

    conn.execute(
        r#"
        INSERT INTO app_state (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        params![active_session_key(profile_id), session_id],
    )
    .map_err(|err| format!("persist migrated active session failed: {err}"))?;

    Ok(Some(session_id))
}

fn session_exists_in_profile(
    conn: &Connection,
    profile_id: &str,
    session_id: &str,
) -> Result<bool, String> {
    let count = conn
        .query_row(
            "SELECT COUNT(1) FROM sessions WHERE id = ?1 AND profile_id = ?2",
            params![session_id, profile_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|err| format!("validate profile session failed: {err}"))?;

    Ok(count > 0)
}

fn active_session_key(profile_id: &str) -> String {
    format!("active_session_id:{profile_id}")
}

fn ensure_profile_column(conn: &Connection) -> Result<(), String> {
    let mut has_profile_id = false;
    let mut stmt = conn
        .prepare("PRAGMA table_info(sessions)")
        .map_err(|err| format!("prepare sessions schema query failed: {err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| format!("query sessions schema failed: {err}"))?;

    for row in rows {
        let column = row.map_err(|err| format!("read sessions schema row failed: {err}"))?;
        if column == "profile_id" {
            has_profile_id = true;
            break;
        }
    }

    if !has_profile_id {
        conn.execute("ALTER TABLE sessions ADD COLUMN profile_id TEXT", [])
            .map_err(|err| format!("add sessions.profile_id failed: {err}"))?;
    }

    Ok(())
}

fn ensure_session_profile_index(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_profile_updated ON sessions(profile_id, updated_at DESC, id ASC)",
        [],
    )
    .map_err(|err| format!("create sessions profile index failed: {err}"))?;
    Ok(())
}

fn ensure_default_profile(conn: &Connection) -> Result<String, String> {
    let existing = conn
        .query_row(
            "SELECT id FROM profiles ORDER BY created_at ASC, id ASC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("load default profile failed: {err}"))?;

    if let Some(profile_id) = existing {
        return Ok(profile_id);
    }

    let profile_id = Uuid::new_v4().to_string();
    let now = now_ts_millis() as i64;
    conn.execute(
        "INSERT INTO profiles (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![&profile_id, DEFAULT_PROFILE_NAME, now, now],
    )
    .map_err(|err| format!("insert default profile failed: {err}"))?;

    Ok(profile_id)
}

fn backfill_profile_id(conn: &Connection, default_profile_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE sessions SET profile_id = ?1 WHERE profile_id IS NULL OR profile_id = ''",
        params![default_profile_id],
    )
    .map_err(|err| format!("backfill session profile_id failed: {err}"))?;
    Ok(())
}

fn migrate_legacy_active_session_key(
    conn: &Connection,
    default_profile_id: &str,
) -> Result<(), String> {
    let legacy = conn
        .query_row(
            "SELECT value FROM app_state WHERE key = ?1",
            params![LEGACY_ACTIVE_SESSION_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("load legacy active session failed: {err}"))?;

    let Some(session_id) = legacy else {
        return Ok(());
    };

    if !session_exists_in_profile(conn, default_profile_id, &session_id)? {
        return Ok(());
    }

    conn.execute(
        r#"
        INSERT INTO app_state (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        params![active_session_key(default_profile_id), session_id],
    )
    .map_err(|err| format!("migrate active session key failed: {err}"))?;

    Ok(())
}

fn ensure_active_profile_key(conn: &Connection, default_profile_id: &str) -> Result<(), String> {
    let active_profile = conn
        .query_row(
            "SELECT value FROM app_state WHERE key = ?1",
            params![ACTIVE_PROFILE_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| format!("load active profile key failed: {err}"))?;

    let Some(profile_id) = active_profile else {
        conn.execute(
            r#"
            INSERT INTO app_state (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![ACTIVE_PROFILE_KEY, default_profile_id],
        )
        .map_err(|err| format!("set default active profile failed: {err}"))?;
        return Ok(());
    };

    let exists = conn
        .query_row(
            "SELECT COUNT(1) FROM profiles WHERE id = ?1",
            params![profile_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|err| format!("validate active profile key failed: {err}"))?;

    if exists > 0 {
        return Ok(());
    }

    conn.execute(
        r#"
        INSERT INTO app_state (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        params![ACTIVE_PROFILE_KEY, default_profile_id],
    )
    .map_err(|err| format!("reset invalid active profile failed: {err}"))?;

    Ok(())
}

fn validate_profile_name(raw_name: &str) -> Result<String, String> {
    let normalized = raw_name.trim();
    if normalized.is_empty() {
        return Err("profile name cannot be empty".to_string());
    }

    if normalized.chars().count() > 80 {
        return Err("profile name is too long (max 80 chars)".to_string());
    }

    Ok(normalized.to_string())
}

fn db_path() -> Result<PathBuf, String> {
    let mut data_dir = dirs::data_dir().ok_or_else(|| "cannot resolve data dir".to_string())?;
    data_dir.push("chatminal");
    std::fs::create_dir_all(&data_dir).map_err(|err| format!("create data dir failed: {err}"))?;
    data_dir.push("chatminal.db");
    Ok(data_dir)
}

fn tail_lines(content: &str, count: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= count {
        return content.to_string();
    }

    let mut result = lines[lines.len() - count..].join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn trim_session_lines(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
    max_lines_per_session: usize,
) -> Result<(), String> {
    let total_lines = tx
        .query_row(
            "SELECT COALESCE(SUM(line_count), 0) FROM scrollback WHERE session_id = ?1",
            params![session_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|err| format!("query total lines failed: {err}"))?;

    if total_lines <= max_lines_per_session as i64 {
        return Ok(());
    }

    let overflow = total_lines - max_lines_per_session as i64;
    let mut consumed = 0_i64;
    let mut cutoff_seq = None;

    let mut stmt = tx
        .prepare("SELECT seq, line_count FROM scrollback WHERE session_id = ?1 ORDER BY seq ASC")
        .map_err(|err| format!("prepare retention scan failed: {err}"))?;
    let rows = stmt
        .query_map(params![session_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|err| format!("scan retention rows failed: {err}"))?;

    for row in rows {
        let (seq, line_count) = row.map_err(|err| format!("read retention row failed: {err}"))?;
        consumed += line_count.max(1);
        if consumed >= overflow {
            cutoff_seq = Some(seq);
            break;
        }
    }

    if let Some(seq) = cutoff_seq {
        tx.execute(
            "DELETE FROM scrollback WHERE session_id = ?1 AND seq <= ?2",
            params![session_id, seq],
        )
        .map_err(|err| format!("delete overflow history failed: {err}"))?;
    }

    Ok(())
}

fn trim_ttl(tx: &rusqlite::Transaction<'_>, auto_delete_after_days: u32) -> Result<(), String> {
    if auto_delete_after_days == 0 {
        return Ok(());
    }

    let cutoff = now_ts_millis().saturating_sub(auto_delete_after_days as u64 * 86_400_000);
    tx.execute(
        "DELETE FROM scrollback WHERE ts < ?1",
        params![cutoff as i64],
    )
    .map_err(|err| format!("trim TTL failed: {err}"))?;
    Ok(())
}

fn status_to_db(status: &SessionStatus) -> &'static str {
    match status {
        SessionStatus::Running => "running",
        SessionStatus::Disconnected => "disconnected",
    }
}

fn bool_to_db(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

pub fn now_ts_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
