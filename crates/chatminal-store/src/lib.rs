mod schema;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

const DEFAULT_PROFILE_NAME: &str = "Default";
const ACTIVE_PROFILE_KEY: &str = "active_profile_id";
const ACTIVE_SESSION_PREFIX: &str = "active_session_id:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredSessionStatus {
    Running,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct StoredProfile {
    pub profile_id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct StoredSessionSummary {
    pub session_id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub startup_command: Option<String>,
    pub status: StoredSessionStatus,
    pub persist_history: bool,
    pub seq: u64,
}

#[derive(Debug, Clone)]
pub struct StoredWorkspace {
    pub profiles: Vec<StoredProfile>,
    pub active_profile_id: String,
    pub sessions: Vec<StoredSessionSummary>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredSessionSnapshot {
    pub content: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredScrollbackRecordKind {
    Line,
    Fragment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredScrollbackRecord {
    pub session_id: String,
    pub seq: u64,
    pub ord: u64,
    pub kind: StoredScrollbackRecordKind,
    pub text: String,
    pub ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredLegacyScrollbackChunk {
    pub session_id: String,
    pub seq: u64,
    pub chunk_text: String,
    pub ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTerminalReplayChunk {
    pub session_id: String,
    pub seq: u64,
    pub chunk_text: String,
    pub ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredScrollbackRecordInput {
    pub ord: u64,
    pub kind: StoredScrollbackRecordKind,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub session_id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub shell: String,
    pub startup_command: Option<String>,
    pub status: StoredSessionStatus,
    pub persist_history: bool,
    pub seq: u64,
}

#[derive(Debug, Clone)]
pub struct StoredSessionExplorerState {
    pub session_id: String,
    pub root_path: String,
    pub current_dir: String,
    pub selected_path: Option<String>,
    pub open_file_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Store {
    db_path: PathBuf,
}

fn scrollback_record_kind_to_db(kind: StoredScrollbackRecordKind) -> &'static str {
    match kind {
        StoredScrollbackRecordKind::Line => "line",
        StoredScrollbackRecordKind::Fragment => "fragment",
    }
}

fn scrollback_record_kind_from_db(value: &str) -> StoredScrollbackRecordKind {
    match value {
        "fragment" => StoredScrollbackRecordKind::Fragment,
        _ => StoredScrollbackRecordKind::Line,
    }
}

impl Store {
    pub fn initialize_default() -> Result<Self, String> {
        let data_dir = default_data_dir()?;
        std::fs::create_dir_all(&data_dir)
            .map_err(|err| format!("create data directory failed: {err}"))?;
        let db_path = data_dir.join("chatminal.db");
        Self::initialize(db_path)
    }

    pub fn initialize<P: AsRef<Path>>(db_path: P) -> Result<Self, String> {
        let store = Self {
            db_path: db_path.as_ref().to_path_buf(),
        };
        store.init_schema()?;
        store.ensure_default_profile()?;
        Ok(store)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn load_workspace(&self) -> Result<StoredWorkspace, String> {
        let conn = self.open_connection()?;
        let profiles = self.list_profiles_with_conn(&conn)?;
        let active_profile_id = self
            .active_profile_id_with_conn(&conn)?
            .or_else(|| profiles.first().map(|value| value.profile_id.clone()))
            .ok_or_else(|| "no profile available".to_string())?;

        let sessions = self.list_sessions_with_conn(&conn)?;
        let active_session_id = self.active_session_with_conn(&conn, &active_profile_id)?;
        Ok(StoredWorkspace {
            profiles,
            active_profile_id,
            sessions,
            active_session_id,
        })
    }

    pub fn list_profiles(&self) -> Result<Vec<StoredProfile>, String> {
        let conn = self.open_connection()?;
        self.list_profiles_with_conn(&conn)
    }

    pub fn create_profile(&self, raw_name: Option<String>) -> Result<StoredProfile, String> {
        let conn = self.open_connection()?;
        let name = match raw_name {
            Some(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    format!("Profile {}", self.list_profiles_with_conn(&conn)?.len() + 1)
                } else {
                    trimmed.to_string()
                }
            }
            None => format!("Profile {}", self.list_profiles_with_conn(&conn)?.len() + 1),
        };

        let profile = StoredProfile {
            profile_id: Uuid::new_v4().to_string(),
            name,
        };
        let now = now_millis() as i64;
        let sort_order = self.next_profile_sort_order_with_conn(&conn)? as i64;
        conn.execute(
            "INSERT INTO profiles (id, name, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&profile.profile_id, &profile.name, sort_order, now, now],
        )
        .map_err(|err| format!("create profile failed: {err}"))?;
        Ok(profile)
    }

    pub fn rename_profile(&self, profile_id: &str, name: &str) -> Result<StoredProfile, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("profile name cannot be empty".to_string());
        }

        let conn = self.open_connection()?;
        let affected = conn
            .execute(
                "UPDATE profiles SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![trimmed, now_millis() as i64, profile_id],
            )
            .map_err(|err| format!("rename profile failed: {err}"))?;

        if affected == 0 {
            return Err("profile not found".to_string());
        }

        Ok(StoredProfile {
            profile_id: profile_id.to_string(),
            name: trimmed.to_string(),
        })
    }

    pub fn delete_profile(&self, profile_id: &str) -> Result<(), String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open delete profile transaction failed: {err}"))?;

        let count: i64 = tx
            .query_row("SELECT COUNT(1) FROM profiles", [], |row| row.get(0))
            .map_err(|err| format!("count profiles failed: {err}"))?;
        if count <= 1 {
            return Err("cannot delete last profile".to_string());
        }

        let exists: i64 = tx
            .query_row(
                "SELECT COUNT(1) FROM profiles WHERE id = ?1",
                params![profile_id],
                |row| row.get(0),
            )
            .map_err(|err| format!("validate profile failed: {err}"))?;
        if exists == 0 {
            return Err("profile not found".to_string());
        }

        let replacement: String = tx
            .query_row(
                "SELECT id FROM profiles WHERE id <> ?1 ORDER BY updated_at DESC, created_at ASC LIMIT 1",
                params![profile_id],
                |row| row.get(0),
            )
            .map_err(|err| format!("resolve replacement profile failed: {err}"))?;

        tx.execute(
            "DELETE FROM app_state WHERE key = ?1",
            params![format!("{ACTIVE_SESSION_PREFIX}{profile_id}")],
        )
        .map_err(|err| format!("clear active session state failed: {err}"))?;

        let active_profile = self.active_profile_id_with_conn(&tx)?;
        if active_profile.as_deref() == Some(profile_id) {
            tx.execute(
                "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![ACTIVE_PROFILE_KEY, replacement],
            )
            .map_err(|err| format!("switch active profile failed: {err}"))?;
        }

        tx.execute(
            "DELETE FROM scrollback_chunks WHERE session_id IN (SELECT id FROM sessions WHERE profile_id = ?1)",
            params![profile_id],
        )
        .map_err(|err| format!("delete profile history failed: {err}"))?;
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

    pub fn set_active_profile(&self, profile_id: &str) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![ACTIVE_PROFILE_KEY, profile_id],
        )
        .map_err(|err| format!("set active profile failed: {err}"))?;
        Ok(())
    }

    pub fn get_bool_state(&self, key: &str, default: bool) -> Result<bool, String> {
        let conn = self.open_connection()?;
        let raw = self.get_string_state_with_conn(&conn, key)?;

        let Some(value) = raw else {
            return Ok(default);
        };

        let normalized = value.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "1" | "true" | "yes" | "on") {
            return Ok(true);
        }
        if matches!(normalized.as_str(), "0" | "false" | "no" | "off") {
            return Ok(false);
        }
        Ok(default)
    }

    pub fn set_bool_state(&self, key: &str, value: bool) -> Result<(), String> {
        self.set_string_state(key, if value { "1" } else { "0" })
    }

    pub fn get_string_state(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.open_connection()?;
        self.get_string_state_with_conn(&conn, key)
    }

    pub fn set_string_state(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|err| format!("set string state failed: {err}"))?;
        Ok(())
    }

    pub fn clear_state(&self, key: &str) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute("DELETE FROM app_state WHERE key = ?1", params![key])
            .map_err(|err| format!("clear state failed: {err}"))?;
        Ok(())
    }

    pub fn get_session_explorer_state(
        &self,
        session_id: &str,
    ) -> Result<Option<StoredSessionExplorerState>, String> {
        let conn = self.open_connection()?;
        conn.query_row(
            r#"
            SELECT session_id, root_path, current_dir, selected_path, open_file_path
            FROM session_explorer_state
            WHERE session_id = ?1
            "#,
            params![session_id],
            |row| {
                Ok(StoredSessionExplorerState {
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
    ) -> Result<StoredSessionExplorerState, String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open session explorer root transaction failed: {err}"))?;

        let exists: i64 = tx
            .query_row(
                "SELECT COUNT(1) FROM sessions WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
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
            params![session_id, root_path, now_millis() as i64],
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
                    Ok(StoredSessionExplorerState {
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
    ) -> Result<StoredSessionExplorerState, String> {
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
                    now_millis() as i64,
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

    pub fn list_sessions_by_profile(
        &self,
        profile_id: &str,
    ) -> Result<Vec<StoredSessionSummary>, String> {
        let conn = self.open_connection()?;
        self.list_sessions_by_profile_with_conn(&conn, profile_id)
    }

    pub fn create_session(
        &self,
        profile_id: &str,
        name: Option<String>,
        cwd: String,
        shell: String,
        persist_history: bool,
    ) -> Result<StoredSession, String> {
        let conn = self.open_connection()?;
        let session_id = Uuid::new_v4().to_string();
        let trimmed_name = name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| "Session".to_string());

        let stored = StoredSession {
            session_id: session_id.clone(),
            profile_id: profile_id.to_string(),
            name: trimmed_name,
            cwd,
            shell,
            startup_command: None,
            status: StoredSessionStatus::Disconnected,
            persist_history,
            seq: 0,
        };

        let sort_order = self.next_session_sort_order_with_conn(&conn, profile_id)? as i64;
        let now = now_millis() as i64;
        conn.execute(
            r#"INSERT INTO sessions (id, profile_id, name, cwd, shell, startup_command, status, persist_history, last_seq, sort_order, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            params![
                &stored.session_id,
                &stored.profile_id,
                &stored.name,
                &stored.cwd,
                &stored.shell,
                &stored.startup_command,
                status_to_db(&stored.status),
                if stored.persist_history { 1 } else { 0 },
                stored.seq as i64,
                sort_order,
                now,
                now
            ],
        )
        .map_err(|err| format!("create session failed: {err}"))?;
        Ok(stored)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<StoredSession>, String> {
        let conn = self.open_connection()?;
        let row = conn
            .query_row(
                "SELECT id, profile_id, name, cwd, shell, startup_command, status, persist_history, last_seq FROM sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    Ok(StoredSession {
                        session_id: row.get(0)?,
                        profile_id: row.get(1)?,
                        name: row.get(2)?,
                        cwd: row.get(3)?,
                        shell: row.get(4)?,
                        startup_command: row.get::<_, Option<String>>(5)?,
                        status: status_from_db(row.get::<_, String>(6)?.as_str()),
                        persist_history: row.get::<_, i64>(7)? != 0,
                        seq: row.get::<_, i64>(8)?.max(0) as u64,
                    })
                },
            )
            .optional()
            .map_err(|err| format!("load session failed: {err}"))?;
        Ok(row)
    }

    pub fn upsert_session(&self, session: &StoredSession) -> Result<(), String> {
        let conn = self.open_connection()?;
        let now = now_millis() as i64;
        conn.execute(
            r#"INSERT INTO sessions (id, profile_id, name, cwd, shell, startup_command, status, persist_history, last_seq, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
               ON CONFLICT(id) DO UPDATE SET
                   profile_id = excluded.profile_id,
                   name = excluded.name,
                   cwd = excluded.cwd,
                   shell = excluded.shell,
                   startup_command = excluded.startup_command,
                   status = excluded.status,
                   persist_history = excluded.persist_history,
                   last_seq = excluded.last_seq,
                   updated_at = excluded.updated_at"#,
            params![
                &session.session_id,
                &session.profile_id,
                &session.name,
                &session.cwd,
                &session.shell,
                &session.startup_command,
                status_to_db(&session.status),
                if session.persist_history { 1 } else { 0 },
                session.seq as i64,
                now,
                now
            ],
        )
        .map_err(|err| format!("upsert session failed: {err}"))?;
        Ok(())
    }

    pub fn set_session_status(
        &self,
        session_id: &str,
        status: StoredSessionStatus,
    ) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status_to_db(&status), now_millis() as i64, session_id],
        )
        .map_err(|err| format!("set session status failed: {err}"))?;
        Ok(())
    }

    pub fn rename_session(&self, session_id: &str, name: &str) -> Result<(), String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("session name cannot be empty".to_string());
        }

        let conn = self.open_connection()?;
        let affected = conn
            .execute(
                "UPDATE sessions SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![trimmed, now_millis() as i64, session_id],
            )
            .map_err(|err| format!("rename session failed: {err}"))?;
        if affected == 0 {
            return Err("session not found".to_string());
        }
        Ok(())
    }

    pub fn set_session_persist(
        &self,
        session_id: &str,
        persist_history: bool,
    ) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE sessions SET persist_history = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                if persist_history { 1 } else { 0 },
                now_millis() as i64,
                session_id
            ],
        )
        .map_err(|err| format!("set session persist failed: {err}"))?;
        Ok(())
    }

    pub fn set_session_startup_command(
        &self,
        session_id: &str,
        startup_command: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.open_connection()?;
        let startup_command = startup_command
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let affected = conn.execute(
            "UPDATE sessions SET startup_command = ?1, updated_at = ?2 WHERE id = ?3",
            params![startup_command, now_millis() as i64, session_id],
        )
        .map_err(|err| format!("set session startup command failed: {err}"))?;
        if affected == 0 {
            return Err("session not found".to_string());
        }
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open delete session transaction failed: {err}"))?;
        let profile_id: Option<String> = tx
            .query_row(
                "SELECT profile_id FROM sessions WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| format!("load session profile failed: {err}"))?;

        tx.execute(
            "DELETE FROM scrollback_chunks WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|err| format!("delete history failed: {err}"))?;
        tx.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
            .map_err(|err| format!("delete session failed: {err}"))?;

        if let Some(profile_id) = profile_id {
            tx.execute(
                "DELETE FROM app_state WHERE key = ?1 AND value = ?2",
                params![format!("{ACTIVE_SESSION_PREFIX}{profile_id}"), session_id],
            )
            .map_err(|err| format!("clear active session marker failed: {err}"))?;
        }

        tx.commit()
            .map_err(|err| format!("commit delete session failed: {err}"))?;
        Ok(())
    }

    pub fn move_session_to_profile(
        &self,
        session_id: &str,
        target_profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<(), String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open move session transaction failed: {err}"))?;

        let source_profile_id: String = tx
            .query_row(
                "SELECT profile_id FROM sessions WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| format!("load source profile failed: {err}"))?
            .ok_or_else(|| "session not found".to_string())?;

        let target_exists: i64 = tx
            .query_row(
                "SELECT COUNT(1) FROM profiles WHERE id = ?1",
                params![target_profile_id],
                |row| row.get(0),
            )
            .map_err(|err| format!("validate target profile failed: {err}"))?;
        if target_exists == 0 {
            return Err("target profile not found".to_string());
        }

        let mut source_ids = self.session_ids_by_profile_with_conn(&tx, &source_profile_id)?;
        let source_index = source_ids
            .iter()
            .position(|id| id == session_id)
            .ok_or_else(|| "session not found in source profile".to_string())?;
        source_ids.remove(source_index);

        let now = now_millis() as i64;
        if source_profile_id == target_profile_id {
            let insert_at = target_index.unwrap_or(source_ids.len()).min(source_ids.len());
            source_ids.insert(insert_at, session_id.to_string());
            self.resequence_sessions_for_profile_with_conn(&tx, &source_profile_id, &source_ids)?;
        } else {
            let mut target_ids = self.session_ids_by_profile_with_conn(&tx, target_profile_id)?;
            let insert_at = target_index.unwrap_or(target_ids.len()).min(target_ids.len());
            target_ids.insert(insert_at, session_id.to_string());

            tx.execute(
                "UPDATE sessions SET profile_id = ?1, updated_at = ?2 WHERE id = ?3",
                params![target_profile_id, now, session_id],
            )
            .map_err(|err| format!("move session profile failed: {err}"))?;

            self.resequence_sessions_for_profile_with_conn(&tx, &source_profile_id, &source_ids)?;
            self.resequence_sessions_for_profile_with_conn(&tx, target_profile_id, &target_ids)?;

            if self.active_session_with_conn(&tx, &source_profile_id)?.as_deref() == Some(session_id)
            {
                self.set_active_session_with_conn(
                    &tx,
                    &source_profile_id,
                    source_ids.first().map(|value| value.as_str()),
                )?;
            }
        }

        tx.commit()
            .map_err(|err| format!("commit move session failed: {err}"))?;
        Ok(())
    }

    pub fn move_sessions_to_profile(
        &self,
        session_ids: &[String],
        target_profile_id: &str,
        target_index: Option<usize>,
    ) -> Result<(), String> {
        if session_ids.is_empty() {
            return Ok(());
        }

        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open move sessions transaction failed: {err}"))?;

        let target_exists: i64 = tx
            .query_row(
                "SELECT COUNT(1) FROM profiles WHERE id = ?1",
                params![target_profile_id],
                |row| row.get(0),
            )
            .map_err(|err| format!("validate target profile failed: {err}"))?;
        if target_exists == 0 {
            return Err("target profile not found".to_string());
        }

        let mut source_profile_id: Option<String> = None;
        let mut moved_ids = Vec::with_capacity(session_ids.len());
        let mut seen = std::collections::BTreeSet::new();
        for session_id in session_ids {
            if !seen.insert(session_id.as_str()) {
                continue;
            }
            let current_profile_id: String = tx
                .query_row(
                    "SELECT profile_id FROM sessions WHERE id = ?1",
                    params![session_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|err| format!("load source profile failed: {err}"))?
                .ok_or_else(|| "session not found".to_string())?;
            match source_profile_id.as_deref() {
                Some(existing) if existing != current_profile_id => {
                    return Err("all sessions must belong to the same profile".to_string());
                }
                Some(_) => {}
                None => source_profile_id = Some(current_profile_id),
            }
            moved_ids.push(session_id.clone());
        }
        let Some(source_profile_id) = source_profile_id else {
            return Ok(());
        };

        let mut source_ids = self.session_ids_by_profile_with_conn(&tx, &source_profile_id)?;
        let moved_lookup: std::collections::BTreeSet<&str> =
            moved_ids.iter().map(String::as_str).collect();
        let source_active_session_id = self.active_session_with_conn(&tx, &source_profile_id)?;
        let original_source_len = source_ids.len();
        source_ids.retain(|id| !moved_lookup.contains(id.as_str()));
        if source_ids.len() + moved_ids.len() != original_source_len {
            return Err("session not found in source profile".to_string());
        }

        if source_profile_id == target_profile_id {
            let insert_at = target_index.unwrap_or(source_ids.len()).min(source_ids.len());
            source_ids.splice(insert_at..insert_at, moved_ids.iter().cloned());
            self.resequence_sessions_for_profile_with_conn(&tx, &source_profile_id, &source_ids)?;
        } else {
            let mut target_ids = self.session_ids_by_profile_with_conn(&tx, target_profile_id)?;
            let insert_at = target_index.unwrap_or(target_ids.len()).min(target_ids.len());
            target_ids.splice(insert_at..insert_at, moved_ids.iter().cloned());
            self.resequence_sessions_for_profile_with_conn(&tx, &source_profile_id, &source_ids)?;
            self.resequence_sessions_for_profile_with_conn(&tx, target_profile_id, &target_ids)?;

            if source_active_session_id
                .as_deref()
                .is_some_and(|session_id| moved_lookup.contains(session_id))
            {
                self.set_active_session_with_conn(
                    &tx,
                    &source_profile_id,
                    source_ids.first().map(|value| value.as_str()),
                )?;
            }
        }

        tx.commit()
            .map_err(|err| format!("commit move sessions failed: {err}"))?;
        Ok(())
    }

    pub fn set_active_session(
        &self,
        profile_id: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.open_connection()?;
        let key = format!("{ACTIVE_SESSION_PREFIX}{profile_id}");
        match session_id {
            Some(value) => {
                conn.execute(
                    "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
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

    pub fn update_session_seq(&self, session_id: &str, seq: u64) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE sessions SET last_seq = MAX(last_seq, ?1), updated_at = ?2 WHERE id = ?3",
            params![seq as i64, now_millis() as i64, session_id],
        )
        .map_err(|err| format!("update session seq failed: {err}"))?;
        Ok(())
    }

    pub fn append_scrollback_chunk(
        &self,
        session_id: &str,
        seq: u64,
        chunk: &str,
        ts: u64,
    ) -> Result<(), String> {
        let conn = self.open_connection()?;
        let line_count = if chunk.is_empty() {
            0
        } else {
            chunk.matches('\n').count().max(1) as u64
        };
        conn.execute(
            "INSERT OR REPLACE INTO scrollback_chunks (session_id, seq, chunk_text, line_count, ts) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, seq as i64, chunk, line_count as i64, ts as i64],
        )
        .map_err(|err| format!("append scrollback chunk failed: {err}"))?;
        Ok(())
    }

    pub fn append_scrollback_records(
        &self,
        session_id: &str,
        seq: u64,
        records: &[StoredScrollbackRecordInput],
        ts: u64,
    ) -> Result<(), String> {
        let mut conn = self.open_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| format!("open append scrollback records transaction failed: {err}"))?;
        tx.execute(
            "DELETE FROM scrollback_records WHERE session_id = ?1 AND seq = ?2",
            params![session_id, seq as i64],
        )
        .map_err(|err| format!("clear existing scrollback records failed: {err}"))?;

        for record in records {
            tx.execute(
                "INSERT INTO scrollback_records (session_id, seq, ord, kind, record_text, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    session_id,
                    seq as i64,
                    record.ord as i64,
                    scrollback_record_kind_to_db(record.kind),
                    &record.text,
                    ts as i64,
                ],
            )
            .map_err(|err| format!("append scrollback record failed: {err}"))?;
        }

        tx.commit()
            .map_err(|err| format!("commit append scrollback records failed: {err}"))?;
        Ok(())
    }

    pub fn append_terminal_replay_chunk(
        &self,
        session_id: &str,
        seq: u64,
        chunk: &str,
        ts: u64,
    ) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO session_terminal_replay_chunks (session_id, seq, chunk_text, ts) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, seq as i64, chunk, ts as i64],
        )
        .map_err(|err| format!("append terminal replay chunk failed: {err}"))?;
        Ok(())
    }

    pub fn list_scrollback_records(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredScrollbackRecord>, String> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, ord, kind, record_text, ts FROM scrollback_records WHERE session_id = ?1 ORDER BY seq ASC, ord ASC",
            )
            .map_err(|err| format!("prepare scrollback records query failed: {err}"))?;
        let mut rows = stmt
            .query(params![session_id])
            .map_err(|err| format!("query scrollback records failed: {err}"))?;

        let mut records = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read scrollback record row failed: {err}"))?
        {
            records.push(StoredScrollbackRecord {
                session_id: session_id.to_string(),
                seq: row.get::<_, i64>(0).unwrap_or_default().max(0) as u64,
                ord: row.get::<_, i64>(1).unwrap_or_default().max(0) as u64,
                kind: scrollback_record_kind_from_db(
                    row.get::<_, String>(2).unwrap_or_else(|_| "line".to_string()).as_str(),
                ),
                text: row.get::<_, String>(3).unwrap_or_default(),
                ts: row.get::<_, i64>(4).unwrap_or_default().max(0) as u64,
            });
        }

        Ok(records)
    }

    pub fn list_legacy_scrollback_chunks(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredLegacyScrollbackChunk>, String> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, chunk_text, ts FROM scrollback_chunks WHERE session_id = ?1 ORDER BY seq ASC",
            )
            .map_err(|err| format!("prepare legacy scrollback query failed: {err}"))?;
        let mut rows = stmt
            .query(params![session_id])
            .map_err(|err| format!("query legacy scrollback failed: {err}"))?;

        let mut chunks = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read legacy scrollback row failed: {err}"))?
        {
            chunks.push(StoredLegacyScrollbackChunk {
                session_id: session_id.to_string(),
                seq: row.get::<_, i64>(0).unwrap_or_default().max(0) as u64,
                chunk_text: row.get::<_, String>(1).unwrap_or_default(),
                ts: row.get::<_, i64>(2).unwrap_or_default().max(0) as u64,
            });
        }

        Ok(chunks)
    }

    pub fn list_terminal_replay_chunks(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredTerminalReplayChunk>, String> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, chunk_text, ts FROM session_terminal_replay_chunks WHERE session_id = ?1 ORDER BY seq ASC",
            )
            .map_err(|err| format!("prepare terminal replay query failed: {err}"))?;
        let mut rows = stmt
            .query(params![session_id])
            .map_err(|err| format!("query terminal replay failed: {err}"))?;

        let mut chunks = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read terminal replay row failed: {err}"))?
        {
            chunks.push(StoredTerminalReplayChunk {
                session_id: session_id.to_string(),
                seq: row.get::<_, i64>(0).unwrap_or_default().max(0) as u64,
                chunk_text: row.get::<_, String>(1).unwrap_or_default(),
                ts: row.get::<_, i64>(2).unwrap_or_default().max(0) as u64,
            });
        }

        Ok(chunks)
    }

    pub fn session_terminal_replay_snapshot(
        &self,
        session_id: &str,
    ) -> Result<StoredSessionSnapshot, String> {
        let chunks = self.list_terminal_replay_chunks(session_id)?;
        let Some(max_seq) = chunks.last().map(|chunk| chunk.seq) else {
            return Ok(StoredSessionSnapshot {
                content: String::new(),
                seq: 0,
            });
        };

        Ok(StoredSessionSnapshot {
            content: chunks
                .into_iter()
                .map(|chunk| chunk.chunk_text)
                .collect::<Vec<_>>()
                .join(""),
            seq: max_seq,
        })
    }

    pub fn enforce_session_scrollback_record_limit(
        &self,
        session_id: &str,
        max_lines: usize,
    ) -> Result<(), String> {
        let max_lines = max_lines.max(1);
        let records = self.list_scrollback_records(session_id)?;
        if records.is_empty() {
            return Ok(());
        }

        let mut retained_lines = 0usize;
        let mut cutoff: Option<(u64, u64)> = None;
        for record in records.iter().rev() {
            if record.kind != StoredScrollbackRecordKind::Line {
                if cutoff.is_none() {
                    cutoff = Some((record.seq, record.ord));
                }
                continue;
            }

            if retained_lines >= max_lines {
                break;
            }
            retained_lines += 1;
            cutoff = Some((record.seq, record.ord));
        }

        let Some((cutoff_seq, cutoff_ord)) = cutoff else {
            return Ok(());
        };

        let conn = self.open_connection()?;
        conn.execute(
            "DELETE FROM scrollback_records WHERE session_id = ?1 AND (seq < ?2 OR (seq = ?2 AND ord < ?3))",
            params![session_id, cutoff_seq as i64, cutoff_ord as i64],
        )
        .map_err(|err| format!("apply canonical retention delete failed: {err}"))?;
        Ok(())
    }

    pub fn enforce_session_scrollback_line_limit(
        &self,
        session_id: &str,
        max_lines: usize,
    ) -> Result<(), String> {
        let max_lines = max_lines.max(1);
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, line_count FROM scrollback_chunks WHERE session_id = ?1 ORDER BY seq DESC",
            )
            .map_err(|err| format!("prepare retention query failed: {err}"))?;
        let mut rows = stmt
            .query(params![session_id])
            .map_err(|err| format!("query retention rows failed: {err}"))?;

        let mut retained_lines = 0usize;
        let mut min_seq_to_keep: Option<u64> = None;

        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read retention row failed: {err}"))?
        {
            let seq = row.get::<_, i64>(0).unwrap_or_default().max(0) as u64;
            let line_count = row.get::<_, i64>(1).unwrap_or_default().max(0) as usize;
            if min_seq_to_keep.is_none() {
                min_seq_to_keep = Some(seq);
                retained_lines = line_count;
                continue;
            }

            if retained_lines.saturating_add(line_count) > max_lines {
                break;
            }
            retained_lines = retained_lines.saturating_add(line_count);
            min_seq_to_keep = Some(seq);
        }

        if let Some(min_seq) = min_seq_to_keep {
            conn.execute(
                "DELETE FROM scrollback_chunks WHERE session_id = ?1 AND seq < ?2",
                params![session_id, min_seq as i64],
            )
            .map_err(|err| format!("apply retention delete failed: {err}"))?;
        }

        Ok(())
    }

    pub fn session_snapshot(
        &self,
        session_id: &str,
        preview_lines: usize,
    ) -> Result<StoredSessionSnapshot, String> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, chunk_text, line_count FROM scrollback_chunks WHERE session_id = ?1 ORDER BY seq DESC LIMIT 4096",
            )
            .map_err(|err| format!("prepare snapshot query failed: {err}"))?;

        let mut rows = stmt
            .query(params![session_id])
            .map_err(|err| format!("query snapshot failed: {err}"))?;

        let mut items: Vec<(u64, String, u64)> = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read snapshot row failed: {err}"))?
        {
            items.push((
                row.get::<_, i64>(0).unwrap_or_default().max(0) as u64,
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, i64>(2).unwrap_or_default().max(0) as u64,
            ));
        }

        if items.is_empty() {
            return Ok(StoredSessionSnapshot {
                content: String::new(),
                seq: 0,
            });
        }

        let max_seq = items[0].0;
        let mut line_budget = 0usize;
        let mut selected: Vec<String> = Vec::new();

        for (_, chunk, lines) in items {
            selected.push(chunk);
            line_budget += lines as usize;
            if preview_lines > 0 && line_budget >= preview_lines {
                break;
            }
        }

        selected.reverse();
        Ok(StoredSessionSnapshot {
            content: selected.join(""),
            seq: max_seq,
        })
    }

    pub fn clear_session_history(&self, session_id: &str) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "DELETE FROM scrollback_chunks WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|err| format!("clear session history failed: {err}"))?;
        conn.execute(
            "DELETE FROM scrollback_records WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|err| format!("clear session canonical history failed: {err}"))?;
        conn.execute(
            "DELETE FROM session_terminal_replay_chunks WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(|err| format!("clear session terminal replay failed: {err}"))?;
        conn.execute(
            "UPDATE sessions SET last_seq = 0, updated_at = ?1 WHERE id = ?2",
            params![now_millis() as i64, session_id],
        )
        .map_err(|err| format!("reset session seq failed: {err}"))?;
        Ok(())
    }

    pub fn clear_all_history(&self) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute("DELETE FROM scrollback_chunks", [])
            .map_err(|err| format!("clear all history failed: {err}"))?;
        conn.execute("DELETE FROM scrollback_records", [])
            .map_err(|err| format!("clear all canonical history failed: {err}"))?;
        conn.execute("DELETE FROM session_terminal_replay_chunks", [])
            .map_err(|err| format!("clear all terminal replay failed: {err}"))?;
        conn.execute(
            "UPDATE sessions SET last_seq = 0, updated_at = ?1",
            params![now_millis() as i64],
        )
        .map_err(|err| format!("reset all session seq failed: {err}"))?;
        Ok(())
    }

    fn init_schema(&self) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute_batch(schema::INIT_SQL)
            .map_err(|err| format!("initialize schema failed: {err}"))?;
        self.migrate_sort_order_columns(&conn)?;
        self.migrate_startup_command_column(&conn)?;
        Ok(())
    }

    fn ensure_default_profile(&self) -> Result<(), String> {
        let conn = self.open_connection()?;
        let count: i64 = conn
            .query_row("SELECT COUNT(1) FROM profiles", [], |row| row.get(0))
            .map_err(|err| format!("count profiles failed: {err}"))?;
        if count > 0 {
            return Ok(());
        }

        let default_id = Uuid::new_v4().to_string();
        let now = now_millis() as i64;
        conn.execute(
            "INSERT INTO profiles (id, name, sort_order, created_at, updated_at) VALUES (?1, ?2, 0, ?3, ?4)",
            params![&default_id, DEFAULT_PROFILE_NAME, now, now],
        )
        .map_err(|err| format!("create default profile failed: {err}"))?;
        conn.execute(
            "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![ACTIVE_PROFILE_KEY, default_id],
        )
        .map_err(|err| format!("set default active profile failed: {err}"))?;
        Ok(())
    }

    fn open_connection(&self) -> Result<Connection, String> {
        let conn = Connection::open(&self.db_path)
            .map_err(|err| format!("open database failed ('{}'): {err}", self.db_path.display()))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|err| format!("enable foreign_keys failed: {err}"))?;
        conn.busy_timeout(std::time::Duration::from_secs(2))
            .map_err(|err| format!("set busy_timeout failed: {err}"))?;
        Ok(conn)
    }

    fn active_profile_id_with_conn(&self, conn: &Connection) -> Result<Option<String>, String> {
        self.get_string_state_with_conn(conn, ACTIVE_PROFILE_KEY)
            .map_err(|err| format!("load active profile failed: {err}"))
    }

    fn active_session_with_conn(
        &self,
        conn: &Connection,
        profile_id: &str,
    ) -> Result<Option<String>, String> {
        self.get_string_state_with_conn(conn, &format!("{ACTIVE_SESSION_PREFIX}{profile_id}"))
            .map_err(|err| format!("load active session failed: {err}"))
    }

    fn set_active_session_with_conn(
        &self,
        conn: &Connection,
        profile_id: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let key = format!("{ACTIVE_SESSION_PREFIX}{profile_id}");
        match session_id {
            Some(value) => conn
                .execute(
                    "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )
                .map(|_| ())
                .map_err(|err| format!("set active session failed: {err}")),
            None => conn
                .execute("DELETE FROM app_state WHERE key = ?1", params![key])
                .map(|_| ())
                .map_err(|err| format!("clear active session failed: {err}")),
        }
    }

    fn get_string_state_with_conn(
        &self,
        conn: &Connection,
        key: &str,
    ) -> Result<Option<String>, String> {
        conn.query_row(
            "SELECT value FROM app_state WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("get string state failed: {err}"))
    }

    fn list_profiles_with_conn(&self, conn: &Connection) -> Result<Vec<StoredProfile>, String> {
        let mut stmt = conn
            .prepare("SELECT id, name FROM profiles ORDER BY sort_order ASC, created_at ASC, rowid ASC")
            .map_err(|err| format!("prepare list profiles failed: {err}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|err| format!("query profiles failed: {err}"))?;

        let mut profiles = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read profile row failed: {err}"))?
        {
            profiles.push(StoredProfile {
                profile_id: row.get(0).unwrap_or_default(),
                name: row.get(1).unwrap_or_default(),
            });
        }

        Ok(profiles)
    }

    fn list_sessions_by_profile_with_conn(
        &self,
        conn: &Connection,
        profile_id: &str,
    ) -> Result<Vec<StoredSessionSummary>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, profile_id, name, cwd, startup_command, status, persist_history, last_seq FROM sessions WHERE profile_id = ?1 ORDER BY sort_order ASC, created_at ASC, rowid ASC",
            )
            .map_err(|err| format!("prepare list sessions failed: {err}"))?;

        let mut rows = stmt
            .query(params![profile_id])
            .map_err(|err| format!("query sessions failed: {err}"))?;

        let mut sessions = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read session row failed: {err}"))?
        {
            sessions.push(StoredSessionSummary {
                session_id: row.get(0).unwrap_or_default(),
                profile_id: row.get(1).unwrap_or_default(),
                name: row.get(2).unwrap_or_default(),
                cwd: row.get(3).unwrap_or_default(),
                startup_command: row.get::<_, Option<String>>(4).unwrap_or_default(),
                status: status_from_db(row.get::<_, String>(5).unwrap_or_default().as_str()),
                persist_history: row.get::<_, i64>(6).unwrap_or_default() != 0,
                seq: row.get::<_, i64>(7).unwrap_or_default().max(0) as u64,
            });
        }

        Ok(sessions)
    }

    fn list_sessions_with_conn(&self, conn: &Connection) -> Result<Vec<StoredSessionSummary>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, profile_id, name, cwd, startup_command, status, persist_history, last_seq FROM sessions ORDER BY profile_id ASC, sort_order ASC, created_at ASC, rowid ASC",
            )
            .map_err(|err| format!("prepare list all sessions failed: {err}"))?;

        let mut rows = stmt
            .query([])
            .map_err(|err| format!("query all sessions failed: {err}"))?;

        let mut sessions = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read all session row failed: {err}"))?
        {
            sessions.push(StoredSessionSummary {
                session_id: row.get(0).unwrap_or_default(),
                profile_id: row.get(1).unwrap_or_default(),
                name: row.get(2).unwrap_or_default(),
                cwd: row.get(3).unwrap_or_default(),
                startup_command: row.get::<_, Option<String>>(4).unwrap_or_default(),
                status: status_from_db(row.get::<_, String>(5).unwrap_or_default().as_str()),
                persist_history: row.get::<_, i64>(6).unwrap_or_default() != 0,
                seq: row.get::<_, i64>(7).unwrap_or_default().max(0) as u64,
            });
        }

        Ok(sessions)
    }

    fn migrate_sort_order_columns(&self, conn: &Connection) -> Result<(), String> {
        let mut migrated_profiles = false;
        let mut migrated_sessions = false;

        if !self.column_exists(conn, "profiles", "sort_order")? {
            conn.execute(
                "ALTER TABLE profiles ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(|err| format!("add profiles.sort_order failed: {err}"))?;
            migrated_profiles = true;
        }

        if !self.column_exists(conn, "sessions", "sort_order")? {
            conn.execute(
                "ALTER TABLE sessions ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(|err| format!("add sessions.sort_order failed: {err}"))?;
            migrated_sessions = true;
        }

        if migrated_profiles {
            self.normalize_profile_sort_order(conn)?;
        }
        if migrated_sessions {
            self.normalize_session_sort_order(conn)?;
        }

        Ok(())
    }

    fn migrate_startup_command_column(&self, conn: &Connection) -> Result<(), String> {
        if self.column_exists(conn, "sessions", "startup_command")? {
            return Ok(());
        }

        conn.execute("ALTER TABLE sessions ADD COLUMN startup_command TEXT", [])
            .map_err(|err| format!("add sessions.startup_command failed: {err}"))?;
        Ok(())
    }

    fn column_exists(&self, conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|err| format!("prepare table_info for {table} failed: {err}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|err| format!("query table_info for {table} failed: {err}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read table_info for {table} failed: {err}"))?
        {
            let name = row.get::<_, String>(1).unwrap_or_default();
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn normalize_profile_sort_order(&self, conn: &Connection) -> Result<(), String> {
        let mut stmt = conn
            .prepare("SELECT id FROM profiles ORDER BY created_at ASC, rowid ASC")
            .map_err(|err| format!("prepare normalize profiles failed: {err}"))?;
        let ids = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|err| format!("query normalize profiles failed: {err}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("collect normalize profiles failed: {err}"))?;

        for (index, profile_id) in ids.iter().enumerate() {
            conn.execute(
                "UPDATE profiles SET sort_order = ?1 WHERE id = ?2",
                params![index as i64, profile_id],
            )
            .map_err(|err| format!("normalize profile sort order failed: {err}"))?;
        }
        Ok(())
    }

    fn normalize_session_sort_order(&self, conn: &Connection) -> Result<(), String> {
        for profile in self.list_profiles_with_conn(conn)? {
            let ids = self.session_ids_by_profile_with_conn(conn, &profile.profile_id)?;
            self.resequence_sessions_for_profile_with_conn(conn, &profile.profile_id, &ids)?;
        }
        Ok(())
    }

    fn next_profile_sort_order_with_conn(&self, conn: &Connection) -> Result<u64, String> {
        conn.query_row(
            "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM profiles",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value.max(0) as u64)
        .map_err(|err| format!("compute next profile sort order failed: {err}"))
    }

    fn next_session_sort_order_with_conn(
        &self,
        conn: &Connection,
        profile_id: &str,
    ) -> Result<u64, String> {
        conn.query_row(
            "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM sessions WHERE profile_id = ?1",
            params![profile_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value.max(0) as u64)
        .map_err(|err| format!("compute next session sort order failed: {err}"))
    }

    fn session_ids_by_profile_with_conn(
        &self,
        conn: &Connection,
        profile_id: &str,
    ) -> Result<Vec<String>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id FROM sessions WHERE profile_id = ?1 ORDER BY sort_order ASC, created_at ASC, rowid ASC",
            )
            .map_err(|err| format!("prepare session id list failed: {err}"))?;
        stmt.query_map(params![profile_id], |row| row.get::<_, String>(0))
            .map_err(|err| format!("query session id list failed: {err}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("collect session id list failed: {err}"))
    }

    fn resequence_sessions_for_profile_with_conn(
        &self,
        conn: &Connection,
        profile_id: &str,
        session_ids: &[String],
    ) -> Result<(), String> {
        for (index, current_session_id) in session_ids.iter().enumerate() {
            conn.execute(
                "UPDATE sessions SET profile_id = ?1, sort_order = ?2, updated_at = ?3 WHERE id = ?4",
                params![profile_id, index as i64, now_millis() as i64, current_session_id],
            )
            .map_err(|err| format!("resequence sessions failed: {err}"))?;
        }
        Ok(())
    }
}

fn status_to_db(status: &StoredSessionStatus) -> &'static str {
    match status {
        StoredSessionStatus::Running => "running",
        StoredSessionStatus::Disconnected => "disconnected",
    }
}

fn status_from_db(value: &str) -> StoredSessionStatus {
    if value.eq_ignore_ascii_case("running") {
        StoredSessionStatus::Running
    } else {
        StoredSessionStatus::Disconnected
    }
}

fn default_data_dir() -> Result<PathBuf, String> {
    if let Ok(raw) = std::env::var("CHATMINAL_DATA_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let configured = PathBuf::from(trimmed);
            if configured.is_absolute() {
                return Ok(configured);
            }
            if let Some(home) = dirs::home_dir() {
                return Ok(home.join(configured));
            }
            let cwd = std::env::current_dir()
                .map_err(|err| format!("resolve current dir failed: {err}"))?;
            return Ok(cwd.join(configured));
        }
    }
    let mut base = dirs::data_dir().ok_or_else(|| "resolve data directory failed".to_string())?;
    base.push("chatminal");
    Ok(base)
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}
