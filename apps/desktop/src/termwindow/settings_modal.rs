use crate::commands::{CommandDef, ExpandedCommand};
use crate::inputmap::ui_key;
use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::palette::{
    sidebar_like_border, sidebar_like_muted_text, sidebar_like_panel_bg, sidebar_like_text,
};
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::{TermWindow, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::keyassignment::KeyAssignment;
use config::ConfigHandle;
use config::Dimension;
use std::cell::{Ref, RefCell};
use std::cmp::Ordering;
use std::rc::Rc;
use std::time::Instant;
use terminal_emulator::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use window::color::LinearRgba;
use window::{ModifierToStringArgs, Modifiers};

const TAB_ORDER: &[&str] = &["Chatminal", "Shell", "Edit", "View", "Window"];

#[derive(Clone)]
struct SettingsTab {
    title: String,
    entries: Vec<SettingsEntry>,
}

#[derive(Clone)]
struct SettingsEntry {
    title: String,
    section: String,
    shortcut: String,
    action: KeyAssignment,
}

#[derive(Clone)]
pub(crate) struct SettingsCatalog {
    tabs: Vec<SettingsTab>,
}

impl SettingsCatalog {
    pub(crate) fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.tabs.iter().map(|tab| tab.entries.len()).sum()
    }
}

pub struct SettingsModal {
    element: RefCell<Option<Vec<ComputedElement>>>,
    catalog: Rc<SettingsCatalog>,
    selected_tab: RefCell<usize>,
    selected_row: RefCell<usize>,
    top_row: RefCell<usize>,
    max_rows_on_screen: RefCell<usize>,
}

impl SettingsModal {
    pub fn new(catalog: Rc<SettingsCatalog>) -> Self {
        Self {
            element: RefCell::new(None),
            catalog,
            selected_tab: RefCell::new(0),
            selected_row: RefCell::new(0),
            top_row: RefCell::new(0),
            max_rows_on_screen: RefCell::new(8),
        }
    }

    fn current_entries(&self) -> &[SettingsEntry] {
        let tab_idx = (*self.selected_tab.borrow()).min(self.catalog.tabs.len().saturating_sub(1));
        self.catalog
            .tabs
            .get(tab_idx)
            .map(|tab| tab.entries.as_slice())
            .unwrap_or(&[])
    }

    fn clamp_selection(&self) {
        let total = self.current_entries().len();
        let page = (*self.max_rows_on_screen.borrow()).max(1);
        if total == 0 {
            *self.selected_row.borrow_mut() = 0;
            *self.top_row.borrow_mut() = 0;
            return;
        }

        let max_top = total.saturating_sub(page);
        {
            let mut top = self.top_row.borrow_mut();
            *top = (*top).min(max_top);
        }

        let top = *self.top_row.borrow();
        let last_visible = (top + page - 1).min(total - 1);
        let mut row = self.selected_row.borrow_mut();
        *row = (*row).min(total - 1).clamp(top, last_visible);
    }

    fn invalidate_cache(&self) {
        self.element.borrow_mut().take();
    }

    fn set_selected_tab(&self, index: usize) {
        let last = self.catalog.tabs.len().saturating_sub(1);
        *self.selected_tab.borrow_mut() = index.min(last);
        *self.selected_row.borrow_mut() = 0;
        *self.top_row.borrow_mut() = 0;
        self.clamp_selection();
        self.invalidate_cache();
    }

    fn move_tab(&self, delta: isize) {
        let tab = (*self.selected_tab.borrow()).saturating_add_signed(delta);
        self.set_selected_tab(tab);
    }

    fn move_row(&self, delta: isize) {
        let total = self.current_entries().len();
        if total == 0 {
            return;
        }

        let page = (*self.max_rows_on_screen.borrow()).max(1);
        let mut row = self.selected_row.borrow_mut();
        *row = row.saturating_add_signed(delta).min(total - 1);

        let mut top = self.top_row.borrow_mut();
        if *row < *top {
            *top = *row;
        } else if *row >= *top + page {
            *top = row.saturating_sub(page.saturating_sub(1));
        }
        drop(top);
        drop(row);
        self.invalidate_cache();
    }

    fn scroll_by_lines(&self, delta: isize) {
        let total = self.current_entries().len();
        let page = (*self.max_rows_on_screen.borrow()).max(1);
        let max_top = total.saturating_sub(page);
        let mut top = self.top_row.borrow_mut();
        *top = top.saturating_add_signed(delta).clamp(0, max_top);
        drop(top);
        self.clamp_selection();
        self.invalidate_cache();
    }

    fn resolve_hit(term_window: &TermWindow) -> Option<UIItemType> {
        let event = term_window.current_mouse_event.as_ref()?;
        term_window
            .ui_items
            .iter()
            .rev()
            .find(|item| item.hit_test(event.coords.x, event.coords.y))
            .map(|item| item.item_type.clone())
    }

    fn run_selected_action(&self, term_window: &mut TermWindow, row: usize) {
        let Some(entry) = self.current_entries().get(row) else {
            return;
        };
        let Some(pane) = term_window.active_terminal_instance_or_overlay() else {
            term_window.cancel_modal();
            return;
        };
        let action = entry.action.clone();
        term_window.cancel_modal();
        if let Err(err) = term_window.perform_key_assignment(&pane, &action) {
            log::error!("Error while performing settings action {action:?}: {err:#}");
        }
    }

    fn text_block(
        font: &std::rc::Rc<terminal_font::LoadedFont>,
        content: impl Into<String>,
        color: LinearRgba,
    ) -> Element {
        Element::new(font, ElementContent::Text(content.into()))
            .display(DisplayType::Block)
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: color.into(),
            })
    }

    fn chip(
        font: &std::rc::Rc<terminal_font::LoadedFont>,
        label: &str,
        item_type: UIItemType,
        active: bool,
        text: LinearRgba,
        muted: LinearRgba,
    ) -> Element {
        let active_bg = accent_active_bg();
        let hover_bg = accent_hover_bg();
        Element::new(font, ElementContent::Text(label.to_string()))
            .item_type(item_type)
            .display(DisplayType::Inline)
            .padding(BoxDimension {
                left: Dimension::Pixels(12.0),
                right: Dimension::Pixels(12.0),
                top: Dimension::Pixels(6.0),
                bottom: Dimension::Pixels(6.0),
            })
            .margin(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(8.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(8.0),
            })
            .border_corners(Some(rounded_corners(5.0)))
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: if active {
                    active_bg
                } else {
                    LinearRgba::TRANSPARENT
                }
                .into(),
                text: if active { text } else { muted }.into(),
            })
            .hover_colors(Some(ElementColors {
                border: BorderColor::default(),
                bg: hover_bg.into(),
                text: text.into(),
            }))
    }

    fn compute(&self, term_window: &mut TermWindow) -> anyhow::Result<Vec<ComputedElement>> {
        let compute_start = Instant::now();
        let font_start = Instant::now();
        let font = term_window.fonts.command_palette_font()?;
        let font_elapsed = font_start.elapsed();
        if crate::termwindow::settings_modal_perf_enabled() {
            log::info!("settings-modal.font elapsed={:?}", font_elapsed);
        }
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let dimensions = term_window.dimensions;
        let width_px = (dimensions.pixel_width as f32 * 0.62).clamp(640.0, 920.0);
        let height_px = (dimensions.pixel_height as f32 * 0.72).clamp(420.0, 760.0);
        let panel_x = ((dimensions.pixel_width as f32 - width_px) / 2.0).max(24.0);
        let panel_y = ((dimensions.pixel_height as f32 - height_px) / 2.0).max(28.0);
        let row_height = (metrics.cell_size.height as f32 * 2.1).max(34.0);
        let rows_per_page = ((height_px - 210.0) / row_height).floor().max(4.0) as usize;
        *self.max_rows_on_screen.borrow_mut() = rows_per_page;
        self.clamp_selection();

        let text = sidebar_like_text();
        let muted = sidebar_like_muted_text();
        let sub_muted = LinearRgba::with_components(0.47, 0.47, 0.47, 1.0);
        let panel_bg = sidebar_like_panel_bg();
        let border = sidebar_like_border();
        let list_border = sidebar_like_border();
        let row_hover = accent_hover_bg();
        let row_selected = accent_active_bg();

        let tabs = self
            .catalog
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| {
                Self::chip(
                    &font,
                    &tab.title,
                    UIItemType::ChatminalSettingsModalTab(idx),
                    idx == *self.selected_tab.borrow(),
                    text,
                    muted,
                )
            })
            .collect::<Vec<_>>();

        let top_row = *self.top_row.borrow();
        let selected_row = *self.selected_row.borrow();
        let entries = self.current_entries();
        let visible_end = (top_row + rows_per_page).min(entries.len());
        let action_rows = if entries.is_empty() {
            vec![Self::text_block(&font, "No actions", muted)]
        } else {
            entries[top_row..visible_end]
                .iter()
                .enumerate()
                .map(|(offset, entry)| {
                    let row_idx = top_row + offset;
                    let is_selected = row_idx == selected_row;
                    let row_text = Element::new(
                        &font,
                        ElementContent::Children(vec![
                            Element::new(&font, ElementContent::Text(entry.title.clone()))
                                .display(DisplayType::Inline)
                                .colors(text_colors(text)),
                            Element::new(&font, ElementContent::Text(entry.shortcut.clone()))
                                .display(DisplayType::Inline)
                                .float(Float::Right)
                                .colors(text_colors(muted)),
                        ]),
                    )
                    .display(DisplayType::Block);

                    let mut children = vec![row_text];
                    if !entry.section.is_empty() {
                        children.push(
                            Self::text_block(&font, entry.section.clone(), sub_muted).padding(
                                BoxDimension {
                                    left: Dimension::Pixels(0.0),
                                    right: Dimension::Pixels(0.0),
                                    top: Dimension::Pixels(4.0),
                                    bottom: Dimension::Pixels(0.0),
                                },
                            ),
                        );
                    }

                    Element::new(&font, ElementContent::Children(children))
                        .item_type(UIItemType::ChatminalSettingsModalAction(row_idx))
                        .display(DisplayType::Block)
                        .padding(BoxDimension {
                            left: Dimension::Pixels(12.0),
                            right: Dimension::Pixels(12.0),
                            top: Dimension::Pixels(9.0),
                            bottom: Dimension::Pixels(9.0),
                        })
                        .margin(BoxDimension {
                            left: Dimension::Pixels(0.0),
                            right: Dimension::Pixels(0.0),
                            top: Dimension::Pixels(0.0),
                            bottom: Dimension::Pixels(4.0),
                        })
                        .border_corners(Some(rounded_corners(4.0)))
                        .colors(ElementColors {
                            border: BorderColor::default(),
                            bg: if is_selected {
                                row_selected
                            } else {
                                LinearRgba::TRANSPARENT
                            }
                            .into(),
                            text: text.into(),
                        })
                        .hover_colors(Some(ElementColors {
                            border: BorderColor::default(),
                            bg: row_hover.into(),
                            text: text.into(),
                        }))
                })
                .collect()
        };

        let summary = if entries.is_empty() {
            "0 actions".to_string()
        } else {
            format!("{}-{} / {}", top_row + 1, visible_end, entries.len())
        };

        let backdrop = Element::new(&font, ElementContent::Children(vec![]))
            .item_type(UIItemType::ChatminalSettingsModalBackdrop)
            .display(DisplayType::Block)
            .min_width(Some(Dimension::Pixels(dimensions.pixel_width as f32)))
            .min_height(Some(Dimension::Pixels(dimensions.pixel_height as f32)))
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::with_components(0.0, 0.0, 0.0, 0.58).into(),
                text: LinearRgba::TRANSPARENT.into(),
            });

        let panel = Element::new(
            &font,
            ElementContent::Children(vec![
                Self::text_block(&font, "Settings", text).padding(BoxDimension {
                    left: Dimension::Pixels(0.0),
                    right: Dimension::Pixels(0.0),
                    top: Dimension::Pixels(0.0),
                    bottom: Dimension::Pixels(4.0),
                }),
                Self::text_block(&font, "Actions and shortcuts", muted).padding(BoxDimension {
                    left: Dimension::Pixels(0.0),
                    right: Dimension::Pixels(0.0),
                    top: Dimension::Pixels(0.0),
                    bottom: Dimension::Pixels(14.0),
                }),
                Element::new(&font, ElementContent::Children(tabs))
                    .item_type(UIItemType::ChatminalSettingsModalPanel)
                    .display(DisplayType::Block)
                    .padding(BoxDimension {
                        left: Dimension::Pixels(0.0),
                        right: Dimension::Pixels(0.0),
                        top: Dimension::Pixels(0.0),
                        bottom: Dimension::Pixels(8.0),
                    }),
                Element::new(&font, ElementContent::Children(action_rows))
                    .item_type(UIItemType::ChatminalSettingsModalPanel)
                    .display(DisplayType::Block)
                    .padding(BoxDimension {
                        left: Dimension::Pixels(12.0),
                        right: Dimension::Pixels(12.0),
                        top: Dimension::Pixels(12.0),
                        bottom: Dimension::Pixels(4.0),
                    })
                    .min_height(Some(Dimension::Pixels(
                        (row_height * rows_per_page as f32).max(180.0),
                    )))
                    .border(BoxDimension::new(Dimension::Pixels(1.0)))
                    .border_corners(Some(rounded_corners(5.0)))
                    .colors(ElementColors {
                        border: BorderColor::new(list_border),
                        bg: panel_bg.into(),
                        text: text.into(),
                    }),
                Self::text_block(
                    &font,
                    format!("Esc close  Enter run  Left/Right tab  Up/Down move  {summary}"),
                    muted,
                )
                .padding(BoxDimension {
                    left: Dimension::Pixels(0.0),
                    right: Dimension::Pixels(0.0),
                    top: Dimension::Pixels(14.0),
                    bottom: Dimension::Pixels(0.0),
                }),
            ]),
        )
        .item_type(UIItemType::ChatminalSettingsModalPanel)
        .display(DisplayType::Block)
        .padding(BoxDimension {
            left: Dimension::Pixels(18.0),
            right: Dimension::Pixels(18.0),
            top: Dimension::Pixels(18.0),
            bottom: Dimension::Pixels(18.0),
        })
        .border(BoxDimension::new(Dimension::Pixels(1.0)))
        .border_corners(Some(rounded_corners(5.0)))
        .colors(ElementColors {
            border: BorderColor::new(border),
            bg: panel_bg.into(),
            text: text.into(),
        })
        .min_width(Some(Dimension::Pixels(width_px)))
        .max_width(Some(Dimension::Pixels(width_px)));

        let gl_state = term_window.render_state.as_ref().unwrap();
        let layout = |bounds| LayoutContext {
            height: config::DimensionContext {
                dpi: dimensions.dpi as f32,
                pixel_max: dimensions.pixel_height as f32,
                pixel_cell: metrics.cell_size.height as f32,
            },
            width: config::DimensionContext {
                dpi: dimensions.dpi as f32,
                pixel_max: dimensions.pixel_width as f32,
                pixel_cell: metrics.cell_size.width as f32,
            },
            bounds,
            metrics: &metrics,
            gl_state,
            custom_block_glyphs: term_window.config.custom_block_glyphs,
            zindex: 121,
        };

        let computed = vec![
            term_window.compute_element(
                &layout(euclid::rect(
                    0.0,
                    0.0,
                    dimensions.pixel_width as f32,
                    dimensions.pixel_height as f32,
                )),
                &backdrop,
            )?,
            term_window.compute_element(
                &layout(euclid::rect(panel_x, panel_y, width_px, height_px)),
                &panel,
            )?,
        ];

        if crate::termwindow::settings_modal_perf_enabled() {
            log::info!(
                "settings-modal.compute elapsed={:?} font_elapsed={:?} tabs={} catalog_entries={} tab_entries={} visible_rows={} panel={}x{}",
                compute_start.elapsed(),
                font_elapsed,
                self.catalog.tab_count(),
                self.catalog.entry_count(),
                entries.len(),
                visible_end.saturating_sub(top_row),
                width_px.round() as usize,
                height_px.round() as usize,
            );
        }

        Ok(computed)
    }
}

impl Modal for SettingsModal {
    fn mouse_event(&self, event: MouseEvent, term_window: &mut TermWindow) -> anyhow::Result<()> {
        if event.kind == MouseEventKind::Press {
            match event.button {
                MouseButton::WheelUp(amount) => {
                    self.scroll_by_lines(-(amount as isize));
                    term_window.invalidate_modal();
                }
                MouseButton::WheelDown(amount) => {
                    self.scroll_by_lines(amount as isize);
                    term_window.invalidate_modal();
                }
                _ => {}
            }
        }

        if event.kind != MouseEventKind::Release || event.button != MouseButton::Left {
            return Ok(());
        }

        match Self::resolve_hit(term_window) {
            Some(UIItemType::ChatminalSettingsModalBackdrop) => term_window.cancel_modal(),
            Some(UIItemType::ChatminalSettingsModalTab(idx)) => {
                self.set_selected_tab(idx);
                term_window.invalidate_modal();
            }
            Some(UIItemType::ChatminalSettingsModalAction(idx)) => {
                self.run_selected_action(term_window, idx);
            }
            _ => {}
        }
        Ok(())
    }

    fn key_down(
        &self,
        key: KeyCode,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool> {
        match (key, mods) {
            (KeyCode::Escape, KeyModifiers::NONE) | (KeyCode::Char('g'), KeyModifiers::CTRL) => {
                term_window.cancel_modal();
            }
            (KeyCode::LeftArrow, KeyModifiers::NONE) => self.move_tab(-1),
            (KeyCode::RightArrow, KeyModifiers::NONE) => self.move_tab(1),
            (KeyCode::UpArrow, KeyModifiers::NONE) => self.move_row(-1),
            (KeyCode::DownArrow, KeyModifiers::NONE) => self.move_row(1),
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                self.scroll_by_lines(-(*self.max_rows_on_screen.borrow() as isize))
            }
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                self.scroll_by_lines(*self.max_rows_on_screen.borrow() as isize)
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                self.run_selected_action(term_window, *self.selected_row.borrow());
                return Ok(true);
            }
            _ => return Ok(false),
        }
        term_window.invalidate_modal();
        Ok(true)
    }

    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<Ref<'_, [ComputedElement]>> {
        if self.element.borrow().is_none() {
            self.element
                .borrow_mut()
                .replace(self.compute(term_window)?);
        }
        Ok(Ref::map(self.element.borrow(), |value| {
            value.as_ref().unwrap().as_slice()
        }))
    }

    fn reconfigure(&self, _term_window: &mut TermWindow) {
        self.element.borrow_mut().take();
    }
}

fn rounded_corners(radius: f32) -> Corners {
    Corners {
        top_left: SizedPoly {
            width: Dimension::Pixels(radius),
            height: Dimension::Pixels(radius),
            poly: TOP_LEFT_ROUNDED_CORNER,
        },
        top_right: SizedPoly {
            width: Dimension::Pixels(radius),
            height: Dimension::Pixels(radius),
            poly: TOP_RIGHT_ROUNDED_CORNER,
        },
        bottom_left: SizedPoly {
            width: Dimension::Pixels(radius),
            height: Dimension::Pixels(radius),
            poly: BOTTOM_LEFT_ROUNDED_CORNER,
        },
        bottom_right: SizedPoly {
            width: Dimension::Pixels(radius),
            height: Dimension::Pixels(radius),
            poly: BOTTOM_RIGHT_ROUNDED_CORNER,
        },
    }
}

fn accent_active_bg() -> LinearRgba {
    LinearRgba::with_components(0.016, 0.224, 0.369, 1.0)
}

fn accent_hover_bg() -> LinearRgba {
    LinearRgba::with_components(0.071, 0.102, 0.141, 1.0)
}

fn text_colors(text: LinearRgba) -> ElementColors {
    ElementColors {
        border: BorderColor::default(),
        bg: LinearRgba::TRANSPARENT.into(),
        text: text.into(),
    }
}

pub(crate) fn build_catalog(config: &ConfigHandle) -> SettingsCatalog {
    let start = Instant::now();
    let mut commands = CommandDef::expanded_commands(config);
    commands.retain(|cmd| !cmd.menubar.is_empty());
    let command_count = commands.len();

    let mut grouped: Vec<SettingsTab> = TAB_ORDER
        .iter()
        .map(|title| SettingsTab {
            title: (*title).to_string(),
            entries: vec![],
        })
        .collect();

    for cmd in commands {
        let title = cmd.menubar[0];
        let entry = settings_entry(config, &cmd);
        if let Some(tab) = grouped.iter_mut().find(|tab| tab.title == title) {
            tab.entries.push(entry);
        } else {
            grouped.push(SettingsTab {
                title: title.to_string(),
                entries: vec![entry],
            });
        }
    }

    grouped.retain(|tab| !tab.entries.is_empty());
    let catalog = SettingsCatalog { tabs: grouped };

    if crate::termwindow::settings_modal_perf_enabled() {
        log::info!(
            "settings-modal.build-catalog elapsed={:?} commands={} tabs={} entries={}",
            start.elapsed(),
            command_count,
            catalog.tab_count(),
            catalog.entry_count(),
        );
    }

    catalog
}

fn settings_entry(config: &ConfigHandle, cmd: &ExpandedCommand) -> SettingsEntry {
    SettingsEntry {
        title: cmd.brief.to_string(),
        section: cmd
            .menubar
            .iter()
            .skip(1)
            .copied()
            .collect::<Vec<_>>()
            .join(" / "),
        shortcut: shortcut_label(config, cmd),
        action: cmd.action.clone(),
    }
}

fn shortcut_label(config: &ConfigHandle, cmd: &ExpandedCommand) -> String {
    let mut keys = cmd.keys.clone();
    keys.sort_by(|(a_mods, a_key), (b_mods, b_key)| {
        fn score_mods(mods: &Modifiers) -> usize {
            let mut score: usize = mods.bits() as usize;
            if cfg!(target_os = "macos") && mods.contains(Modifiers::SUPER) {
                score += 1000;
            } else if !cfg!(target_os = "macos") && !mods.contains(Modifiers::SUPER) {
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

        a_key.cmp(b_key)
    });

    let separator = if config.ui_key_cap_rendering == window::UIKeyCapRendering::AppleSymbols {
        " "
    } else {
        "-"
    };

    let mut labels = keys
        .into_iter()
        .map(|(mods, keycode)| {
            let mut mod_string = mods.to_string_with_separator(ModifierToStringArgs {
                separator,
                want_none: false,
                ui_key_cap_rendering: Some(config.ui_key_cap_rendering),
            });
            if !mod_string.is_empty() {
                mod_string.push_str(separator);
            }
            let keycode = ui_key(&keycode, config.ui_key_cap_rendering);
            format!("{mod_string}{keycode}")
        })
        .collect::<Vec<_>>();

    labels.dedup();
    labels.truncate(config.palette_max_key_assigments_for_action);
    labels.join(", ")
}
