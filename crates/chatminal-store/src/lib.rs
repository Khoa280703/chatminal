mod schema;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chatminal_protocol::{ProfileInfo, SessionInfo, SessionSnapshot, SessionStatus};
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

const DEFAULT_PROFILE_NAME: &str = "Default";
const ACTIVE_PROFILE_KEY: &str = "active_profile_id";
const ACTIVE_SESSION_PREFIX: &str = "active_session_id:";

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub session_id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub shell: String,
    pub status: SessionStatus,
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

impl Store {
    pub fn initialize_default() -> Result<Self, String> {
        let data_dir = default_data_dir()?;
        std::fs::create_dir_all(&data_dir)
            .map_err(|err| format!("create data directory failed: {err}"))?;
        let db_path = data_dir.join("chatminald.db");
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

    pub fn load_workspace(&self) -> Result<(Vec<ProfileInfo>, String, Vec<SessionInfo>, Option<String>), String> {
        let conn = self.open_connection()?;
        let profiles = self.list_profiles_with_conn(&conn)?;
        let active_profile_id = self
            .active_profile_id_with_conn(&conn)?
            .or_else(|| profiles.first().map(|value| value.profile_id.clone()))
            .ok_or_else(|| "no profile available".to_string())?;

        let sessions = self.list_sessions_by_profile_with_conn(&conn, &active_profile_id)?;
        let active_session_id = self.active_session_with_conn(&conn, &active_profile_id)?;
        Ok((profiles, active_profile_id, sessions, active_session_id))
    }

    pub fn list_profiles(&self) -> Result<Vec<ProfileInfo>, String> {
        let conn = self.open_connection()?;
        self.list_profiles_with_conn(&conn)
    }

    pub fn create_profile(&self, raw_name: Option<String>) -> Result<ProfileInfo, String> {
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

        let profile = ProfileInfo {
            profile_id: Uuid::new_v4().to_string(),
            name,
        };
        let now = now_millis() as i64;
        conn.execute(
            "INSERT INTO profiles (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![&profile.profile_id, &profile.name, now, now],
        )
        .map_err(|err| format!("create profile failed: {err}"))?;
        Ok(profile)
    }

    pub fn rename_profile(&self, profile_id: &str, name: &str) -> Result<ProfileInfo, String> {
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

        Ok(ProfileInfo {
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
        tx.execute("DELETE FROM sessions WHERE profile_id = ?1", params![profile_id])
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
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM app_state WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| format!("get bool state failed: {err}"))?;

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
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, if value { "1" } else { "0" }],
        )
        .map_err(|err| format!("set bool state failed: {err}"))?;
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

    pub fn list_sessions_by_profile(&self, profile_id: &str) -> Result<Vec<SessionInfo>, String> {
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
            status: SessionStatus::Disconnected,
            persist_history,
            seq: 0,
        };

        self.upsert_session(&stored)?;
        Ok(stored)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<StoredSession>, String> {
        let conn = self.open_connection()?;
        let row = conn
            .query_row(
                "SELECT id, profile_id, name, cwd, shell, status, persist_history, last_seq FROM sessions WHERE id = ?1",
                params![session_id],
                |row| {
                    Ok(StoredSession {
                        session_id: row.get(0)?,
                        profile_id: row.get(1)?,
                        name: row.get(2)?,
                        cwd: row.get(3)?,
                        shell: row.get(4)?,
                        status: status_from_db(row.get::<_, String>(5)?.as_str()),
                        persist_history: row.get::<_, i64>(6)? != 0,
                        seq: row.get::<_, i64>(7)?.max(0) as u64,
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
            r#"INSERT INTO sessions (id, profile_id, name, cwd, shell, status, persist_history, last_seq, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
               ON CONFLICT(id) DO UPDATE SET
                   profile_id = excluded.profile_id,
                   name = excluded.name,
                   cwd = excluded.cwd,
                   shell = excluded.shell,
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

    pub fn set_session_status(&self, session_id: &str, status: SessionStatus) -> Result<(), String> {
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

    pub fn set_session_persist(&self, session_id: &str, persist_history: bool) -> Result<(), String> {
        let conn = self.open_connection()?;
        conn.execute(
            "UPDATE sessions SET persist_history = ?1, updated_at = ?2 WHERE id = ?3",
            params![if persist_history { 1 } else { 0 }, now_millis() as i64, session_id],
        )
        .map_err(|err| format!("set session persist failed: {err}"))?;
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

    pub fn set_active_session(&self, profile_id: &str, session_id: Option<&str>) -> Result<(), String> {
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

    pub fn append_scrollback_chunk(&self, session_id: &str, seq: u64, chunk: &str, ts: u64) -> Result<(), String> {
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

    pub fn session_snapshot(&self, session_id: &str, preview_lines: usize) -> Result<SessionSnapshot, String> {
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
            return Ok(SessionSnapshot {
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
        Ok(SessionSnapshot {
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
            "INSERT INTO profiles (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
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
        conn.query_row(
            "SELECT value FROM app_state WHERE key = ?1",
            params![ACTIVE_PROFILE_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("load active profile failed: {err}"))
    }

    fn active_session_with_conn(&self, conn: &Connection, profile_id: &str) -> Result<Option<String>, String> {
        conn.query_row(
            "SELECT value FROM app_state WHERE key = ?1",
            params![format!("{ACTIVE_SESSION_PREFIX}{profile_id}")],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("load active session failed: {err}"))
    }

    fn list_profiles_with_conn(&self, conn: &Connection) -> Result<Vec<ProfileInfo>, String> {
        let mut stmt = conn
            .prepare("SELECT id, name FROM profiles ORDER BY updated_at DESC, created_at ASC")
            .map_err(|err| format!("prepare list profiles failed: {err}"))?;
        let mut rows = stmt
            .query([])
            .map_err(|err| format!("query profiles failed: {err}"))?;

        let mut profiles = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("read profile row failed: {err}"))?
        {
            profiles.push(ProfileInfo {
                profile_id: row.get(0).unwrap_or_default(),
                name: row.get(1).unwrap_or_default(),
            });
        }

        Ok(profiles)
    }

    fn list_sessions_by_profile_with_conn(&self, conn: &Connection, profile_id: &str) -> Result<Vec<SessionInfo>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, profile_id, name, cwd, status, persist_history, last_seq FROM sessions WHERE profile_id = ?1 ORDER BY updated_at DESC, created_at ASC",
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
            sessions.push(SessionInfo {
                session_id: row.get(0).unwrap_or_default(),
                profile_id: row.get(1).unwrap_or_default(),
                name: row.get(2).unwrap_or_default(),
                cwd: row.get(3).unwrap_or_default(),
                status: status_from_db(row.get::<_, String>(4).unwrap_or_default().as_str()),
                persist_history: row.get::<_, i64>(5).unwrap_or_default() != 0,
                seq: row.get::<_, i64>(6).unwrap_or_default().max(0) as u64,
            });
        }

        Ok(sessions)
    }
}

fn status_to_db(status: &SessionStatus) -> &'static str {
    match status {
        SessionStatus::Running => "running",
        SessionStatus::Disconnected => "disconnected",
    }
}

fn status_from_db(value: &str) -> SessionStatus {
    if value.eq_ignore_ascii_case("running") {
        SessionStatus::Running
    } else {
        SessionStatus::Disconnected
    }
}

fn default_data_dir() -> Result<PathBuf, String> {
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
