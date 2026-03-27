use crate::chatminal_runtime::{
    active_frontend_client, active_workspace_for_client, focus_terminal_handle_by_id,
    frontend_resolve_focused_pane, frontend_resolve_pane, set_active_workspace_for_client,
    subscribe_frontend_notifications, workspace_is_empty, workspace_names,
    FrontendClientHandle, RuntimeNotification, host_workspace_has_windows,
    primary_host_window_exists, primary_host_window_id,
};
use crate::scripting::guiwin::PrimaryGuiWindowId;
use crate::scripting::guiwin::GuiWin;
use crate::spawn::SpawnWhere;
use crate::TermWindow;
use ::window::*;
use anyhow::{Context, Error};
use config::keyassignment::{KeyAssignment, SpawnCommand};
use config::{ConfigSubscription, NotificationHandling};
use engine_term::{Alert, ClipboardSelection};
use engine_toast_notification::*;
use promise::{Future, Promise};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub struct GuiFrontEnd {
    connection: Rc<Connection>,
    switching_workspaces: RefCell<bool>,
    closing_primary_window: RefCell<bool>,
    spawning_primary_window: RefCell<bool>,
    known_window: RefCell<Option<Window>>,
    client_id: FrontendClientHandle,
    config_subscription: RefCell<Option<ConfigSubscription>>,
    _host_activity_guard: Option<crate::chatminal_runtime::HostActivityGuard>,
}

impl Drop for GuiFrontEnd {
    fn drop(&mut self) {
        ::window::shutdown();
    }
}

impl GuiFrontEnd {
    pub fn try_new() -> anyhow::Result<Rc<GuiFrontEnd>> {
        let connection = Connection::init()?;
        connection.set_event_handler(Self::app_event_handler);

        let client_id = active_frontend_client().expect("to have set my own id");

        let front_end = Rc::new(GuiFrontEnd {
            connection,
            switching_workspaces: RefCell::new(false),
            closing_primary_window: RefCell::new(false),
            spawning_primary_window: RefCell::new(false),
            known_window: RefCell::new(None),
            client_id: client_id.clone(),
            config_subscription: RefCell::new(None),
            _host_activity_guard: crate::chatminal_sidebar::sidebar_enabled_from_env()
                .then(crate::chatminal_runtime::start_host_activity),
        });

        subscribe_frontend_notifications(move |n| {
            match n {
                RuntimeNotification::WorkspaceRenamed {
                    old_workspace,
                    new_workspace,
                } => {
                    let active = crate::chatminal_runtime::host_workspace_name();
                    if active == old_workspace || active == new_workspace {
                        let switcher = WorkspaceSwitcher::new(&new_workspace);
                        promise::spawn::spawn_into_main_thread(async move {
                            drop(switcher);
                        })
                        .detach();
                    }
                }
                RuntimeNotification::WindowWorkspaceChanged
                | RuntimeNotification::ActiveWorkspaceChanged(_) => {
                    promise::spawn::spawn_into_main_thread(async move {
                        let fe = crate::frontend::front_end();
                        if !fe.is_switching_workspace() {
                            fe.reconcile_workspace();
                        }
                    })
                    .detach();
                }
                RuntimeNotification::PaneFocused(pane_id) => {
                    promise::spawn::spawn_into_main_thread(async move {
                        if let Err(err) = focus_terminal_handle_by_id(pane_id as u64) {
                            log::error!("Error reconciling PaneFocused notification: {err:#}");
                        }
                    })
                    .detach();
                }
                RuntimeNotification::TabTitleChanged { .. } => {}
                RuntimeNotification::WindowTitleChanged { .. } => {}
                RuntimeNotification::TabResized(_) => {}
                RuntimeNotification::TabAddedToWindow { .. } => {}
                RuntimeNotification::PaneRemoved(_) => {}
                RuntimeNotification::WindowInvalidated => {}
                RuntimeNotification::PaneOutput(_) => {}
                RuntimeNotification::PaneAdded(_) => {}
                RuntimeNotification::Alert {
                    pane_id,
                    alert:
                        Alert::ToastNotification {
                            title,
                            body,
                            focus: _,
                        },
                } => {
                    if let Some(resolved) = frontend_resolve_pane(pane_id as u64) {
                        let config = config::configuration();

                        if let Some(focused) = frontend_resolve_focused_pane(&client_id) {
                            let show = match config.notification_handling {
                                NotificationHandling::NeverShow => false,
                                NotificationHandling::AlwaysShow => true,
                                NotificationHandling::SuppressFromFocusedPane => {
                                    focused.pane_id != pane_id as u64
                                }
                                NotificationHandling::SuppressFromFocusedTab => {
                                    focused.runtime_entry_id != resolved.runtime_entry_id
                                }
                                NotificationHandling::SuppressFromFocusedWindow => false,
                            };

                            if show {
                                let message = if title.is_none() { "" } else { &body };
                                let title = title.as_ref().unwrap_or(&body);
                                // FIXME: if notification.focus is true, we should do
                                // something here to arrange to focus pane_id when the
                                // notification is clicked
                                persistent_toast_notification(title, message);
                            }
                        }
                    }
                }
                RuntimeNotification::Alert {
                    pane_id: _,
                    alert: Alert::Bell | Alert::Progress(_),
                } => {
                    // Handled via TermWindowNotif; NOP it here.
                }
                RuntimeNotification::Alert {
                    pane_id: _,
                    alert:
                        Alert::OutputSinceFocusLost
                        | Alert::PaletteChanged
                        | Alert::CurrentWorkingDirectoryChanged
                        | Alert::WindowTitleChanged(_)
                        | Alert::TabTitleChanged(_)
                        | Alert::IconTitleChanged(_)
                        | Alert::SetUserVar { .. },
                } => {}
                RuntimeNotification::Empty => {
                    if config::configuration().quit_when_all_windows_are_closed {
                        promise::spawn::spawn_into_main_thread(async move {
                            let activity_count = crate::chatminal_runtime::host_activity_count();
                            let should_terminate = crate::frontend::try_front_end()
                                .as_ref()
                                .map(|fe| fe.can_terminate_on_mux_empty())
                                .unwrap_or(false);
                            if activity_count == 0 && should_terminate {
                                log::trace!("Mux is now empty, terminate gui");
                                Connection::get().unwrap().terminate_message_loop();
                            } else {
                                log::debug!(
                                    "skip gui terminate on mux empty: activity_count={activity_count}, should_terminate={should_terminate}"
                                );
                            }
                        })
                        .detach();
                    }
                }
                RuntimeNotification::SaveToDownloads { name, data } => {
                    if !config::configuration().allow_download_protocols {
                        log::error!(
                            "Ignoring download request for {:?}, \
                                 as allow_download_protocols=false",
                            name
                        );
                    } else if let Err(err) = crate::download::save_to_downloads(name, &*data) {
                        log::error!("save_to_downloads: {:#}", err);
                    }
                }
                RuntimeNotification::AssignClipboard {
                    pane_id,
                    selection,
                    clipboard,
                } => {
                    promise::spawn::spawn_into_main_thread(async move {
                        let fe = crate::frontend::front_end();
                        log::trace!(
                            "set clipboard in pane {} {:?} {:?}",
                            pane_id,
                            selection,
                            clipboard
                        );
                        if let Some(window) = fe.known_window.borrow().as_ref() {
                            window.set_clipboard(
                                match selection {
                                    ClipboardSelection::Clipboard => Clipboard::Clipboard,
                                    ClipboardSelection::PrimarySelection => {
                                        Clipboard::PrimarySelection
                                    }
                                },
                                clipboard.unwrap_or_else(String::new),
                            );
                        } else {
                            log::error!("Cannot assign clipboard as there are no windows");
                        };
                    })
                    .detach();
                }
            }
            true
        });
        // Re-evaluate the config so that folks that are using
        // `chatminal.gui.get_appearance()` can have that take effect
        // before any windows are created
        config::reload();

        // And build the initial menu bar.
        // TODO: arrange for this to happen on config reload.
        crate::commands::CommandDef::recreate_menubar(&config::configuration());

        Ok(front_end)
    }

    fn app_event_handler(event: ApplicationEvent) {
        log::trace!("Got app event {event:?}");
        match event {
            ApplicationEvent::OpenCommandScript(file_name) => {
                let quoted_file_name = match shlex::try_quote(&file_name) {
                    Ok(name) => name.to_owned().to_string(),
                    Err(_) => {
                        log::error!(
                            "OpenCommandScript: {file_name} has embedded NUL bytes and
                             cannot be launched via the shell"
                        );
                        return;
                    }
                };
                promise::spawn::spawn(async move {
                    // We send the script to execute to the shell on stdin, rather than ask the
                    // shell to execute it directly, so that we start the shell and read in the
                    // user's rc files before running the script.  Without this, Chatminal on macOS
                    // is launched with a default and very anemic path, and that is frustrating for
                    // users.

                    match crate::chatminal_runtime::spawn_local_shell_runner().await {
                        Ok(pane) => {
                            log::trace!("Spawned {file_name} as pane_id {}", pane.pane_id());
                            let mut writer = pane.writer();
                            write!(writer, "{quoted_file_name} ; exit\n").ok();
                        }
                        Err(err) => {
                            log::error!("Failed to spawn {file_name}: {err:#?}");
                        }
                    };
                })
                .detach();
            }
            ApplicationEvent::PerformKeyAssignment(action) => {
                // We should only get here when there are no windows open
                // and the user picks an action from the menubar.
                // This is not currently possible, but could be in the
                // future.

                fn spawn_command(spawn: &SpawnCommand, spawn_where: SpawnWhere) {
                    let config = config::configuration();
                    let dpi = config.dpi.unwrap_or_else(|| ::window::default_dpi());
                    let size =
                        config.initial_size(dpi as u32, crate::cell_pixel_dims(&config, dpi).ok());
                    let term_config = Arc::new(config::TermConfig::with_config(config));

                    crate::spawn::spawn_command_impl(spawn, spawn_where, size, term_config)
                }

                match action {
                    KeyAssignment::QuitApplication => {
                        // If we get here, there are no windows that could have received
                        // the QuitApplication command, therefore it must be ok to quit
                        // immediately
                        Connection::get().unwrap().terminate_message_loop();
                    }
                    KeyAssignment::SpawnSession => {
                        spawn_command(&SpawnCommand::default(), SpawnWhere::InitialWindow);
                    }
                    KeyAssignment::SpawnCommandInNewSession(spawn) => {
                        spawn_command(&spawn, SpawnWhere::NewSession);
                    }
                    _ => {
                        log::warn!("unhandled perform: {action:?}");
                    }
                }
            }
        }
    }

    pub fn run_forever(&self) -> anyhow::Result<()> {
        self.connection
            .run_message_loop()
            .context("running message loop")
    }

    pub fn gui_windows(&self) -> Vec<GuiWin> {
        let active_workspace = active_workspace_for_client(&self.client_id);
        self.known_window
            .borrow()
            .as_ref()
            .map(|window| {
                vec![GuiWin {
                    active_workspace,
                    primary_window_id: primary_host_window_id(),
                    window: window.clone(),
                }]
            })
            .unwrap_or_default()
    }

    pub fn reconcile_workspace(&self) -> Future<()> {
        let mut promise = Promise::new();
        let workspace = active_workspace_for_client(&self.client_id);

        if workspace_is_empty(&workspace) {
            // We don't want to silently kill off things that might
            // be running in other workspaces, so let's pick one
            // and activate it
            if self.is_switching_workspace() {
                promise.ok(());
                return promise.get_future().unwrap();
            }
            for workspace in workspace_names() {
                if !workspace_is_empty(&workspace) {
                    set_active_workspace_for_client(&self.client_id, &workspace);
                    log::debug!("using {} instead, as it is not empty", workspace);
                    break;
                }
            }
        }

        let workspace = active_workspace_for_client(&self.client_id);
        log::debug!("workspace is {}, fixup windows", workspace);

        self.prune_closing_primary_window();
        let desired_primary_window_id =
            host_workspace_has_windows(&workspace).then(primary_host_window_id);

        let known_window = self.known_window.borrow_mut().take();

        if let Some(window) = known_window {
            if desired_primary_window_id.is_none() {
                window.close();
                *front_end().spawning_primary_window.borrow_mut() = false;
            } else {
                *self.known_window.borrow_mut() = Some(window);
            }
        }

        let future = promise.get_future().unwrap();

        promise::spawn::spawn(async move {
            if front_end().known_window.borrow().is_none() {
                if let Some(primary_window_id) = desired_primary_window_id {
                    if front_end().is_closing_primary_window(primary_window_id) {
                        log::debug!(
                            "frontend reconcile: skip respawn for closing primary window {primary_window_id}"
                        );
                    } else if !*front_end().spawning_primary_window.borrow() {
                        *front_end().spawning_primary_window.borrow_mut() = true;
                        log::trace!(
                            "Creating TermWindow for primary_window_id={}",
                            primary_window_id
                        );
                        if let Err(err) =
                            TermWindow::new_primary_window(primary_window_id).await
                        {
                            log::error!("Failed to create window: {:#}", err);
                            *front_end().spawning_primary_window.borrow_mut() = false;
                        }
                    }
                }
            }
            *front_end().switching_workspaces.borrow_mut() = false;
            promise.ok(());
        })
        .detach();
        future
    }

    fn has_live_host_windows(&self) -> bool {
        primary_host_window_exists()
    }

    fn can_terminate_on_mux_empty(&self) -> bool {
        self.prune_closing_primary_window();
        if self.known_window.borrow().is_some() || *self.spawning_primary_window.borrow() {
            return false;
        }
        !self.has_live_host_windows()
    }

    pub fn switch_workspace(&self, workspace: &str) {
        set_active_workspace_for_client(&self.client_id, workspace);
        *self.switching_workspaces.borrow_mut() = false;
        self.reconcile_workspace();
    }

    pub fn record_primary_window_binding(
        &self,
        window: Window,
        primary_window_id: PrimaryGuiWindowId,
    ) {
        if primary_window_id != primary_host_window_id() {
            log::warn!(
                "single-window mode: closing unexpected GUI binding for non-primary primary_window_id={primary_window_id}"
            );
            window.close();
            return;
        }
        *self.closing_primary_window.borrow_mut() = false;
        *self.spawning_primary_window.borrow_mut() = false;
        {
            let mut known_window = self.known_window.borrow_mut();
            if known_window
                .as_ref()
                .is_some_and(|existing| existing != &window)
            {
                log::warn!(
                    "single-window mode: closing unexpected extra GUI window binding for primary_window_id={primary_window_id}"
                );
                drop(known_window);
                window.close();
                return;
            }
            *known_window = Some(window);
        }
        crate::commands::CommandDef::recreate_menubar(&config::configuration());
        if !self.is_switching_workspace() {
            self.reconcile_workspace();
        }
    }

    pub fn forget_known_window(&self, window: &Window) {
        let should_forget = self
            .known_window
            .borrow()
            .as_ref()
            .is_some_and(|known_window| known_window == window);
        if should_forget {
            *self.known_window.borrow_mut() = None;
            *self.closing_primary_window.borrow_mut() = true;
            *self.spawning_primary_window.borrow_mut() = false;
        }
        crate::commands::CommandDef::recreate_menubar(&config::configuration());
        if !self.is_switching_workspace() {
            self.reconcile_workspace();
        }
    }

    pub fn is_switching_workspace(&self) -> bool {
        *self.switching_workspaces.borrow()
    }

    fn is_closing_primary_window(&self, primary_window_id: PrimaryGuiWindowId) -> bool {
        primary_window_id == primary_host_window_id() && *self.closing_primary_window.borrow()
    }

    fn prune_closing_primary_window(&self) {
        if !crate::chatminal_runtime::host_window_exists() {
            *self.closing_primary_window.borrow_mut() = false;
        }
    }

    pub fn gui_window(&self) -> Option<GuiWin> {
        let active_workspace = active_workspace_for_client(&self.client_id);
        self.known_window.borrow().as_ref().map(|window| GuiWin {
            active_workspace,
            primary_window_id: primary_host_window_id(),
            window: window.clone(),
        })
    }
}

thread_local! {
    static FRONT_END: RefCell<Option<Rc<GuiFrontEnd>>> = RefCell::new(None);
}

pub fn try_front_end() -> Option<Rc<GuiFrontEnd>> {
    FRONT_END.with(|f| f.borrow().as_ref().map(Rc::clone))
}

pub fn front_end() -> Rc<GuiFrontEnd> {
    FRONT_END
        .with(|f| f.borrow().as_ref().map(Rc::clone))
        .expect("to be called on gui thread")
}

pub struct WorkspaceSwitcher {
    new_name: String,
}

impl WorkspaceSwitcher {
    pub fn new(new_name: &str) -> Self {
        *front_end().switching_workspaces.borrow_mut() = true;
        Self {
            new_name: new_name.to_string(),
        }
    }

    pub fn do_switch(self) {
        // Drop is invoked, which will complete the switch
    }
}

impl Drop for WorkspaceSwitcher {
    fn drop(&mut self) {
        front_end().switch_workspace(&self.new_name);
    }
}

pub fn shutdown() {
    FRONT_END.with(|f| drop(f.borrow_mut().take()));
}

pub fn try_new() -> Result<Rc<GuiFrontEnd>, Error> {
    let front_end = GuiFrontEnd::try_new()?;
    FRONT_END.with(|f| *f.borrow_mut() = Some(Rc::clone(&front_end)));

    let config_subscription = config::subscribe_to_config_reload({
        move || {
            promise::spawn::spawn_into_main_thread(async {
                crate::commands::CommandDef::recreate_menubar(&config::configuration());
            })
            .detach();
            true
        }
    });
    front_end
        .config_subscription
        .borrow_mut()
        .replace(config_subscription);

    Ok(front_end)
}
