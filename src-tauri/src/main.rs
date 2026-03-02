mod config;
mod models;
mod persistence;
mod service;

use std::sync::Arc;

use models::{
    ActivateSessionPayload, CreateProfilePayload, CreateSessionPayload, CreateSessionResponse,
    DeleteProfilePayload, ProfileInfo, RenameProfilePayload, RenameSessionPayload,
    ResizeSessionPayload, SessionActionPayload, SessionInfo, SessionSnapshot,
    SetSessionPersistPayload, SwitchProfilePayload, WorkspaceState, WriteInputPayload,
};
use service::PtyService;
use tauri::{Manager, State};

struct AppState {
    service: Arc<PtyService>,
}

#[tauri::command]
fn list_sessions(state: State<'_, AppState>) -> Vec<SessionInfo> {
    state.service.list_sessions()
}

#[tauri::command]
fn list_profiles(state: State<'_, AppState>) -> Vec<ProfileInfo> {
    state.service.list_profiles()
}

#[tauri::command]
fn load_workspace(state: State<'_, AppState>) -> WorkspaceState {
    state.service.load_workspace()
}

#[tauri::command]
fn create_profile(
    state: State<'_, AppState>,
    payload: CreateProfilePayload,
) -> Result<ProfileInfo, String> {
    state.service.create_profile(payload)
}

#[tauri::command]
fn switch_profile(
    state: State<'_, AppState>,
    payload: SwitchProfilePayload,
) -> Result<WorkspaceState, String> {
    state.service.switch_profile(payload)
}

#[tauri::command]
fn rename_profile(
    state: State<'_, AppState>,
    payload: RenameProfilePayload,
) -> Result<ProfileInfo, String> {
    state.service.rename_profile(payload)
}

#[tauri::command]
fn delete_profile(
    state: State<'_, AppState>,
    payload: DeleteProfilePayload,
) -> Result<WorkspaceState, String> {
    state.service.delete_profile(payload)
}

#[tauri::command]
fn create_session(
    state: State<'_, AppState>,
    payload: CreateSessionPayload,
) -> Result<CreateSessionResponse, String> {
    state.service.create_session(payload)
}

#[tauri::command]
fn activate_session(
    state: State<'_, AppState>,
    payload: ActivateSessionPayload,
) -> Result<(), String> {
    state.service.activate_session(payload)
}

#[tauri::command]
fn write_input(state: State<'_, AppState>, payload: WriteInputPayload) -> Result<(), String> {
    state.service.write_input(payload)
}

#[tauri::command]
fn resize_session(state: State<'_, AppState>, payload: ResizeSessionPayload) -> Result<(), String> {
    state.service.resize_session(payload)
}

#[tauri::command]
fn rename_session(state: State<'_, AppState>, payload: RenameSessionPayload) -> Result<(), String> {
    state.service.rename_session(payload)
}

#[tauri::command]
fn set_session_persist(
    state: State<'_, AppState>,
    payload: SetSessionPersistPayload,
) -> Result<(), String> {
    state.service.set_session_persist(payload)
}

#[tauri::command]
fn close_session(state: State<'_, AppState>, payload: SessionActionPayload) -> Result<(), String> {
    state.service.close_session(payload)
}

#[tauri::command]
fn clear_session_history(
    state: State<'_, AppState>,
    payload: SessionActionPayload,
) -> Result<(), String> {
    state.service.clear_session_history(payload)
}

#[tauri::command]
fn clear_all_history(state: State<'_, AppState>) -> Result<(), String> {
    state.service.clear_all_history()
}

#[tauri::command]
fn get_session_snapshot(
    state: State<'_, AppState>,
    payload: SessionActionPayload,
) -> Result<SessionSnapshot, String> {
    state.service.get_session_snapshot(payload)
}

fn main() {
    let _ = env_logger::try_init();

    tauri::Builder::default()
        .setup(|app| {
            let config = config::load_config();
            let service = Arc::new(PtyService::new(app.handle().clone(), config));
            app.manage(AppState { service });
            Ok(())
        })
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            load_workspace,
            list_profiles,
            list_sessions,
            create_profile,
            switch_profile,
            rename_profile,
            delete_profile,
            create_session,
            activate_session,
            write_input,
            resize_session,
            rename_session,
            set_session_persist,
            close_session,
            clear_session_history,
            clear_all_history,
            get_session_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
