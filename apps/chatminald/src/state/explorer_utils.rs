use std::path::{Path, PathBuf};

use chatminal_protocol::SessionExplorerState;
use chatminal_store::StoredSessionExplorerState;

pub(super) fn explorer_state_to_protocol(
    session_id: &str,
    state: Option<StoredSessionExplorerState>,
) -> SessionExplorerState {
    if let Some(value) = state {
        return SessionExplorerState {
            session_id: value.session_id,
            root_path: Some(value.root_path),
            current_dir: value.current_dir,
            selected_path: value.selected_path,
            open_file_path: value.open_file_path,
        };
    }

    SessionExplorerState {
        session_id: session_id.to_string(),
        root_path: None,
        current_dir: String::new(),
        selected_path: None,
        open_file_path: None,
    }
}

pub(super) fn normalize_relative_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err("absolute path is not allowed in session explorer".to_string());
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(value) => normalized.push(value),
            std::path::Component::ParentDir => {
                return Err("parent path '..' is not allowed in session explorer".to_string());
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err("invalid path component in session explorer".to_string());
            }
        }
    }

    Ok(normalized.to_string_lossy().replace('\\', "/"))
}

pub(super) fn resolve_explorer_root_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("explorer root path cannot be empty".to_string());
    }

    let canonical = std::fs::canonicalize(trimmed)
        .map_err(|err| format!("invalid explorer root '{trimmed}': {err}"))?;
    if !canonical.is_dir() {
        return Err("explorer root is not a directory".to_string());
    }
    Ok(canonical)
}

pub(super) fn resolve_explorer_target(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let canonical_root = std::fs::canonicalize(root)
        .map_err(|err| format!("invalid explorer root '{}': {err}", root.display()))?;
    let normalized = normalize_relative_path(relative)?;
    let joined = if normalized.is_empty() {
        canonical_root.clone()
    } else {
        canonical_root.join(normalized)
    };

    let canonical = std::fs::canonicalize(&joined)
        .map_err(|err| format!("invalid explorer path '{}': {err}", joined.display()))?;
    if !canonical.starts_with(&canonical_root) {
        return Err("explorer path escapes selected root".to_string());
    }
    Ok(canonical)
}
