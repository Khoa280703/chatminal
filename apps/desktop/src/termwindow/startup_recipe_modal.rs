use crate::color::LinearRgba;
use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::{TermWindow, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::Dimension;
use std::cell::{Ref, RefCell};
use std::time::{Duration, Instant};
use terminal_emulator::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

const MODAL_MAX_VISIBLE_LINES: usize = 10;
const MODAL_MIN_INPUT_HEIGHT_PX: f32 = 220.0;
const MODAL_INPUT_HORIZONTAL_PADDING_PX: f32 = 12.0;
const MODAL_VISIBLE_COLUMN_GUTTER: usize = 6;
const MODAL_CURSOR_BLINK_MS: u64 = 530;
const MODAL_CURSOR_GLYPH: char = '|';

#[derive(Clone, Copy)]
struct LineSpan {
    start: usize,
    end: usize,
}

pub struct StartupRecipeModal {
    element: RefCell<Option<Vec<ComputedElement>>>,
    session_id: String,
    value: RefCell<String>,
    cursor: RefCell<usize>,
    select_all: RefCell<bool>,
    preferred_column: RefCell<Option<usize>>,
    scroll_top_line: RefCell<usize>,
    horizontal_scroll_col: RefCell<usize>,
}

impl StartupRecipeModal {
    pub fn new(session_id: String, value: String) -> Self {
        let cursor = value.len();
        Self {
            element: RefCell::new(None),
            session_id,
            value: RefCell::new(value),
            cursor: RefCell::new(cursor),
            select_all: RefCell::new(false),
            preferred_column: RefCell::new(None),
            scroll_top_line: RefCell::new(0),
            horizontal_scroll_col: RefCell::new(0),
        }
    }

    fn command_font(
        term_window: &mut TermWindow,
    ) -> anyhow::Result<std::rc::Rc<terminal_font::LoadedFont>> {
        term_window.fonts.command_palette_font()
    }

    fn current_value(&self) -> String {
        self.value.borrow().clone()
    }

    fn current_cursor(&self) -> usize {
        let value = self.value.borrow();
        (*self.cursor.borrow()).min(value.len())
    }

    fn clear_select_all(&self) {
        *self.select_all.borrow_mut() = false;
    }

    fn save_only(&self, term_window: &mut TermWindow) {
        let value = self.current_value();
        term_window.set_startup_command_chatminal_session(&self.session_id, &value);
    }

    fn save(&self, term_window: &mut TermWindow) {
        self.save_only(term_window);
        term_window.cancel_modal();
    }

    fn save_and_run(&self, term_window: &mut TermWindow) {
        self.save_only(term_window);
        term_window.run_startup_command_chatminal_session(&self.session_id);
        term_window.cancel_modal();
    }

    fn replace_selection_if_needed(&self) {
        if !*self.select_all.borrow() {
            return;
        }
        self.value.borrow_mut().clear();
        *self.cursor.borrow_mut() = 0;
        *self.select_all.borrow_mut() = false;
        *self.preferred_column.borrow_mut() = None;
        *self.scroll_top_line.borrow_mut() = 0;
        *self.horizontal_scroll_col.borrow_mut() = 0;
    }

    fn insert_text(&self, text: &str) {
        self.replace_selection_if_needed();
        let cursor = self.current_cursor();
        let mut value = self.value.borrow_mut();
        value.insert_str(cursor, text);
        *self.cursor.borrow_mut() = cursor + text.len();
        *self.preferred_column.borrow_mut() = None;
    }

    fn push_char(&self, c: char) {
        let mut buffer = [0u8; 4];
        self.insert_text(c.encode_utf8(&mut buffer));
    }

    fn push_newline(&self) {
        self.insert_text("\n");
    }

    fn backspace(&self) {
        if *self.select_all.borrow() {
            self.value.borrow_mut().clear();
            *self.cursor.borrow_mut() = 0;
            *self.select_all.borrow_mut() = false;
            *self.preferred_column.borrow_mut() = None;
            return;
        }

        let cursor = self.current_cursor();
        if cursor == 0 {
            return;
        }
        let value_snapshot = self.current_value();
        let prev = previous_char_boundary(&value_snapshot, cursor);
        self.value.borrow_mut().replace_range(prev..cursor, "");
        *self.cursor.borrow_mut() = prev;
        *self.preferred_column.borrow_mut() = None;
    }

    fn clear(&self) {
        self.value.borrow_mut().clear();
        *self.cursor.borrow_mut() = 0;
        *self.select_all.borrow_mut() = false;
        *self.preferred_column.borrow_mut() = None;
        *self.scroll_top_line.borrow_mut() = 0;
        *self.horizontal_scroll_col.borrow_mut() = 0;
    }

    fn select_all(&self) {
        *self.select_all.borrow_mut() = true;
    }

    fn move_left(&self) {
        if *self.select_all.borrow() {
            *self.cursor.borrow_mut() = 0;
            self.clear_select_all();
            *self.preferred_column.borrow_mut() = None;
            return;
        }
        let value = self.current_value();
        let cursor = self.current_cursor();
        if cursor == 0 {
            return;
        }
        *self.cursor.borrow_mut() = previous_char_boundary(&value, cursor);
        *self.preferred_column.borrow_mut() = None;
    }

    fn move_right(&self) {
        let value = self.current_value();
        if *self.select_all.borrow() {
            *self.cursor.borrow_mut() = value.len();
            self.clear_select_all();
            *self.preferred_column.borrow_mut() = None;
            return;
        }
        let cursor = self.current_cursor();
        if cursor >= value.len() {
            return;
        }
        *self.cursor.borrow_mut() = next_char_boundary(&value, cursor);
        *self.preferred_column.borrow_mut() = None;
    }

    fn move_home(&self) {
        let value = self.current_value();
        let cursor = self.current_cursor();
        let (line_idx, _, spans) = locate_cursor(&value, cursor);
        *self.cursor.borrow_mut() = spans[line_idx].start;
        self.clear_select_all();
        *self.preferred_column.borrow_mut() = None;
    }

    fn move_end(&self) {
        let value = self.current_value();
        let cursor = self.current_cursor();
        let (line_idx, _, spans) = locate_cursor(&value, cursor);
        *self.cursor.borrow_mut() = spans[line_idx].end;
        self.clear_select_all();
        *self.preferred_column.borrow_mut() = None;
    }

    fn move_vertical(&self, delta: isize) {
        let value = self.current_value();
        let cursor = self.current_cursor();
        let (line_idx, col, spans) = locate_cursor(&value, cursor);
        if spans.is_empty() {
            return;
        }

        let target_line = line_idx.saturating_add_signed(delta).min(spans.len() - 1);
        if target_line == line_idx {
            return;
        }

        let preferred = self.preferred_column.borrow().unwrap_or(col);
        let target_span = spans[target_line];
        let target_text = &value[target_span.start..target_span.end];
        let target_col = preferred.min(target_text.chars().count());
        let target_cursor = target_span.start + byte_offset_for_char_pos(target_text, target_col);
        *self.cursor.borrow_mut() = target_cursor;
        *self.preferred_column.borrow_mut() = Some(preferred);
        self.clear_select_all();
    }

    fn ensure_cursor_visible(&self, visible_columns: usize) -> (usize, usize) {
        let value = self.current_value();
        let cursor = self.current_cursor();
        let (line_idx, col, spans) = locate_cursor(&value, cursor);

        let mut top = *self.scroll_top_line.borrow();
        if line_idx < top {
            top = line_idx;
        } else if line_idx >= top.saturating_add(MODAL_MAX_VISIBLE_LINES) {
            top = line_idx.saturating_sub(MODAL_MAX_VISIBLE_LINES.saturating_sub(1));
        }
        let max_top = spans.len().saturating_sub(MODAL_MAX_VISIBLE_LINES);
        top = top.min(max_top);
        *self.scroll_top_line.borrow_mut() = top;

        let mut left = *self.horizontal_scroll_col.borrow();
        if col < left {
            left = col;
        } else if col >= left.saturating_add(visible_columns) {
            left = col.saturating_sub(visible_columns.saturating_sub(1));
        }
        *self.horizontal_scroll_col.borrow_mut() = left;

        (top, left)
    }

    fn visible_recipe_lines(&self, visible_columns: usize, cursor_visible: bool) -> Vec<String> {
        let value = self.current_value();
        let cursor = self.current_cursor();
        let (cursor_line, cursor_col, spans) = locate_cursor(&value, cursor);
        let (top, left) = self.ensure_cursor_visible(visible_columns.max(1));

        spans
            .iter()
            .enumerate()
            .skip(top)
            .take(MODAL_MAX_VISIBLE_LINES)
            .map(|(line_idx, span)| {
                let line = &value[span.start..span.end];
                let total_cols = line.chars().count();
                let visible_start = left.min(total_cols);
                let visible_end = (visible_start + visible_columns).min(total_cols);
                let mut rendered = slice_chars(line, visible_start, visible_end);
                if cursor_visible && line_idx == cursor_line {
                    let cursor_in_view =
                        cursor_col.clamp(visible_start, visible_end) - visible_start;
                    let insert_byte = byte_offset_for_char_pos(&rendered, cursor_in_view);
                    rendered.insert(insert_byte, MODAL_CURSOR_GLYPH);
                }
                if rendered.is_empty() && line_idx == cursor_line {
                    rendered.push(if cursor_visible {
                        MODAL_CURSOR_GLYPH
                    } else {
                        ' '
                    });
                }
                if visible_start > 0 {
                    rendered.insert(0, '…');
                }
                if visible_end < total_cols {
                    rendered.push('…');
                }
                rendered
            })
            .collect()
    }

    fn cursor_visible(term_window: &TermWindow) -> bool {
        (term_window.created.elapsed().as_millis() / MODAL_CURSOR_BLINK_MS as u128)
            .is_multiple_of(2)
    }

    fn schedule_cursor_blink(term_window: &TermWindow) {
        let next_due = Instant::now() + Duration::from_millis(MODAL_CURSOR_BLINK_MS / 2);
        let mut slot = term_window.has_animation.borrow_mut();
        match *slot {
            Some(existing) if existing <= next_due => {}
            _ => *slot = Some(next_due),
        }
    }

    fn resolve_modal_hit(term_window: &TermWindow) -> Option<UIItemType> {
        let event = term_window.current_mouse_event.as_ref()?;
        term_window
            .ui_items
            .iter()
            .rev()
            .find(|item| item.hit_test(event.coords.x, event.coords.y))
            .map(|item| item.item_type.clone())
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

    fn action_button(
        font: &std::rc::Rc<terminal_font::LoadedFont>,
        label: &str,
        item_type: UIItemType,
        border: LinearRgba,
        bg: LinearRgba,
        hover_border: LinearRgba,
        hover: LinearRgba,
        text: LinearRgba,
    ) -> Element {
        Element::new(font, ElementContent::Text(label.to_string()))
            .item_type(item_type)
            .display(DisplayType::Inline)
            .padding(BoxDimension {
                left: Dimension::Pixels(14.0),
                right: Dimension::Pixels(14.0),
                top: Dimension::Pixels(7.0),
                bottom: Dimension::Pixels(7.0),
            })
            .margin(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(10.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(0.0),
            })
            .border(BoxDimension::new(Dimension::Pixels(1.0)))
            .border_corners(Some(rounded_corners(5.0)))
            .colors(ElementColors {
                border: BorderColor::new(border),
                bg: bg.into(),
                text: text.into(),
            })
            .hover_colors(Some(ElementColors {
                border: BorderColor::new(hover_border),
                bg: hover.into(),
                text: text.into(),
            }))
    }

    fn compute(&self, term_window: &mut TermWindow) -> anyhow::Result<Vec<ComputedElement>> {
        Self::schedule_cursor_blink(term_window);
        let font = Self::command_font(term_window)?;
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let dimensions = term_window.dimensions;
        let max_panel_width = (dimensions.pixel_width as f32 - 48.0).max(360.0);
        let width_px = (dimensions.pixel_width as f32 * 0.54)
            .clamp(560.0, 820.0)
            .min(max_panel_width);
        let height_px = dimensions.pixel_height as f32;
        let panel_x = ((dimensions.pixel_width as f32 - width_px) / 2.0).max(24.0);
        let panel_y = (dimensions.pixel_height as f32 * 0.14).max(42.0);
        let panel_height = (dimensions.pixel_height as f32 * 0.68).max(400.0);
        let visible_columns = ((width_px - MODAL_INPUT_HORIZONTAL_PADDING_PX * 2.0)
            / metrics.cell_size.width as f32)
            .floor()
            .max(8.0) as usize;
        let visible_columns = visible_columns.saturating_sub(MODAL_VISIBLE_COLUMN_GUTTER);

        let backdrop = Element::new(&font, ElementContent::Children(vec![]))
            .item_type(UIItemType::ChatminalStartupRecipeModalBackdrop)
            .display(DisplayType::Block)
            .min_width(Some(Dimension::Pixels(dimensions.pixel_width as f32)))
            .min_height(Some(Dimension::Pixels(dimensions.pixel_height as f32)))
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::with_components(0.0, 0.0, 0.0, 0.58).into(),
                text: LinearRgba::TRANSPARENT.into(),
            });

        let text = LinearRgba::with_components(0.88, 0.88, 0.88, 1.0);
        let sub_muted = LinearRgba::with_components(0.50, 0.50, 0.50, 1.0);
        let panel_bg = LinearRgba::with_components(0.02, 0.02, 0.02, 1.0);
        let border = LinearRgba::with_components(0.12, 0.12, 0.12, 1.0);
        let cursor_visible = Self::cursor_visible(term_window);
        let input_bg = if *self.select_all.borrow() {
            LinearRgba::with_components(0.08, 0.08, 0.08, 1.0)
        } else {
            LinearRgba::with_components(0.01, 0.01, 0.01, 1.0)
        };
        let input_border = if *self.select_all.borrow() {
            LinearRgba::with_components(0.34, 0.34, 0.34, 1.0)
        } else {
            LinearRgba::with_components(0.18, 0.18, 0.18, 1.0)
        };
        let button_border = LinearRgba::with_components(0.18, 0.18, 0.18, 1.0);
        let button_bg = panel_bg;
        let button_hover_border = LinearRgba::with_components(0.28, 0.28, 0.28, 1.0);
        let button_hover = LinearRgba::with_components(0.06, 0.06, 0.06, 1.0);
        let save_border = LinearRgba::with_components(0.96, 0.96, 0.96, 1.0);
        let save_bg = LinearRgba::with_components(0.96, 0.96, 0.96, 1.0);
        let save_hover = LinearRgba::with_components(0.86, 0.86, 0.86, 1.0);
        let save_text = LinearRgba::with_components(0.03, 0.03, 0.03, 1.0);

        let recipe_lines: Vec<Element> = self
            .visible_recipe_lines(visible_columns.max(1), cursor_visible)
            .into_iter()
            .map(|line| Self::text_block(&font, line, text))
            .collect();

        let editor = Element::new(
            &font,
            ElementContent::Children(vec![Element::new(
                &font,
                ElementContent::Children(recipe_lines),
            )
            .item_type(UIItemType::ChatminalStartupRecipeModalInput)
            .display(DisplayType::Block)
            .min_width(Some(Dimension::Percent(1.0)))
            .min_height(Some(Dimension::Pixels(MODAL_MIN_INPUT_HEIGHT_PX)))
            .padding(BoxDimension {
                left: Dimension::Pixels(MODAL_INPUT_HORIZONTAL_PADDING_PX),
                right: Dimension::Pixels(MODAL_INPUT_HORIZONTAL_PADDING_PX),
                top: Dimension::Pixels(12.0),
                bottom: Dimension::Pixels(12.0),
            })
            .border(BoxDimension::new(Dimension::Pixels(1.0)))
            .border_corners(Some(rounded_corners(4.0)))
            .colors(ElementColors {
                border: BorderColor::new(input_border),
                bg: input_bg.into(),
                text: text.into(),
            })]),
        )
        .item_type(UIItemType::ChatminalStartupRecipeModalPanel)
        .display(DisplayType::Block)
        .padding(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(ElementColors {
            border: BorderColor::default(),
            bg: LinearRgba::TRANSPARENT.into(),
            text: text.into(),
        });

        let footer_actions = Element::new(
            &font,
            ElementContent::Children(vec![
                Self::action_button(
                    &font,
                    "Cancel",
                    UIItemType::ChatminalStartupRecipeModalCancel,
                    button_border,
                    button_bg,
                    button_hover_border,
                    button_hover,
                    text,
                )
                .float(Float::Right),
                Self::action_button(
                    &font,
                    "Save",
                    UIItemType::ChatminalStartupRecipeModalSave,
                    save_border,
                    save_bg,
                    save_border,
                    save_hover,
                    save_text,
                )
                .float(Float::Right),
                Self::action_button(
                    &font,
                    "Run now",
                    UIItemType::ChatminalStartupRecipeModalRun,
                    button_border,
                    button_bg,
                    button_hover_border,
                    button_hover,
                    text,
                )
                .float(Float::Right),
            ]),
        )
        .display(DisplayType::Block);

        let panel = Element::new(
            &font,
            ElementContent::Children(vec![
                Self::text_block(&font, "Startup recipe", text).padding(BoxDimension {
                    left: Dimension::Pixels(0.0),
                    right: Dimension::Pixels(0.0),
                    top: Dimension::Pixels(0.0),
                    bottom: Dimension::Pixels(4.0),
                }),
                Self::text_block(
                    &font,
                    "Save a command sequence for this session.",
                    sub_muted,
                )
                .padding(BoxDimension {
                    left: Dimension::Pixels(0.0),
                    right: Dimension::Pixels(0.0),
                    top: Dimension::Pixels(0.0),
                    bottom: Dimension::Pixels(14.0),
                }),
                editor.padding(BoxDimension {
                    left: Dimension::Pixels(0.0),
                    right: Dimension::Pixels(0.0),
                    top: Dimension::Pixels(0.0),
                    bottom: Dimension::Pixels(12.0),
                }),
                Self::text_block(
                    &font,
                    "Esc close  Save: Ctrl/Cmd+Enter  Run: Ctrl/Cmd+R",
                    sub_muted,
                )
                .padding(BoxDimension {
                    left: Dimension::Pixels(0.0),
                    right: Dimension::Pixels(0.0),
                    top: Dimension::Pixels(0.0),
                    bottom: Dimension::Pixels(14.0),
                }),
                footer_actions,
            ]),
        )
        .item_type(UIItemType::ChatminalStartupRecipeModalPanel)
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
        let backdrop_computed = term_window.compute_element(
            &LayoutContext {
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
                bounds: euclid::rect(0.0, 0.0, dimensions.pixel_width as f32, height_px),
                metrics: &metrics,
                gl_state,
                custom_block_glyphs: term_window.config.custom_block_glyphs,
                zindex: 120,
            },
            &backdrop,
        )?;
        let panel_computed = term_window.compute_element(
            &LayoutContext {
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
                bounds: euclid::rect(panel_x, panel_y, width_px, panel_height),
                metrics: &metrics,
                gl_state,
                custom_block_glyphs: term_window.config.custom_block_glyphs,
                zindex: 121,
            },
            &panel,
        )?;
        Ok(vec![backdrop_computed, panel_computed])
    }
}

impl Modal for StartupRecipeModal {
    fn mouse_event(&self, event: MouseEvent, term_window: &mut TermWindow) -> anyhow::Result<()> {
        if event.kind != MouseEventKind::Release || event.button != MouseButton::Left {
            return Ok(());
        }
        match Self::resolve_modal_hit(term_window) {
            Some(UIItemType::ChatminalStartupRecipeModalSave) => self.save(term_window),
            Some(UIItemType::ChatminalStartupRecipeModalRun) => self.save_and_run(term_window),
            Some(UIItemType::ChatminalStartupRecipeModalCancel)
            | Some(UIItemType::ChatminalStartupRecipeModalBackdrop) => term_window.cancel_modal(),
            Some(UIItemType::ChatminalStartupRecipeModalPanel)
            | Some(UIItemType::ChatminalStartupRecipeModalInput) => {}
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
        match key {
            KeyCode::Escape if mods == KeyModifiers::NONE => {
                term_window.cancel_modal();
            }
            KeyCode::Char('g') if mods == KeyModifiers::CTRL => {
                term_window.cancel_modal();
            }
            KeyCode::Char('r')
                if mods.contains(KeyModifiers::CTRL) || mods.contains(KeyModifiers::SUPER) =>
            {
                self.save_and_run(term_window);
            }
            KeyCode::Enter
                if mods.contains(KeyModifiers::CTRL) || mods.contains(KeyModifiers::SUPER) =>
            {
                self.save(term_window);
            }
            KeyCode::Enter => self.push_newline(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::LeftArrow if mods == KeyModifiers::NONE => self.move_left(),
            KeyCode::RightArrow if mods == KeyModifiers::NONE => self.move_right(),
            KeyCode::UpArrow if mods == KeyModifiers::NONE => self.move_vertical(-1),
            KeyCode::DownArrow if mods == KeyModifiers::NONE => self.move_vertical(1),
            KeyCode::Home if mods == KeyModifiers::NONE => self.move_home(),
            KeyCode::End if mods == KeyModifiers::NONE => self.move_end(),
            KeyCode::Char('u') if mods == KeyModifiers::CTRL => self.clear(),
            KeyCode::Char('a') if mods == KeyModifiers::CTRL || mods == KeyModifiers::SUPER => {
                self.select_all()
            }
            KeyCode::Char(c)
                if mods == KeyModifiers::NONE
                    || mods == KeyModifiers::SHIFT
                    || mods == KeyModifiers::ALT =>
            {
                self.push_char(c)
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
        self.element
            .borrow_mut()
            .replace(self.compute(term_window)?);
        Ok(Ref::map(self.element.borrow(), |value| {
            value.as_ref().unwrap().as_slice()
        }))
    }

    fn reconfigure(&self, _term_window: &mut TermWindow) {
        self.element.borrow_mut().take();
    }
}

fn previous_char_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_char_boundary(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .chars()
        .next()
        .map(|ch| cursor + ch.len_utf8())
        .unwrap_or_else(|| value.len())
}

fn line_spans(value: &str) -> Vec<LineSpan> {
    let mut spans = Vec::new();
    let mut start = 0;
    for (idx, ch) in value.char_indices() {
        if ch == '\n' {
            spans.push(LineSpan { start, end: idx });
            start = idx + ch.len_utf8();
        }
    }
    spans.push(LineSpan {
        start,
        end: value.len(),
    });
    spans
}

fn locate_cursor(value: &str, cursor: usize) -> (usize, usize, Vec<LineSpan>) {
    let spans = line_spans(value);
    let mut line_idx = spans.len().saturating_sub(1);
    for (idx, span) in spans.iter().enumerate() {
        let next_start = spans
            .get(idx + 1)
            .map(|next| next.start)
            .unwrap_or(usize::MAX);
        if cursor < next_start {
            line_idx = idx;
            break;
        }
        if cursor <= span.end {
            line_idx = idx;
            break;
        }
    }
    let span = spans[line_idx];
    let col_end = cursor.min(span.end);
    let col = value[span.start..col_end].chars().count();
    (line_idx, col, spans)
}

fn byte_offset_for_char_pos(value: &str, char_pos: usize) -> usize {
    if char_pos == 0 {
        return 0;
    }
    value
        .char_indices()
        .nth(char_pos)
        .map(|(idx, _)| idx)
        .unwrap_or(value.len())
}

fn slice_chars(value: &str, start_col: usize, end_col: usize) -> String {
    value
        .chars()
        .skip(start_col)
        .take(end_col.saturating_sub(start_col))
        .collect()
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
