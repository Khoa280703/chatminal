mod chatminald_client;
mod config;
mod models;
mod persistence;
mod runtime_backend;
mod service;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use models::{
    ActivateSessionPayload, CreateProfilePayload, CreateSessionPayload, CreateSessionResponse,
    DeleteProfilePayload, LifecyclePreferences, ProfileInfo, RenameProfilePayload,
    RenameSessionPayload, ResizeSessionPayload, RuntimeBackendInfo, RuntimeBackendPing,
    RuntimeUiSettings, SessionActionPayload, SessionInfo, SessionSnapshot,
    SetLifecyclePreferencesPayload, SetSessionPersistPayload, SwitchProfilePayload, WorkspaceState,
    WriteInputPayload,
};
use runtime_backend::RuntimeBackend;
use service::PtyService;
use tauri::{Emitter, Manager, State, WindowEvent, menu::MenuBuilder, tray::TrayIconBuilder};

const TRAY_SHOW_ID: &str = "tray_show";
const TRAY_NEW_SESSION_ID: &str = "tray_new_session";
const TRAY_QUIT_ID: &str = "tray_quit";

struct AppState {
    service: Arc<PtyService>,
    runtime_backend: RuntimeBackend,
    is_quitting: AtomicBool,
}

fn show_main_window(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    window
        .show()
        .map_err(|err| format!("show window failed: {err}"))?;
    window
        .set_focus()
        .map_err(|err| format!("focus window failed: {err}"))?;
    Ok(())
}

fn request_quit(app: &tauri::AppHandle, state: &AppState) -> Result<(), String> {
    if state.is_quitting.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    if let Err(err) = state.service.shutdown_graceful() {
        log::warn!("graceful shutdown failed: {err}");
    }

    app.exit(0);
    Ok(())
}

fn build_tray(app: &tauri::AppHandle) -> Result<(), String> {
    let menu = MenuBuilder::new(app)
        .text(TRAY_SHOW_ID, "Show Chatminal")
        .text(TRAY_NEW_SESSION_ID, "New Session")
        .separator()
        .text(TRAY_QUIT_ID, "Quit Completely")
        .build()
        .map_err(|err| format!("build tray menu failed: {err}"))?;

    TrayIconBuilder::with_id("chatminal-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_ID => {
                if let Err(err) = show_main_window(app) {
                    log::warn!("show main window failed: {err}");
                }
            }
            TRAY_NEW_SESSION_ID => {
                if let Err(err) = show_main_window(app) {
                    log::warn!("show main window before new session failed: {err}");
                }
                let _ = app.emit("app/tray-new-session", ());
            }
            TRAY_QUIT_ID => {
                let state = app.state::<AppState>();
                if let Err(err) = request_quit(app, &state) {
                    log::warn!("quit from tray failed: {err}");
                }
            }
            _ => {}
        })
        .build(app)
        .map_err(|err| format!("build tray icon failed: {err}"))?;

    Ok(())
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
fn get_lifecycle_preferences(state: State<'_, AppState>) -> Result<LifecyclePreferences, String> {
    state.service.get_lifecycle_preferences()
}

#[tauri::command]
fn set_lifecycle_preferences(
    state: State<'_, AppState>,
    payload: SetLifecyclePreferencesPayload,
) -> Result<LifecyclePreferences, String> {
    state.service.set_lifecycle_preferences(payload)
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

#[tauri::command]
fn shutdown_app(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    request_quit(&app, &state)
}

#[tauri::command]
fn get_runtime_backend_info(state: State<'_, AppState>) -> RuntimeBackendInfo {
    state.runtime_backend.info()
}

#[tauri::command]
fn ping_runtime_backend(state: State<'_, AppState>) -> RuntimeBackendPing {
    state.runtime_backend.ping()
}

#[tauri::command]
fn get_runtime_ui_settings(state: State<'_, AppState>) -> RuntimeUiSettings {
    state.service.runtime_ui_settings()
}

fn main() {
    let _ = env_logger::try_init();

    tauri::Builder::default()
        .setup(|app| {
            let config = config::load_config();
            let service = Arc::new(PtyService::new(app.handle().clone(), config));
            let runtime_backend = RuntimeBackend::from_env();
            let lifecycle_preferences = service.get_lifecycle_preferences().unwrap_or_default();
            app.manage(AppState {
                service,
                runtime_backend,
                is_quitting: AtomicBool::new(false),
            });

            build_tray(&app.handle())?;

            if lifecycle_preferences.start_in_tray
                && let Some(window) = app.get_webview_window("main")
            {
                let _ = window.hide();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.is_quitting.load(Ordering::SeqCst) {
                    return;
                }

                let keep_alive_on_close = state
                    .service
                    .get_lifecycle_preferences()
                    .map(|prefs| prefs.keep_alive_on_close)
                    .unwrap_or(true);

                if keep_alive_on_close {
                    api.prevent_close();
                    let _ = window.hide();
                    let _ = window.app_handle().emit("app/lifecycle-hidden", ());
                }
            }
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
            get_lifecycle_preferences,
            set_lifecycle_preferences,
            close_session,
            clear_session_history,
            clear_all_history,
            get_session_snapshot,
            get_runtime_backend_info,
            ping_runtime_backend,
            get_runtime_ui_settings,
            shutdown_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
