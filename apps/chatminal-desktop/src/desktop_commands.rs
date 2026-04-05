use crate::chatminal_runtime::desktop_current_active_session_id;
use crate::inputmap::InputMap;
use KeyAssignment::*;
use config::keyassignment::*;
use config::window::WindowLevel;
use config::{ConfigHandle, DeferredKeyCode};
use ordered_float::NotNan;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::convert::TryFrom;
use window::{KeyCode, Modifiers};

// Compatibility translation layer for upstream-style command/config names.
// Product-facing desktop code should consume SessionBarAssignment and other
// Chatminal vocabulary instead of routing KeyAssignment::*Tab* directly.

/// Describes an argument/parameter/context that is required
/// in order for the command to have meaning.
/// The intent is for this to be used when filtering the items
/// that should be shown in eg: a context menu.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArgType {
    /// Operates on the active pane
    ActiveTerminal,
    /// Operates on the active tab
    ActiveSession,
    /// Operates on the active window
    ActiveWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionBarAssignment {
    ActivateRelative { delta: isize, wrap: bool },
    ActivateLast,
    ActivateIndex(isize),
    MoveTo(usize),
    MoveRelative(isize),
}

pub fn session_bar_assignment_for_key_assignment(
    assignment: &KeyAssignment,
) -> Option<SessionBarAssignment> {
    match assignment {
        KeyAssignment::ActivateSessionRelative(delta) => {
            Some(SessionBarAssignment::ActivateRelative {
                delta: *delta,
                wrap: true,
            })
        }
        KeyAssignment::ActivateSessionRelativeNoWrap(delta) => {
            Some(SessionBarAssignment::ActivateRelative {
                delta: *delta,
                wrap: false,
            })
        }
        KeyAssignment::ActivateLastSession => Some(SessionBarAssignment::ActivateLast),
        KeyAssignment::ActivateSession(index) => Some(SessionBarAssignment::ActivateIndex(*index)),
        KeyAssignment::MoveSession(index) => Some(SessionBarAssignment::MoveTo(*index)),
        KeyAssignment::MoveSessionRelative(delta) => {
            Some(SessionBarAssignment::MoveRelative(*delta))
        }
        _ => None,
    }
}

pub fn session_bar_activate_index_assignment(index: isize) -> KeyAssignment {
    KeyAssignment::ActivateSession(index)
}

pub fn is_session_bar_switching_key_assignment(assignment: &KeyAssignment) -> bool {
    matches!(
        session_bar_assignment_for_key_assignment(assignment),
        Some(
            SessionBarAssignment::ActivateRelative { .. }
                | SessionBarAssignment::ActivateLast
                | SessionBarAssignment::ActivateIndex(_)
        )
    )
}

pub fn is_supported_in_session_ui(assignment: &KeyAssignment) -> bool {
    !matches!(
        assignment,
        KeyAssignment::SessionSelect(SessionSelectArguments {
            mode: SessionSelectMode::SwapWithActive
                | SessionSelectMode::SwapWithActiveKeepFocus
                | SessionSelectMode::MoveToNewSession,
            ..
        }) | KeyAssignment::ActivateSessionDirection(_)
            | KeyAssignment::ToggleTerminalZoomState
            | KeyAssignment::SetTerminalZoomState(_)
            | KeyAssignment::AdjustSplitSize(_, _)
            | KeyAssignment::RotatePanes(_)
    )
}

pub fn retain_supported_for_session_ui(commands: &mut Vec<ExpandedCommand>, session_ui_mode: bool) {
    if session_ui_mode {
        commands.retain(|cmd| is_supported_in_session_ui(&cmd.action));
    }
}

#[cfg(target_os = "macos")]
fn session_ui_mode_for_menubar() -> bool {
    crate::frontend::try_front_end()
        .map(|front_end| {
            front_end.gui_windows().into_iter().any(|window| {
                let _ = window;
                desktop_current_active_session_id().is_some()
            })
        })
        .unwrap_or(false)
}

/// A helper function used to synthesize key binding permutations.
/// If the input is a character on a US ANSI keyboard layout, returns
/// the the typical character that is produced when holding down
/// the shift key and pressing the original key.
/// This doesn't produce an exhaustive list because there are only
/// a handful of default assignments in the command DEFS below.
fn us_layout_shift(s: &str) -> String {
    match s {
        "1" => "!".to_string(),
        "2" => "@".to_string(),
        "3" => "#".to_string(),
        "4" => "$".to_string(),
        "5" => "%".to_string(),
        "6" => "^".to_string(),
        "7" => "&".to_string(),
        "8" => "*".to_string(),
        "9" => "(".to_string(),
        "0" => ")".to_string(),
        "[" => "{".to_string(),
        "]" => "}".to_string(),
        "=" => "+".to_string(),
        "-" => "_".to_string(),
        "'" => "\"".to_string(),
        s if s.len() == 1 => s.to_ascii_uppercase(),
        s => s.to_string(),
    }
}

/// `CommandDef` defines a command in the UI.
pub struct CommandDef {
    /// Brief description
    pub brief: Cow<'static, str>,
    /// A longer, more detailed, description
    pub doc: Cow<'static, str>,
    /// The key assignments associated with this command.
    pub keys: Vec<(Modifiers, String)>,
    /// The argument types/context in which this command is valid.
    pub args: &'static [ArgType],
    /// Where to place the command in a menubar
    pub menubar: &'static [&'static str],
    pub icon: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct ExpandedCommand {
    pub brief: Cow<'static, str>,
    pub doc: Cow<'static, str>,
    pub action: KeyAssignment,
    pub keys: Vec<(Modifiers, KeyCode)>,
    pub menubar: &'static [&'static str],
    pub icon: Option<Cow<'static, str>>,
}

impl std::fmt::Debug for CommandDef {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("CommandDef")
            .field("brief", &self.brief)
            .field("doc", &self.doc)
            .field("keys", &self.keys)
            .field("args", &self.args)
            .finish()
    }
}

impl CommandDef {
    /// Blech. Depending on the OS, a shifted key combination
    /// such as CTRL-SHIFT-L may present as either:
    /// CTRL+SHIFT + mapped lowercase l
    /// CTRL+SHIFT + mapped uppercase l
    /// CTRL       + mapped uppercase l
    ///
    /// This logic synthesizes the different combinations so
    /// that it isn't such a headache to maintain the mapping
    /// and prevents missing cases.
    ///
    /// Note that the mapped form of these things assumes
    /// US layout for some of the special shifted/punctuation cases.
    /// It's not perfect.
    ///
    /// The synthesis here requires that the defaults in
    /// the keymap below use the lowercase form of single characters!
    fn permute_keys(&self, config: &ConfigHandle) -> Vec<(Modifiers, KeyCode)> {
        let mut keys = vec![];

        for (mods, label) in &self.keys {
            let mods = *mods;
            let key = DeferredKeyCode::try_from(label.as_str())
                .unwrap()
                .resolve(config.key_map_preference)
                .clone();

            let ukey = DeferredKeyCode::try_from(us_layout_shift(&label))
                .unwrap()
                .resolve(config.key_map_preference)
                .clone();

            keys.push((mods, key.clone()));

            if mods == Modifiers::SUPER {
                // We want each SUPER/CMD version of the keys to also have
                // CTRL+SHIFT version(s) for environments where SUPER/CMD
                // is reserved for the window manager.
                // This bit synthesizes those.
                keys.push((Modifiers::CTRL | Modifiers::SHIFT, key.clone()));
                if ukey != key {
                    keys.push((Modifiers::CTRL | Modifiers::SHIFT, ukey.clone()));
                    keys.push((Modifiers::CTRL, ukey.clone()));
                }
            } else if mods.contains(Modifiers::SHIFT) && ukey != key {
                keys.push((mods, ukey.clone()));
                keys.push((mods - Modifiers::SHIFT, ukey.clone()));
            }
        }

        keys
    }

    /// Produces the list of default key assignments and actions.
    /// Used by the InputMap.
    pub fn default_key_assignments(
        config: &ConfigHandle,
    ) -> Vec<(Modifiers, KeyCode, KeyAssignment)> {
        let mut result = vec![];
        for cmd in Self::expanded_commands(config) {
            for (mods, code) in cmd.keys {
                result.push((mods, code.clone(), cmd.action.clone()));
            }
        }
        result
    }

    fn expand_action(
        action: KeyAssignment,
        config: &ConfigHandle,
        is_built_in: bool,
    ) -> Option<ExpandedCommand> {
        match derive_command_from_key_assignment(&action) {
            None => {
                if is_built_in {
                    log::warn!(
                        "{action:?} is a default action, but we cannot derive a CommandDef for it"
                    );
                }
                None
            }
            Some(def) => {
                let keys = if is_built_in && config.disable_default_key_bindings {
                    vec![]
                } else {
                    def.permute_keys(config)
                };
                Some(ExpandedCommand {
                    brief: def.brief.into(),
                    doc: def.doc.into(),
                    keys,
                    action,
                    menubar: def.menubar,
                    icon: def.icon.map(Cow::Borrowed),
                })
            }
        }
    }

    /// Produces the complete set of expanded commands.
    pub fn expanded_commands(config: &ConfigHandle) -> Vec<ExpandedCommand> {
        let mut result = vec![];

        for action in compute_default_actions() {
            if let Some(command) = Self::expand_action(action, config, true) {
                result.push(command);
            }
        }

        result
    }

    pub fn actions_for_palette_and_menubar_with_session_ui(
        config: &ConfigHandle,
        session_ui_mode: bool,
    ) -> Vec<ExpandedCommand> {
        let mut result = Self::expanded_commands(config);

        // Generate some stuff based on the config
        for cmd in &config.launch_menu {
            let label = match cmd.label.as_ref() {
                Some(label) => label.to_string(),
                None => match cmd.args.as_ref() {
                    Some(args) => args.join(" "),
                    None => "(default shell)".to_string(),
                },
            };
            result.push(ExpandedCommand {
                brief: format!("{label} (New Session)").into(),
                doc: "".into(),
                keys: vec![],
                action: KeyAssignment::SpawnCommandInNewSession(cmd.clone()),
                menubar: &["Shell"],
                icon: Some("md_tab_plus".into()),
            });
        }

        // And sweep to pick up stuff from their key assignments
        let inputmap = InputMap::new(config);
        for ((keycode, mods), entry) in inputmap.keys.default.iter() {
            if result
                .iter()
                .position(|cmd| cmd.action == entry.action)
                .is_some()
            {
                continue;
            }
            if let Some(cmd) = derive_command_from_key_assignment(&entry.action) {
                result.push(ExpandedCommand {
                    brief: cmd.brief.into(),
                    doc: cmd.doc.into(),
                    keys: vec![(*mods, keycode.clone())],
                    action: entry.action.clone(),
                    menubar: cmd.menubar,
                    icon: cmd.icon.map(Cow::Borrowed),
                });
            }
        }
        for table in inputmap.keys.by_name.values() {
            for entry in table.values() {
                if result
                    .iter()
                    .position(|cmd| cmd.action == entry.action)
                    .is_some()
                {
                    continue;
                }
                if let Some(cmd) = derive_command_from_key_assignment(&entry.action) {
                    result.push(ExpandedCommand {
                        brief: cmd.brief.into(),
                        doc: cmd.doc.into(),
                        keys: vec![],
                        action: entry.action.clone(),
                        menubar: cmd.menubar,
                        icon: cmd.icon.map(Cow::Borrowed),
                    });
                }
            }
        }

        retain_supported_for_session_ui(&mut result, session_ui_mode);
        result
    }

    #[cfg(not(target_os = "macos"))]
    pub fn recreate_menubar(_config: &ConfigHandle) {}

    /// Update the menubar to reflect the current config state.
    /// We cannot simply build a completely new one and replace it at runtime,
    /// because something in cocoa get's unhappy and crashes shortly after.
    /// The strategy we have is to try to find the existing item with the
    /// same action and update it.
    /// We use the macos menu item tag to do a mark-sweep style garbage
    /// collection to figure out which items were not reused/updated
    /// and remove them at the end.
    #[cfg(target_os = "macos")]
    pub fn recreate_menubar(config: &ConfigHandle) {
        use window::os::macos::menu::*;

        let inputmap = InputMap::new(config);

        let mut candidates_for_removal = vec![];
        #[allow(unexpected_cfgs)] // <https://github.com/SSheldon/rust-objc/issues/125>
        let chatminal_perform_key_assignment_sel = sel!(chatminalPerformKeyAssignment:);

        /// Mark menu items as candidates for removal
        fn mark_candidates(menu: &Menu, candidates: &mut Vec<MenuItem>, action: SEL) {
            for item in menu.items() {
                if let Some(submenu) = item.get_sub_menu() {
                    mark_candidates(&submenu, candidates, action);
                }
                if item.get_action() == Some(action) {
                    item.set_tag(0);
                    candidates.push(item);
                }
            }
        }

        fn prune_empty_submenus(menu: &Menu) {
            for item in menu.items() {
                if let Some(submenu) = item.get_sub_menu() {
                    prune_empty_submenus(&submenu);
                    if submenu.items().is_empty() {
                        menu.remove_item(&item);
                    }
                }
            }
        }

        let main_menu = match Menu::get_main_menu() {
            Some(existing) => {
                mark_candidates(
                    &existing,
                    &mut candidates_for_removal,
                    chatminal_perform_key_assignment_sel,
                );

                existing
            }
            None => {
                let menu = Menu::new_with_title("MainMenu");
                menu.assign_as_main_menu();
                menu
            }
        };

        let mut commands = Self::actions_for_palette_and_menubar_with_session_ui(
            config,
            session_ui_mode_for_menubar(),
        );
        commands.retain(|cmd| !cmd.menubar.is_empty());

        // Prefer to put the menus in this order
        let mut order: Vec<&'static str> = vec!["Chatminal", "Shell", "Edit", "View", "Window"];
        // Add any other menus on the end
        for cmd in &commands {
            if !order.contains(&cmd.menubar[0]) {
                order.push(cmd.menubar[0]);
            }
        }

        for &title in &order {
            for cmd in &commands {
                if cmd.menubar[0] != title {
                    continue;
                }

                let mut submenu = main_menu.get_or_create_sub_menu(&cmd.menubar[0], |menu| {
                    if cmd.menubar[0] == "Window" {
                    } else if cmd.menubar[0] == "Chatminal" {
                        menu.assign_as_app_menu();

                        let about_item = MenuItem::new_with(
                            &format!("Chatminal {}", config::engine_version()),
                            Some(chatminal_perform_key_assignment_sel),
                            "",
                        );
                        about_item.set_tool_tip("Click to copy version number");
                        about_item.set_represented_item(RepresentedItem::KeyAssignment(
                            KeyAssignment::CopyTextTo {
                                text: config::engine_version().to_string(),
                                destination: ClipboardCopyDestination::ClipboardAndPrimarySelection,
                            },
                        ));

                        menu.add_item(&about_item);
                        menu.add_item(&MenuItem::new_separator());

                        let services_menu = Menu::new_with_title("Services");
                        services_menu.assign_as_services_menu();
                        let services_item = MenuItem::new_with("Services", None, "");
                        menu.add_item(&services_item);
                        services_item.set_sub_menu(&services_menu);

                        menu.add_item(&MenuItem::new_separator());
                    } else if cmd.menubar[0] == "Help" {
                        menu.assign_as_help_menu();
                    }
                });

                // Fill out any submenu hierarchy
                for sub_title in cmd.menubar.iter().skip(1) {
                    submenu = submenu.get_or_create_sub_menu(sub_title, |_menu| {});
                }

                let mut candidate = inputmap.locate_app_wide_key_assignment(&cmd.action);
                candidate.sort_by(|(a_key, a_mods), (b_key, b_mods)| {
                    fn score_mods(mods: &Modifiers) -> usize {
                        let mut score: usize = mods.bits() as usize;
                        // Prefer keys with CMD on macOS
                        if mods.contains(Modifiers::SUPER) {
                            score += 1000;
                        }
                        score
                    }

                    let a_mods = score_mods(a_mods);
                    let b_mods = score_mods(b_mods);

                    match b_mods.cmp(&a_mods) {
                        Ordering::Equal => {}
                        ordering => return ordering,
                    }

                    a_key.cmp(&b_key)
                });

                fn key_code_to_equivalent(key: &KeyCode) -> String {
                    match key {
                        KeyCode::Hyper
                        | KeyCode::Super
                        | KeyCode::Meta
                        | KeyCode::Cancel
                        | KeyCode::Composed(_)
                        | KeyCode::RawCode(_) => "".to_string(),
                        KeyCode::Char(c) => c.to_string(),
                        KeyCode::Physical(phys) => key_code_to_equivalent(&phys.to_key_code()),
                        _ => "".to_string(),
                    }
                }

                let short_cut = candidate
                    .get(0)
                    .map(|(key, _)| key_code_to_equivalent(key))
                    .unwrap_or_else(String::new);

                let represented_item = RepresentedItem::KeyAssignment(cmd.action.clone());
                let item = match submenu.get_item_with_represented_item(&represented_item) {
                    Some(existing) => {
                        existing.set_title(&cmd.brief);
                        existing.set_key_equivalent(&short_cut);
                        existing
                    }
                    None => {
                        let item = MenuItem::new_with(
                            &cmd.brief,
                            Some(chatminal_perform_key_assignment_sel),
                            &short_cut,
                        );
                        submenu.add_item(&item);
                        item
                    }
                };

                if !short_cut.is_empty() {
                    let mods: Modifiers = candidate[0].1;
                    let mut equiv_mods = NSEventModifierFlags::empty();

                    equiv_mods.set(
                        NSEventModifierFlags::NSShiftKeyMask,
                        mods.contains(Modifiers::SHIFT),
                    );
                    equiv_mods.set(
                        NSEventModifierFlags::NSAlternateKeyMask,
                        mods.contains(Modifiers::ALT),
                    );
                    equiv_mods.set(
                        NSEventModifierFlags::NSControlKeyMask,
                        mods.contains(Modifiers::CTRL),
                    );
                    equiv_mods.set(
                        NSEventModifierFlags::NSCommandKeyMask,
                        mods.contains(Modifiers::SUPER),
                    );

                    item.set_key_equiv_modifier_mask(equiv_mods);
                }

                item.set_represented_item(represented_item);
                item.set_tool_tip(&cmd.doc);
                // Update the tag to indicate that this item should
                // not be removed by the sweep below
                item.set_tag(1);
            }
        }

        // Now sweep away any items that were not updated
        for item in candidates_for_removal {
            if item.get_tag() == 0 {
                item.get_menu().map(|menu| menu.remove_item(&item));
            }
        }

        prune_empty_submenus(&main_menu);
    }
}

/// Given "1" return "1st", "2" -> "2nd" and so on
fn english_ordinal(n: isize) -> String {
    let n = n.to_string();
    if n.ends_with('1') && !n.ends_with("11") {
        format!("{n}st")
    } else if n.ends_with('2') && !n.ends_with("12") {
        format!("{n}nd")
    } else if n.ends_with('3') && !n.ends_with("13") {
        format!("{n}rd")
    } else {
        format!("{n}th")
    }
}

fn spawn_command_from_action(action: &KeyAssignment) -> Option<&SpawnCommand> {
    match action {
        SplitSession(config::keyassignment::SplitSession { command, .. }) => Some(command),
        SplitHorizontal(command) | SplitVertical(command) | SpawnCommandInNewSession(command) => {
            Some(command)
        }
        _ => None,
    }
}

fn label_string(action: &KeyAssignment, candidate: String) -> String {
    if let Some(label) = spawn_command_from_action(action).and_then(|cmd| cmd.label_for_palette()) {
        label
    } else {
        candidate
    }
}

/// Describes a key assignment action; returns a bunch
/// of metadata that is useful in the command palette/menubar context.
/// This function will be called for the result of compute_default_actions(),
/// but can also be used to describe user-provided commands
pub fn derive_command_from_key_assignment(action: &KeyAssignment) -> Option<CommandDef> {
    Some(match action {
        PasteFrom(ClipboardPasteSource::PrimarySelection) => CommandDef {
            brief: "Paste primary selection".into(),
            doc: "Pastes text from the primary selection".into(),
            keys: vec![(Modifiers::SHIFT, "Insert".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit"],
            icon: Some("md_content_paste"),
        },
        CopyTextTo {
            text: _,
            destination: ClipboardCopyDestination::PrimarySelection,
        }
        | CopyTo(ClipboardCopyDestination::PrimarySelection) => CommandDef {
            brief: "Copy to primary selection".into(),
            doc: "Copies text to the primary selection".into(),
            keys: vec![(Modifiers::CTRL, "Insert".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit"],
            icon: Some("md_content_copy"),
        },
        CopyTextTo {
            text: _,
            destination: ClipboardCopyDestination::Clipboard,
        }
        | CopyTo(ClipboardCopyDestination::Clipboard) => CommandDef {
            brief: "Copy to clipboard".into(),
            doc: "Copies text to the clipboard".into(),
            keys: vec![
                (Modifiers::SUPER, "c".into()),
                (Modifiers::NONE, "Copy".into()),
            ],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit"],
            icon: Some("md_content_copy"),
        },
        CopyTextTo {
            text: _,
            destination: ClipboardCopyDestination::ClipboardAndPrimarySelection,
        }
        | CopyTo(ClipboardCopyDestination::ClipboardAndPrimarySelection) => CommandDef {
            brief: "Copy to clipboard and primary selection".into(),
            doc: "Copies text to the clipboard and the primary selection".into(),
            keys: vec![(Modifiers::CTRL, "Insert".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit"],
            icon: Some("md_content_copy"),
        },
        PasteFrom(ClipboardPasteSource::Clipboard) => CommandDef {
            brief: "Paste from clipboard".into(),
            doc: "Pastes text from the clipboard".into(),
            keys: vec![
                (Modifiers::SUPER, "v".into()),
                (Modifiers::NONE, "Paste".into()),
            ],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit"],
            icon: Some("md_content_paste"),
        },
        ToggleFullScreen => CommandDef {
            brief: "Toggle full screen mode".into(),
            doc: "Switch between normal and full screen mode".into(),
            keys: vec![(Modifiers::ALT, "Return".into())],
            args: &[ArgType::ActiveWindow],
            menubar: &["View"],
            icon: Some("md_fullscreen"),
        },
        ToggleAlwaysOnTop => CommandDef {
            brief: "Toggle always on Top".into(),
            doc: "Toggles the window between floating and non-floating states to stay on top of other windows.".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window"],
            icon: None,

        },
        ToggleAlwaysOnBottom => CommandDef {
            brief: "Toggle always on Bottom".into(),
            doc: "Toggles the window to remain behind all other windows.".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window"],
            icon: None,
        },
        SetWindowLevel(WindowLevel::AlwaysOnTop) => CommandDef {
            brief: "Always on Top".into(),
            doc: "Set the window level to be on top of other windows.".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window", "Level"],
            icon: None,
        },
        SetWindowLevel(WindowLevel::Normal) => CommandDef {
            brief: "Normal".into(),
            doc: "Set window level to normal".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window", "Level"],
            icon: None,
        },
        SetWindowLevel(WindowLevel::AlwaysOnBottom) => CommandDef {
            brief: "Always on Bottom".into(),
            doc: "Set window to remain behind all other windows.".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window", "Level"],
            icon: None,
        },
        Hide => CommandDef {
            brief: "Hide/Minimize Window".into(),
            doc: "Hides/Mimimizes the current window".into(),
            keys: vec![(Modifiers::SUPER, "m".into())],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window"],
            icon: Some("md_window_minimize"),
        },
        Show => CommandDef {
            brief: "Show/Restore Window".into(),
            doc: "Show/Restore the current window".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: Some("md_window_restore"),
        },
        HideApplication => CommandDef {
            brief: "Hide Application".into(),
            doc: "Hides all of the windows of the application. \
              This is macOS specific."
                .into(),
            keys: vec![(Modifiers::SUPER, "h".into())],
            args: &[],
            menubar: &["Chatminal"],
            icon: None,
        },
        Search(Pattern::CurrentSelectionOrEmptyString) => CommandDef {
            brief: "Search Terminal Output".into(),
            doc: "Search the visible terminal output and scrollback".into(),
            keys: vec![(Modifiers::SUPER, "f".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit"],
            icon: Some("oct_search"),
        },
        Search(_) => CommandDef {
            brief: "Search Terminal Output".into(),
            doc: "Search the visible terminal output and scrollback".into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &[],
            icon: Some("oct_search"),
        },
        ShowDebugOverlay => CommandDef {
            brief: "Show debug overlay".into(),
            doc: "Activates the debug overlay and Lua REPL".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "l".into())],
            args: &[ArgType::ActiveWindow],
            menubar: &["Help"],
            icon: Some("cod_debug"),
        },
        InputSelector(_) => CommandDef {
            brief: "Prompt the user to choose from a list".into(),
            doc: "Activates the selector overlay and wait for input".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: None,
        },
        Confirmation(_) => CommandDef {
            brief: "Prompt the user for confirmation".into(),
            doc: "Activates the confirmation overlay and wait for input".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: None,
        },
        PromptInputLine(_) => CommandDef {
            brief: "Prompt the user for a line of text".into(),
            doc: "Activates the prompt overlay and wait for input".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: None,
        },
        QuickSelect => CommandDef {
            brief: "Open Quick Select".into(),
            doc: "Find and jump to selectable text in the current terminal".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "Space".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit"],
            icon: None,
        },
        QuickSelectArgs(_) => CommandDef {
            brief: "Open Quick Select".into(),
            doc: "Find and jump to selectable text in the current terminal".into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &[],
            icon: None,
        },
        SessionSelect(SessionSelectArguments {
            mode: SessionSelectMode::SwapWithActive,
            ..
        }) => CommandDef {
            brief: "Swap With Another Pane".into(),
            doc: "Pick another pane and swap it with the current pane".into(),
            keys: vec![], // FIXME: find a new assignment
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window"],
            icon: Some("cod_multiple_windows"),
        },
        SessionSelect(SessionSelectArguments {
            mode: SessionSelectMode::SwapWithActiveKeepFocus,
            ..
        }) => CommandDef {
            brief: "Swap With Another Pane And Keep Focus".into(),
            doc: "Swap the current pane with another pane but keep focus here".into(),
            keys: vec![], // FIXME: find a new assignment
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window"],
            icon: Some("cod_multiple_windows"),
        },
        SessionSelect(SessionSelectArguments {
            mode: SessionSelectMode::MoveToNewSession,
            ..
        }) => CommandDef {
            brief: "Move Pane To New Session".into(),
            doc: "Pick a pane and move it into its own session".into(),
            keys: vec![], // FIXME: find a new assignment
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window"],
            icon: Some("cod_multiple_windows"),
        },
        DecreaseFontSize => CommandDef {
            brief: "Decrease font size".into(),
            doc: "Scales the font size smaller by 10%".into(),
            keys: vec![
                (Modifiers::SUPER, "-".into()),
                (Modifiers::CTRL, "-".into()),
            ],
            args: &[ArgType::ActiveWindow],
            menubar: &["View", "Font Size"],
            icon: Some("md_format_size"),
        },
        IncreaseFontSize => CommandDef {
            brief: "Increase font size".into(),
            doc: "Scales the font size larger by 10%".into(),
            keys: vec![
                (Modifiers::SUPER, "=".into()),
                (Modifiers::CTRL, "=".into()),
            ],
            args: &[ArgType::ActiveWindow],
            menubar: &["View", "Font Size"],
            icon: Some("md_format_size"),
        },
        ResetFontSize => CommandDef {
            brief: "Reset font size".into(),
            doc: "Restores the font size to match your configuration file".into(),
            keys: vec![
                (Modifiers::SUPER, "0".into()),
                (Modifiers::CTRL, "0".into()),
            ],
            args: &[ArgType::ActiveWindow],
            menubar: &["View", "Font Size"],
            icon: Some("md_format_size"),
        },
        ResetFontAndWindowSize => CommandDef {
            brief: "Reset the window and font size".into(),
            doc: "Restores the original window and font size".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["View", "Font Size"],
            icon: Some("md_format_size"),
        },
        SpawnSession => CommandDef {
            brief: "New Session".into(),
            doc: "Create a new session".into(),
            keys: vec![(Modifiers::SUPER, "t".into())],
            args: &[ArgType::ActiveWindow],
            menubar: &["Shell"],
            icon: Some("md_tab_plus"),
        },
        SpawnCommandInNewSession(cmd) => CommandDef {
            brief: label_string(action, format!("Spawn a new Session with {cmd:?}").to_string())
                .into(),
            doc: format!("Spawn a new Session with {cmd:?}").into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: Some("md_tab_plus"),
        },
        ActivateSession(-1) => CommandDef {
            brief: "Activate right-most session".into(),
            doc: "Activates the session on the far right".into(),
            keys: vec![(Modifiers::SUPER, "9".into())],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window", "Select Session"],
            icon: None,
        },
        ActivateSession(n) => {
            let n = *n;
            let ordinal = english_ordinal(n + 1);
            let keys = if n >= 0 && n <= 7 {
                vec![(Modifiers::SUPER, (n + 1).to_string())]
            } else {
                vec![]
            };
            CommandDef {
                brief: format!("Activate {ordinal} Session").into(),
                doc: format!("Activates the {ordinal} session").into(),
                keys,
                args: &[ArgType::ActiveWindow],
                menubar: &["Window", "Select Session"],
                icon: None,
            }
        }
        ActivateTerminalByIndex(n) => {
            let n = *n;
            let ordinal = english_ordinal(n as isize);
            CommandDef {
                brief: format!("Activate {ordinal} Session View").into(),
                doc: format!("Activates the {ordinal} session view").into(),
                keys: vec![],
                args: &[ArgType::ActiveWindow],
                menubar: &[],
                icon: None,
            }
        }
        SetTerminalZoomState(true) => CommandDef {
            brief: "Zoom the current Session View".into(),
            doc: format!(
                "Places the current session view into the zoomed state, \
                             filling all of the space in the session layout"
            )
            .into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: Some("md_fullscreen"),
        },
        SetTerminalZoomState(false) => CommandDef {
            brief: "Exit Session View Zoom".into(),
            doc: "Takes the current session view out of the zoomed state".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: Some("md_fullscreen"),
        },
        EmitEvent(name) => CommandDef {
            brief: format!("Emit event `{name}`").into(),
            doc: format!(
                "Emits the named event, causing any \
                             associated event handler(s) to trigger"
            )
            .into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: None,
        },
        CloseCurrentSession { confirm: true } => CommandDef {
            brief: "Delete current session".into(),
            doc: "Deletes the current session, terminating all the \
            processes that are running in its terminal instances."
                .into(),
            keys: vec![(Modifiers::SUPER, "w".into())],
            args: &[ArgType::ActiveSession],
            menubar: &["Shell"],
            icon: Some("md_close_box_outline"),
        },
        CloseCurrentSession { confirm: false } => CommandDef {
            brief: "Delete current session".into(),
            doc: "Deletes the current session, terminating all the \
            processes that are running in its terminal instances."
                .into(),
            keys: vec![],
            args: &[ArgType::ActiveSession],
            menubar: &[],
            icon: Some("md_close_box_outline"),
        },
        ActivateSessionRelative(-1) => CommandDef {
            brief: "Activate the session to the left".into(),
            doc: "Activates the session to the left. If this is the left-most \
            session then cycles around and activates the right-most session"
                .into(),
            keys: vec![
                (Modifiers::SUPER.union(Modifiers::SHIFT), "[".into()),
                (Modifiers::CTRL.union(Modifiers::SHIFT), "Tab".into()),
                (Modifiers::CTRL, "PageUp".into()),
            ],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window", "Select Session"],
            icon: None,
        },
        ActivateSessionRelative(1) => CommandDef {
            brief: "Activate the session to the right".into(),
            doc: "Activates the session to the right. If this is the right-most \
            session then cycles around and activates the left-most session"
                .into(),
            keys: vec![
                (Modifiers::SUPER.union(Modifiers::SHIFT), "]".into()),
                (Modifiers::CTRL, "Tab".into()),
                (Modifiers::CTRL, "PageDown".into()),
            ],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window", "Select Session"],
            icon: None,
        },
        ActivateSessionRelative(n) => {
            let (direction, amount) = if *n < 0 { ("left", -n) } else { ("right", *n) };
            let ordinal = english_ordinal(amount + 1);
            CommandDef {
                brief: format!("Activate the {ordinal} session to the {direction}").into(),
                doc: format!(
                    "Activates the {ordinal} session to the {direction}. \
                         Wraps around to the other end"
                )
                .into(),
                keys: vec![],
                args: &[ArgType::ActiveWindow],
                menubar: &[],
                icon: None,
            }
        }
        ActivateSessionRelativeNoWrap(-1) => CommandDef {
            brief: "Activate the session to the left (no wrapping)".into(),
            doc: "Activates the session to the left. Stopping at the left-most session".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: None,
        },
        ActivateSessionRelativeNoWrap(1) => CommandDef {
            brief: "Activate the session to the right (no wrapping)".into(),
            doc: "Activates the session to the right. Stopping at the right-most session".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &[],
            icon: None,
        },
        ActivateSessionRelativeNoWrap(n) => {
            let (direction, amount) = if *n < 0 { ("left", -n) } else { ("right", *n) };
            let ordinal = english_ordinal(amount + 1);
            CommandDef {
                brief: format!("Activate the {ordinal} session to the {direction}").into(),
                doc: format!("Activates the {ordinal} session to the {direction}").into(),
                keys: vec![],
                args: &[ArgType::ActiveWindow],
                menubar: &[],
                icon: None,
            }
        }
        ReloadConfiguration => CommandDef {
            brief: "Reload configuration".into(),
            doc: "Reloads the configuration file".into(),
            keys: vec![(Modifiers::SUPER, "r".into())],
            args: &[],
            menubar: &["Chatminal"],
            icon: Some("md_reload"),
        },
        QuitApplication => CommandDef {
            brief: "Quit Chatminal".into(),
            doc: "Quits Chatminal".into(),
            keys: vec![(Modifiers::SUPER, "q".into())],
            args: &[],
            menubar: &["Chatminal"],
            icon: Some("oct_stop"),
        },
        MoveSessionRelative(-1) => CommandDef {
            brief: "Move session one place to the left".into(),
            doc: "Rearranges the sessions so that the current session moves \
            one place to the left"
                .into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "PageUp".into())],
            args: &[ArgType::ActiveSession],
            menubar: &["Window", "Move Session"],
            icon: Some("fa_long_arrow_left"),
        },
        MoveSessionRelative(1) => CommandDef {
            brief: "Move session one place to the right".into(),
            doc: "Rearranges the sessions so that the current session moves \
            one place to the right"
                .into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "PageDown".into())],
            args: &[ArgType::ActiveSession],
            menubar: &["Window", "Move Session"],
            icon: Some("fa_long_arrow_right"),
        },
        MoveSessionRelative(n) => {
            let (direction, amount, icon) = if *n < 0 {
                ("left", (-n).to_string(), "md_chevron_double_left")
            } else {
                ("right", n.to_string(), "md_chevron_double_right")
            };

            CommandDef {
                brief: format!("Move session {amount} place(s) to the {direction}").into(),
                doc: format!(
                    "Rearranges the sessions so that the current session moves \
            {amount} place(s) to the {direction}"
                )
                .into(),
                keys: vec![],
                args: &[ArgType::ActiveSession],
                menubar: &[],
                icon: Some(icon),
            }
        }
        MoveSession(n) => {
            let n = (*n) + 1;
            CommandDef {
                brief: format!("Move session to index {n}").into(),
                doc: format!(
                    "Rearranges the sessions so that the current session \
                             moves to position {n}"
                )
                .into(),
                keys: vec![],
                args: &[ArgType::ActiveSession],
                menubar: &[],
                icon: None,
            }
        }
        ScrollByPage(amount) => {
            let amount = amount.into_inner();
            if amount == -1.0 {
                CommandDef {
                    brief: "Scroll Up One Page".into(),
                    doc: "Scrolls the viewport up by 1 page".into(),
                    keys: vec![(Modifiers::SHIFT, "PageUp".into())],
                    args: &[ArgType::ActiveTerminal],
                    menubar: &["View"],
                    icon: None,
                }
            } else if amount == 1.0 {
                CommandDef {
                    brief: "Scroll Down One Page".into(),
                    doc: "Scrolls the viewport down by 1 page".into(),
                    keys: vec![(Modifiers::SHIFT, "PageDown".into())],
                    args: &[ArgType::ActiveTerminal],
                    menubar: &["View"],
                    icon: None,
                }
            } else if amount < 0.0 {
                let amount = -amount;
                CommandDef {
                    brief: format!("Scroll Up {amount} Page(s)").into(),
                    doc: format!("Scrolls the viewport up by {amount} pages").into(),
                    keys: vec![],
                    args: &[ArgType::ActiveTerminal],
                    menubar: &["View"],
                    icon: None,
                }
            } else {
                CommandDef {
                    brief: format!("Scroll Down {amount} Page(s)").into(),
                    doc: format!("Scrolls the viewport down by {amount} pages").into(),
                    keys: vec![],
                    args: &[ArgType::ActiveTerminal],
                    menubar: &["View"],
                    icon: None,
                }
            }
        }
        ScrollByLine(n) => {
            let (direction, amount) = if *n < 0 {
                ("up", (-n).to_string())
            } else {
                ("down", n.to_string())
            };
            CommandDef {
                brief: format!("Scroll {direction} {amount} line(s)").into(),
                doc: format!(
                    "Scrolls the viewport {direction} by \
                             {amount} line(s)"
                )
                .into(),
                keys: vec![],
                args: &[ArgType::ActiveTerminal],
                menubar: &[],
                icon: None,
            }
        }
        ScrollToPrompt(n) => {
            let (direction, amount) = if *n < 0 { ("up", -n) } else { ("down", *n) };
            let ordinal = english_ordinal(amount);
            CommandDef {
                brief: format!("Scroll {direction} {amount} prompt(s)").into(),
                doc: format!(
                    "Scrolls the viewport {direction} to the \
                             {ordinal} semantic prompt zone in that direction"
                )
                .into(),
                keys: vec![],
                args: &[ArgType::ActiveTerminal],
                menubar: &[],
                icon: Some("oct_terminal"),
            }
        }
        ScrollByCurrentEventWheelDelta => CommandDef {
            brief: "Scrolls based on the mouse wheel position \
                in the current mouse event"
                .into(),
            doc: "Scrolls based on the mouse wheel position \
                in the current mouse event"
                .into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &[],
            icon: None,
        },
        ScrollToBottom => CommandDef {
            brief: "Scroll to the bottom".into(),
            doc: "Scrolls to the bottom of the viewport".into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &["View"],
            icon: Some("md_format_align_bottom"),
        },
        ToggleRealtimeFooter => CommandDef {
            brief: "Toggle realtime footer".into(),
            doc: "Shows or hides the realtime footer at the bottom of the shell".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["View"],
            icon: None,
        },
        ScrollToTop => CommandDef {
            brief: "Scroll to the top".into(),
            doc: "Scrolls to the top of the viewport".into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &["View"],
            icon: Some("md_format_align_top"),
        },
        ActivateCopyMode => CommandDef {
            brief: "Open Copy Mode".into(),
            doc: "Use the keyboard to move through terminal output and copy text".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "x".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit"],
            icon: Some("md_content_copy"),
        },
        SplitVertical(_) => CommandDef {
            brief: label_string(action, "Split Top/Bottom".to_string()).into(),
            doc: "Split the current pane into top and bottom panes".into(),
            keys: vec![(
                Modifiers::CTRL
                    .union(Modifiers::ALT)
                    .union(Modifiers::SHIFT),
                "'".into(),
            )],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Shell"],
            icon: Some("cod_split_vertical"),
        },
        SplitHorizontal(_) => CommandDef {
            brief: label_string(action, "Split Left/Right".to_string()).into(),
            doc: "Split the current pane into left and right panes".into(),
            keys: vec![(
                Modifiers::CTRL
                    .union(Modifiers::ALT)
                    .union(Modifiers::SHIFT),
                "5".into(),
            )],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Shell"],
            icon: Some("cod_split_horizontal"),
        },
        AdjustSplitSize(SessionDirection::Left, amount) => CommandDef {
            brief: format!("Resize Split Left By {amount}").into(),
            doc: "Move the nearest split divider to the left".into(),
            keys: vec![(
                Modifiers::CTRL
                    .union(Modifiers::ALT)
                    .union(Modifiers::SHIFT),
                "LeftArrow".into(),
            )],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Resize Session Layout"],
            icon: None,
        },
        AdjustSplitSize(SessionDirection::Right, amount) => CommandDef {
            brief: format!("Resize Split Right By {amount}").into(),
            doc: "Move the nearest split divider to the right".into(),
            keys: vec![(
                Modifiers::CTRL
                    .union(Modifiers::ALT)
                    .union(Modifiers::SHIFT),
                "RightArrow".into(),
            )],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Resize Session Layout"],
            icon: None,
        },
        AdjustSplitSize(SessionDirection::Up, amount) => CommandDef {
            brief: format!("Resize Split Up By {amount}").into(),
            doc: "Move the nearest split divider upward".into(),
            keys: vec![(
                Modifiers::CTRL
                    .union(Modifiers::ALT)
                    .union(Modifiers::SHIFT),
                "UpArrow".into(),
            )],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Resize Session Layout"],
            icon: None,
        },
        AdjustSplitSize(SessionDirection::Down, amount) => CommandDef {
            brief: format!("Resize Split Down By {amount}").into(),
            doc: "Move the nearest split divider downward".into(),
            keys: vec![(
                Modifiers::CTRL
                    .union(Modifiers::ALT)
                    .union(Modifiers::SHIFT),
                "DownArrow".into(),
            )],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Resize Session Layout"],
            icon: None,
        },
        AdjustSplitSize(SessionDirection::Next | SessionDirection::Prev, _) => return None,
        ActivateSessionDirection(SessionDirection::Next | SessionDirection::Prev) => return None,
        ActivateSessionDirection(SessionDirection::Left) => CommandDef {
            brief: "Focus Pane Left".into(),
            doc: "Move focus to the pane on the left".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "LeftArrow".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Select Session View"],
            icon: Some("fa_long_arrow_left"),
        },
        ActivateSessionDirection(SessionDirection::Right) => CommandDef {
            brief: "Focus Pane Right".into(),
            doc: "Move focus to the pane on the right".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "RightArrow".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Select Session View"],
            icon: Some("fa_long_arrow_right"),
        },
        ActivateSessionDirection(SessionDirection::Up) => CommandDef {
            brief: "Focus Pane Up".into(),
            doc: "Move focus to the pane above".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "UpArrow".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Select Session View"],
            icon: Some("fa_long_arrow_up"),
        },
        ActivateSessionDirection(SessionDirection::Down) => CommandDef {
            brief: "Focus Pane Down".into(),
            doc: "Move focus to the pane below".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "DownArrow".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Select Session View"],
            icon: Some("fa_long_arrow_down"),
        },
        ToggleTerminalZoomState => CommandDef {
            brief: "Toggle Pane Zoom".into(),
            doc: "Expand the current pane, or restore the split layout".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "z".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window"],
            icon: Some("md_fullscreen"),
        },
        ActivateLastSession => CommandDef {
            brief: "Switch To Last Session".into(),
            doc: "Switch back to the session you used most recently".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window", "Select Session"],
            icon: None,
        },
        ClearKeyTableStack => CommandDef {
            brief: "Clear the key table stack".into(),
            doc: "Removes all entries from the stack".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["Edit"],
            icon: None,
        },
        OpenLinkAtMouseCursor => return None,
        ShowLauncherArgs(_) | ShowLauncher => return None,
        ShowSessionNavigator => CommandDef {
            brief: "Open Session Navigator".into(),
            doc: "Browse and switch sessions from a searchable list".into(),
            keys: vec![],
            args: &[ArgType::ActiveWindow],
            menubar: &["Window", "Select Session"],
            icon: Some("cod_list_flat"),
        },
        OpenUri(uri) => match uri.as_ref() {
            "https://github.com/Khoa280703/chatminal" => CommandDef {
                brief: "Documentation".into(),
                doc: "Open the documentation website in your browser".into(),
                keys: vec![],
                args: &[],
                menubar: &["Help"],
                icon: Some("md_help"),
            },
            "https://github.com/Khoa280703/chatminal/discussions/" => CommandDef {
                brief: "Discuss on GitHub".into(),
                doc: "Open the GitHub discussions page in your browser".into(),
                keys: vec![],
                args: &[],
                menubar: &["Help"],
                icon: Some("oct_comment_discussion"),
            },
            "https://github.com/Khoa280703/chatminalissues/" => CommandDef {
                brief: "Search or report issue on GitHub".into(),
                doc: "Open the GitHub issues page in your browser".into(),
                keys: vec![],
                args: &[],
                menubar: &["Help"],
                icon: Some("fa_ticket"),
            },
            _ => CommandDef {
                brief: format!("Open {uri} in your browser").into(),
                doc: format!("Open {uri} in your browser").into(),
                keys: vec![],
                args: &[],
                menubar: &[],
                icon: Some("oct_browser"),
            },
        },
        SendString(text) => CommandDef {
            brief: format!(
                "Sends `{text}` to the active session view, \
                           as though you typed it"
            )
            .into(),
            doc: format!(
                "Sends `{text}` to the active session view, as \
                         though you typed it"
            )
            .into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: Some("md_keyboard_variant"),
        },
        SendKey(key) => CommandDef {
            brief: format!(
                "Sends {key:?} to the active session view, \
                           as though you typed it"
            )
            .into(),
            doc: format!(
                "Sends {key:?} to the active session view, \
                         as though you typed it"
            )
            .into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: Some("md_keyboard_variant"),
        },
        Nop => CommandDef {
            brief: "Does nothing".into(),
            doc: "Has no effect".into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: None,
        },
        DisableDefaultAssignment => return None,
        SelectTextAtMouseCursor(mode) => CommandDef {
            brief: format!(
                "Selects text at the mouse cursor \
                           location using {mode:?}"
            )
            .into(),
            doc: format!(
                "Selects text at the mouse cursor \
                         location using {mode:?}"
            )
            .into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: None,
        },
        ExtendSelectionToMouseCursor(mode) => CommandDef {
            brief: format!(
                "Extends the selection text to the mouse \
                           cursor location using {mode:?}"
            )
            .into(),
            doc: format!(
                "Extends the selection text to the mouse \
                         cursor location using {mode:?}"
            )
            .into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: None,
        },
        ClearSelection => CommandDef {
            brief: "Clears the selection in the current session view".into(),
            doc: "Clears the selection in the current session view".into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: None,
        },
        CompleteSelection(destination) => CommandDef {
            brief: format!("Completes selection, and copy {destination:?}").into(),
            doc: format!(
                "Completes text selection using the mouse, and copies \
                to {destination:?}"
            )
            .into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: None,
        },
        CompleteSelectionOrOpenLinkAtMouseCursor(destination) => CommandDef {
            brief: format!("Completes selection, and copy {destination:?}").into(),
            doc: format!(
                "Completes text selection using the mouse, and copies \
                to {destination:?}"
            )
            .into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: None,
        },
        StartWindowDrag => CommandDef {
            brief: "Requests a window drag operation from \
                the window environment"
                .into(),
            doc: "Requests a window drag operation from \
                the window environment"
                .into(),
            keys: vec![],
            args: &[],
            menubar: &[],
            icon: Some("md_drag"),
        },
        Multiple(actions) => {
            let mut brief = String::new();
            for act in actions {
                if !brief.is_empty() {
                    brief.push_str(", ");
                }
                match derive_command_from_key_assignment(act) {
                    Some(cmd) => {
                        brief.push_str(&cmd.brief);
                    }
                    None => {
                        brief.push_str(&format!("{act:?}"));
                    }
                }
            }
            CommandDef {
                brief: brief.into(),
                doc: "Performs multiple nested actions".into(),
                keys: vec![],
                args: &[ArgType::ActiveTerminal],
                menubar: &[],
                icon: None,
            }
        }
        SwitchToWorkspace { .. } | SwitchWorkspaceRelative(_) => return None,
        ActivateKeyTable { name, .. } => CommandDef {
            brief: format!("Activate key table `{name}`").into(),
            doc: format!("Activate key table `{name}`").into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &[],
            icon: None,
        },
        PopKeyTable => CommandDef {
            brief: "Pop the current key table".into(),
            doc: "Pop the current key table".into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &[],
            icon: None,
        },
        CopyMode(copy_mode) => CommandDef {
            brief: format!("{copy_mode:?}").into(),
            doc: "".into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Edit", "Copy Mode"],
            icon: None,
        },
        RotatePanes(direction) => CommandDef {
            brief: format!("Rotate Split Layout {direction:?}").into(),
            doc: format!("Rotate the current split layout {direction:?}").into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Window", "Rotate Session Layout"],
            icon: Some(match direction {
                RotationDirection::Clockwise => "md_rotate_right",
                RotationDirection::CounterClockwise => "md_rotate_left",
            }),
        },
        SplitSession(split) => {
            let direction = split.direction;
            CommandDef {
                brief: label_string(action, format!("Split the current session view {direction:?}")).into(),
                doc: format!("Split the current session view {direction:?}").into(),
                keys: vec![],
                args: &[ArgType::ActiveTerminal],
                menubar: &[],
                icon: match split.direction {
                    SessionDirection::Up | SessionDirection::Down => Some("cod_split_vertical"),
                    SessionDirection::Left | SessionDirection::Right => Some("cod_split_horizontal"),
                    SessionDirection::Next | SessionDirection::Prev => None,
                },
            }
        }
        ResetTerminal => CommandDef {
            brief: "Reset terminal display state".into(),
            doc: "Reset terminal display state".into(),
            keys: vec![],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Shell"],
            icon: None,
        },
        ActivateCommandPalette => CommandDef {
            brief: "Open Action Finder".into(),
            doc: "Shows the Action Finder modal".into(),
            keys: vec![(Modifiers::CTRL.union(Modifiers::SHIFT), "p".into())],
            args: &[ArgType::ActiveTerminal],
            menubar: &["Chatminal"],
            icon: None,
        },
    })
}

/// Returns a list of key assignment actions that should be
/// included in the default key assignments and command palette.
fn compute_default_actions() -> Vec<KeyAssignment> {
    // These are ordered by their position within the various menus
    return vec![
        // ----------------- Chatminal
        ReloadConfiguration,
        #[cfg(target_os = "macos")]
        HideApplication,
        #[cfg(target_os = "macos")]
        QuitApplication,
        // ----------------- Shell
        SpawnSession,
        SplitVertical(SpawnCommand::default()),
        SplitHorizontal(SpawnCommand::default()),
        CloseCurrentSession { confirm: true },
        CloseCurrentSession { confirm: true },
        ResetTerminal,
        // ----------------- Edit
        #[cfg(not(target_os = "macos"))]
        PasteFrom(ClipboardPasteSource::PrimarySelection),
        #[cfg(not(target_os = "macos"))]
        CopyTo(ClipboardCopyDestination::PrimarySelection),
        CopyTo(ClipboardCopyDestination::Clipboard),
        PasteFrom(ClipboardPasteSource::Clipboard),
        QuickSelect,
        ActivateCopyMode,
        ClearKeyTableStack,
        ActivateCommandPalette,
        // ----------------- View
        DecreaseFontSize,
        IncreaseFontSize,
        ResetFontSize,
        ResetFontAndWindowSize,
        ScrollByPage(NotNan::new(-1.0).unwrap()),
        ScrollByPage(NotNan::new(1.0).unwrap()),
        ScrollToTop,
        ScrollToBottom,
        ToggleRealtimeFooter,
        // ----------------- Window
        ToggleFullScreen,
        ToggleAlwaysOnTop,
        ToggleAlwaysOnBottom,
        SetWindowLevel(WindowLevel::AlwaysOnBottom),
        SetWindowLevel(WindowLevel::Normal),
        SetWindowLevel(WindowLevel::AlwaysOnTop),
        Hide,
        Search(Pattern::CurrentSelectionOrEmptyString),
        SessionSelect(SessionSelectArguments {
            alphabet: String::new(),
            mode: SessionSelectMode::SwapWithActive,
            show_session_ids: false,
        }),
        SessionSelect(SessionSelectArguments {
            alphabet: String::new(),
            mode: SessionSelectMode::SwapWithActiveKeepFocus,
            show_session_ids: false,
        }),
        SessionSelect(SessionSelectArguments {
            alphabet: String::new(),
            mode: SessionSelectMode::MoveToNewSession,
            show_session_ids: false,
        }),
        RotatePanes(RotationDirection::Clockwise),
        RotatePanes(RotationDirection::CounterClockwise),
        ActivateSession(0),
        ActivateSession(1),
        ActivateSession(2),
        ActivateSession(3),
        ActivateSession(4),
        ActivateSession(5),
        ActivateSession(6),
        ActivateSession(7),
        ActivateSession(-1),
        ActivateSessionRelative(-1),
        ActivateSessionRelative(1),
        MoveSessionRelative(-1),
        MoveSessionRelative(1),
        AdjustSplitSize(SessionDirection::Left, 1),
        AdjustSplitSize(SessionDirection::Right, 1),
        AdjustSplitSize(SessionDirection::Up, 1),
        AdjustSplitSize(SessionDirection::Down, 1),
        ActivateSessionDirection(SessionDirection::Left),
        ActivateSessionDirection(SessionDirection::Right),
        ActivateSessionDirection(SessionDirection::Up),
        ActivateSessionDirection(SessionDirection::Down),
        ToggleTerminalZoomState,
        ActivateLastSession,
        ShowSessionNavigator,
        // ----------------- Help
        ShowDebugOverlay,
        // ----------------- Misc
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ui_filter_rejects_layout_only_actions() {
        assert!(!is_supported_in_session_ui(
            &KeyAssignment::AdjustSplitSize(SessionDirection::Left, 1,)
        ));
        assert!(!is_supported_in_session_ui(
            &KeyAssignment::ActivateSessionDirection(SessionDirection::Left,)
        ));
        assert!(!is_supported_in_session_ui(&KeyAssignment::RotatePanes(
            RotationDirection::Clockwise,
        )));
        assert!(!is_supported_in_session_ui(
            &KeyAssignment::ToggleTerminalZoomState
        ));
        assert!(!is_supported_in_session_ui(
            &KeyAssignment::SetTerminalZoomState(true)
        ));
    }

    #[test]
    fn session_ui_filter_keeps_supported_actions() {
        assert!(is_supported_in_session_ui(&KeyAssignment::SpawnSession));
        assert!(is_supported_in_session_ui(&KeyAssignment::ActivateSession(
            0
        )));
    }

    #[test]
    fn retain_supported_for_session_ui_removes_unsupported_entries() {
        let mut commands = vec![
            ExpandedCommand {
                brief: "Keep".into(),
                doc: "".into(),
                action: KeyAssignment::ActivateSession(0),
                keys: vec![],
                menubar: &["Window"],
                icon: None,
            },
            ExpandedCommand {
                brief: "Drop".into(),
                doc: "".into(),
                action: KeyAssignment::AdjustSplitSize(SessionDirection::Left, 1),
                keys: vec![],
                menubar: &["Window"],
                icon: None,
            },
            ExpandedCommand {
                brief: "Drop zoom".into(),
                doc: "".into(),
                action: KeyAssignment::ToggleTerminalZoomState,
                keys: vec![],
                menubar: &["Window"],
                icon: None,
            },
        ];

        retain_supported_for_session_ui(&mut commands, true);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].brief, "Keep");
    }
}
