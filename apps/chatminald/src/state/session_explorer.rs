use std::fs::File;
use std::io::Read;

use chatminal_protocol::{SessionExplorerEntry, SessionExplorerFileContent, SessionExplorerState};

use super::StateInner;
use super::explorer_utils::{
    explorer_state_to_protocol, normalize_relative_path, resolve_explorer_root_path,
    resolve_explorer_target,
};

const MAX_EXPLORER_FILE_PREVIEW_BYTES: usize = 512 * 1024;
const MAX_EXPLORER_ENTRIES_PER_DIR: usize = 2_000;

impl StateInner {
    pub(super) fn get_session_explorer_state(
        &self,
        session_id: &str,
    ) -> Result<SessionExplorerState, String> {
        self.ensure_session_exists(session_id)?;
        let state = self.store.get_session_explorer_state(session_id)?;
        Ok(explorer_state_to_protocol(session_id, state))
    }

    pub(super) fn set_session_explorer_root(
        &self,
        session_id: &str,
        root_path: &str,
    ) -> Result<SessionExplorerState, String> {
        self.ensure_session_exists(session_id)?;
        let root = resolve_explorer_root_path(root_path)?;
        let saved = self
            .store
            .set_session_explorer_root(session_id, &root.to_string_lossy())?;
        Ok(explorer_state_to_protocol(session_id, Some(saved)))
    }

    pub(super) fn update_session_explorer_state(
        &self,
        session_id: &str,
        current_dir: &str,
        selected_path: Option<&str>,
        open_file_path: Option<&str>,
    ) -> Result<SessionExplorerState, String> {
        self.ensure_session_exists(session_id)?;
        let Some(current_state) = self.store.get_session_explorer_state(session_id)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&current_state.root_path)?;
        let normalized_current_dir = normalize_relative_path(current_dir)?;
        let current_dir_path = resolve_explorer_target(&root, &normalized_current_dir)?;
        if !current_dir_path.is_dir() {
            return Err("current explorer directory is not valid".to_string());
        }

        let selected = match selected_path {
            Some(value) => {
                let normalized = normalize_relative_path(value)?;
                if !normalized.is_empty() {
                    let target = resolve_explorer_target(&root, &normalized)?;
                    if !target.exists() {
                        return Err("selected explorer path does not exist".to_string());
                    }
                }
                Some(normalized)
            }
            None => None,
        };

        let open_file = match open_file_path {
            Some(value) => {
                let normalized = normalize_relative_path(value)?;
                let target = resolve_explorer_target(&root, &normalized)?;
                if !target.is_file() {
                    return Err("open file path is not a file".to_string());
                }
                Some(normalized)
            }
            None => None,
        };

        let saved = self.store.update_session_explorer_state(
            session_id,
            &normalized_current_dir,
            selected.as_deref(),
            open_file.as_deref(),
        )?;
        Ok(explorer_state_to_protocol(session_id, Some(saved)))
    }

    pub(super) fn list_session_explorer_entries(
        &self,
        session_id: &str,
        relative_path: Option<&str>,
    ) -> Result<Vec<SessionExplorerEntry>, String> {
        self.ensure_session_exists(session_id)?;
        let Some(state) = self.store.get_session_explorer_state(session_id)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&state.root_path)?;
        let target_relative = match relative_path {
            Some(value) => normalize_relative_path(value)?,
            None => state.current_dir,
        };
        let target_dir = resolve_explorer_target(&root, &target_relative)?;
        if !target_dir.is_dir() {
            return Err("explorer target is not a directory".to_string());
        }
        let lexical_target_dir = if target_relative.is_empty() {
            root.clone()
        } else {
            root.join(&target_relative)
        };

        let mut entries = Vec::new();
        let read_dir = std::fs::read_dir(&lexical_target_dir)
            .map_err(|err| format!("read explorer directory failed: {err}"))?;
        for item in read_dir {
            if entries.len() >= MAX_EXPLORER_ENTRIES_PER_DIR {
                break;
            }

            let entry = match item {
                Ok(value) => value,
                Err(_) => continue,
            };
            let entry_path = entry.path();
            let lexical_relative = match entry_path.strip_prefix(&root) {
                Ok(value) => value.to_path_buf(),
                Err(_) => continue,
            };

            let canonical = match std::fs::canonicalize(&entry_path) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if !canonical.starts_with(&root) {
                continue;
            }

            let metadata = match std::fs::metadata(&canonical) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let is_dir = metadata.is_dir();
            let relative = normalize_relative_path(&lexical_relative.to_string_lossy())?;
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(SessionExplorerEntry {
                name,
                relative_path: relative,
                is_dir,
                size: if is_dir { None } else { Some(metadata.len()) },
            });
        }

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase()),
        });
        Ok(entries)
    }

    pub(super) fn read_session_explorer_file(
        &self,
        session_id: &str,
        relative_path: &str,
        max_bytes: Option<usize>,
    ) -> Result<SessionExplorerFileContent, String> {
        self.ensure_session_exists(session_id)?;
        let Some(state) = self.store.get_session_explorer_state(session_id)? else {
            return Err("session explorer root is not set".to_string());
        };

        let root = resolve_explorer_root_path(&state.root_path)?;
        let normalized = normalize_relative_path(relative_path)?;
        let target = resolve_explorer_target(&root, &normalized)?;
        if !target.is_file() {
            return Err("explorer target is not a file".to_string());
        }

        let max_bytes = max_bytes
            .unwrap_or(256 * 1024)
            .clamp(1_024, MAX_EXPLORER_FILE_PREVIEW_BYTES);
        let file =
            File::open(&target).map_err(|err| format!("open explorer file failed: {err}"))?;
        let mut buffer = Vec::new();
        file.take((max_bytes + 1) as u64)
            .read_to_end(&mut buffer)
            .map_err(|err| format!("read explorer file failed: {err}"))?;

        let truncated = buffer.len() > max_bytes;
        if truncated {
            buffer.truncate(max_bytes);
        }
        if buffer.contains(&0) {
            return Err("binary file preview is not supported yet".to_string());
        }

        Ok(SessionExplorerFileContent {
            relative_path: normalized,
            content: String::from_utf8_lossy(&buffer).to_string(),
            truncated,
            byte_len: buffer.len(),
        })
    }

    fn ensure_session_exists(&self, session_id: &str) -> Result<(), String> {
        if self.sessions.contains_key(session_id) {
            return Ok(());
        }
        Err("session not found".to_string())
    }
}
