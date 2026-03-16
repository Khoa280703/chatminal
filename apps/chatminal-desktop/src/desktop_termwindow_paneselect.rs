use crate::scripting::guiwin::DesktopWindowId;
use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::DimensionContext;
use crate::utilsprites::RenderMetrics;
use crate::TermWindow;
use ::window::WindowOps;
use config::keyassignment::{KeyAssignment, SessionSelectArguments, SessionSelectMode};
use config::Dimension;
use engine_term::{KeyCode, KeyModifiers, MouseEvent};
use std::cell::{Ref, RefCell};

pub struct PaneSelector {
    element: RefCell<Option<Vec<ComputedElement>>>,
    labels: RefCell<Vec<String>>,
    selection: RefCell<String>,
    alphabet: String,
    mode: SessionSelectMode,
    was_zoomed: bool,
    show_session_ids: bool,
}

impl PaneSelector {
    pub fn new(term_window: &mut TermWindow, args: &SessionSelectArguments) -> Self {
        let alphabet = if args.alphabet.is_empty() {
            term_window.config.quick_select_alphabet.clone()
        } else {
            args.alphabet.clone()
        };

        // Ensure that we are un-zoomed and remember the original state
        let was_zoomed = term_window
            .set_active_runtime_zoomed(false)
            .unwrap_or(false);

        Self {
            element: RefCell::new(None),
            labels: RefCell::new(vec![]),
            selection: RefCell::new(String::new()),
            alphabet,
            mode: args.mode,
            was_zoomed,
            show_session_ids: args.show_session_ids,
        }
    }

    fn compute(
        term_window: &mut TermWindow,
        alphabet: &str,
        show_session_ids: bool,
    ) -> anyhow::Result<(Vec<ComputedElement>, Vec<String>)> {
        let font = term_window
            .fonts
            .pane_select_font()
            .expect("to resolve pane selection font");
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());

        let top_bar_height = if term_window.show_session_bar && !term_window.config.session_bar_at_bottom {
            term_window.tab_bar_pixel_height().unwrap()
        } else {
            0.
        };
        let (padding_left, padding_top) = term_window.padding_left_top();
        let border = term_window.get_os_border();
        let top_pixel_y = top_bar_height + padding_top + border.top.get() as f32;

        let panes = term_window.get_panes_to_render();
        let labels =
            crate::overlay::quickselect::compute_labels_for_alphabet(alphabet, panes.len());

        let mut elements = vec![];
        for (pane_index, pos) in panes.into_iter().enumerate() {
            let caption = if show_session_ids {
                format!("{}: {}", labels[pane_index], pos.pane.pane_id())
            } else {
                labels[pane_index].clone()
            };
            let element = Element::new(&font, ElementContent::Text(caption))
                .colors(ElementColors {
                    border: BorderColor::new(
                        term_window.config.pane_select_bg_color.to_linear().into(),
                    ),
                    bg: term_window.config.pane_select_bg_color.to_linear().into(),
                    text: term_window.config.pane_select_fg_color.to_linear().into(),
                })
                .padding(BoxDimension {
                    left: Dimension::Cells(0.25),
                    right: Dimension::Cells(0.25),
                    top: Dimension::Cells(0.),
                    bottom: Dimension::Cells(0.),
                })
                .border(BoxDimension::new(Dimension::Pixels(1.)))
                .border_corners(Some(Corners {
                    top_left: SizedPoly {
                        width: Dimension::Cells(0.25),
                        height: Dimension::Cells(0.25),
                        poly: TOP_LEFT_ROUNDED_CORNER,
                    },
                    top_right: SizedPoly {
                        width: Dimension::Cells(0.25),
                        height: Dimension::Cells(0.25),
                        poly: TOP_RIGHT_ROUNDED_CORNER,
                    },
                    bottom_left: SizedPoly {
                        width: Dimension::Cells(0.25),
                        height: Dimension::Cells(0.25),
                        poly: BOTTOM_LEFT_ROUNDED_CORNER,
                    },
                    bottom_right: SizedPoly {
                        width: Dimension::Cells(0.25),
                        height: Dimension::Cells(0.25),
                        poly: BOTTOM_RIGHT_ROUNDED_CORNER,
                    },
                }));

            let dimensions = term_window.dimensions;
            let pane_dims = pos.pane.get_dimensions();

            let computed = term_window.compute_element(
                &LayoutContext {
                    height: DimensionContext {
                        dpi: dimensions.dpi as f32,
                        pixel_max: dimensions.pixel_height as f32,
                        pixel_cell: metrics.cell_size.height as f32,
                    },
                    width: DimensionContext {
                        dpi: dimensions.dpi as f32,
                        pixel_max: dimensions.pixel_width as f32,
                        pixel_cell: metrics.cell_size.width as f32,
                    },
                    bounds: euclid::rect(
                        padding_left
                            + ((pos.left as f32 + pane_dims.cols as f32 / 2.)
                                * term_window.render_metrics.cell_size.width as f32),
                        top_pixel_y
                            + ((pos.top as f32 + pane_dims.viewport_rows as f32 / 2.)
                                * term_window.render_metrics.cell_size.height as f32),
                        pane_dims.cols as f32 * term_window.render_metrics.cell_size.width as f32,
                        pane_dims.viewport_rows as f32
                            * term_window.render_metrics.cell_size.height as f32,
                    ),
                    metrics: &metrics,
                    gl_state: term_window.render_state.as_ref().unwrap(),
                    zindex: 100,
                },
                &element,
            )?;
            elements.push(computed);
        }

        Ok((elements, labels))
    }

    fn perform_selection(
        &self,
        pane_index: usize,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<()> {
        let target = match term_window
            .get_panes_to_render()
            .into_iter()
            .nth(pane_index)
        {
            Some(pos) => pos,
            None => return Ok(()),
        };

        if !term_window.active_runtime_has_overlay() {
            match self.mode {
                SessionSelectMode::Activate => {
                    let focused_leaf = term_window.chatminal_sidebar.is_enabled()
                        && term_window.focus_active_session_terminal_instance(&target.pane);
                    if focused_leaf {
                        if let Some(window) = term_window.window.as_ref() {
                            window.invalidate();
                        }
                    } else {
                        let _ = term_window
                            .activate_terminal_handle_in_active_runtime(target.pane.pane_id() as u64);
                    }
                }
                SessionSelectMode::SwapWithActiveKeepFocus | SessionSelectMode::SwapWithActive => {
                    let swapped_leaf = term_window.chatminal_sidebar.is_enabled()
                        && term_window.swap_active_with_session_terminal_instance(
                            &target.pane,
                            self.mode == SessionSelectMode::SwapWithActiveKeepFocus,
                        );
                    if !swapped_leaf {
                        let _ = term_window.swap_active_with_terminal_handle_in_active_runtime(
                            target.pane.pane_id() as u64,
                            self.mode == SessionSelectMode::SwapWithActiveKeepFocus,
                        );
                    }
                }
                SessionSelectMode::MoveToNewWindow => {
                    if term_window.chatminal_sidebar.is_enabled() {
                        let moved_leaf =
                            crate::chatminal_runtime::desktop_move_terminal_handle_to_new_window(
                                term_window.window_id as DesktopWindowId,
                                target.pane.pane_id() as u64,
                            );
                        if !moved_leaf {
                            log::warn!(
                                "session-native move to new window is unavailable for this pane"
                            );
                        }
                    } else {
                        let host_terminal_handle = target.pane.pane_id();
                        promise::spawn::spawn(async move {
                            if let Err(err) = crate::chatminal_runtime::move_terminal_handle_to_new_runtime(
                                host_terminal_handle,
                                None,
                            )
                                .await
                            {
                                log::error!("failed to move leaf to new window: {err:#}");
                            }
                        })
                        .detach();
                    }
                }
                SessionSelectMode::MoveToNewSession => {
                    if term_window.chatminal_sidebar.is_enabled() {
                        let moved_leaf =
                            crate::chatminal_runtime::desktop_move_terminal_handle_to_new_runtime(
                                term_window.window_id as DesktopWindowId,
                                target.pane.pane_id() as u64,
                            );
                        if !moved_leaf {
                            log::warn!(
                                "session-native move to new runtime is unavailable for this pane"
                            );
                        }
                    } else {
                        let host_terminal_handle = target.pane.pane_id();
                        let window_id = term_window.window_id;
                        promise::spawn::spawn(async move {
                            if let Err(err) = crate::chatminal_runtime::move_terminal_handle_to_new_runtime(
                                host_terminal_handle,
                                Some(window_id as u64),
                            )
                                .await
                            {
                                log::error!("failed to move leaf to new surface: {err:#}");
                            }

                            let _ = crate::chatminal_runtime::focus_terminal_handle(host_terminal_handle);
                        })
                        .detach();
                    }
                }
            }
        }

        if self.was_zoomed {
            let _ = term_window.set_active_runtime_zoomed(true);
        }

        term_window.cancel_modal();
        Ok(())
    }
}

impl Modal for PaneSelector {
    fn perform_assignment(
        &self,
        _assignment: &KeyAssignment,
        _term_window: &mut TermWindow,
    ) -> bool {
        false
    }

    fn mouse_event(&self, _event: MouseEvent, _term_window: &mut TermWindow) -> anyhow::Result<()> {
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
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                // Type to add to the selection
                let mut selection = self.selection.borrow_mut();
                selection.push(c);

                // and if we have a complete match, activate that pane
                if let Some(pane_index) = self.labels.borrow().iter().position(|s| s == &*selection)
                {
                    self.perform_selection(pane_index, term_window)?;
                    return Ok(true);
                }
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                // Backspace to edit the selection
                let mut selection = self.selection.borrow_mut();
                selection.pop();
            }
            (KeyCode::Char('u'), KeyModifiers::CTRL) => {
                // CTRL-u to clear the selection
                let mut selection = self.selection.borrow_mut();
                selection.clear();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<Ref<'_, [ComputedElement]>> {
        if self.element.borrow().is_none() {
            let (element, labels) = Self::compute(term_window, &self.alphabet, self.show_session_ids)?;
            self.element.borrow_mut().replace(element);
            *self.labels.borrow_mut() = labels;
        }
        Ok(Ref::map(self.element.borrow(), |v| {
            v.as_ref().unwrap().as_slice()
        }))
    }

    fn reconfigure(&self, _term_window: &mut TermWindow) {
        self.element.borrow_mut().take();
    }
}
