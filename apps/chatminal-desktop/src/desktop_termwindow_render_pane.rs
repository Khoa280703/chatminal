use crate::desktop_host_runtime::overlay_shell::{
    OverlayWithPaneLines as WithPaneLines, RenderableDimensions, StableCursorPosition,
};
use crate::desktop_termwindow_types::terminal_ui_key_for_pane;
use crate::desktop_termwindow_types::{TerminalPaneLayout, TerminalUiKey};
use crate::quad::{HeapQuadAllocator, QuadTrait, TripleLayerQuadAllocator};
use crate::selection::SelectionRange;
use crate::termwindow::box_model::*;
use crate::termwindow::render::{
    same_hyperlink, CursorProperties, LineQuadCacheKey, LineQuadCacheValue, LineToEleShapeCacheKey,
    RenderScreenLineParams,
};
use crate::termwindow::{ScrollHit, UIItem, UIItemType};
use anyhow::Context;
use config::VisualBellTarget;
use engine_dynamic::Value;
use engine_term::color::{ColorAttribute, ColorPalette};
use engine_term::{Line, StableRowIndex};
use ordered_float::NotNan;
use std::time::Instant;
use window::bitmaps::TextureRect;
use window::color::LinearRgba;
use window::DeadKeyStatus;
use window::PointF;
use window::RectF;

struct PanePixelRects {
    background_rect: RectF,
    content_rect: RectF,
    grid_origin: PointF,
    content_top: f32,
}

fn clamp_renderable_dimensions_to_layout(
    dims: RenderableDimensions,
    pos: &TerminalPaneLayout,
) -> RenderableDimensions {
    RenderableDimensions {
        cols: pos.width.max(1).min(dims.cols.max(1)),
        viewport_rows: pos.height.max(1).min(dims.viewport_rows.max(1)),
        pixel_width: pos.pixel_width.max(1).min(dims.pixel_width.max(1)),
        pixel_height: pos.pixel_height.max(1).min(dims.pixel_height.max(1)),
        ..dims
    }
}

impl crate::TermWindow {
    fn pane_pixel_rects(&self, pos: &TerminalPaneLayout) -> PanePixelRects {
        let sb = self.shell_bounds();
        let grid_origin = self.terminal_grid_origin();
        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;

        let x = if pos.left == 0 {
            0.0
        } else {
            grid_origin.x - (cell_width / 2.0) + (pos.left as f32 * cell_width)
        };
        let width_delta = if pos.left == 0 {
            grid_origin.x + (cell_width / 2.0)
        } else {
            cell_width
        };

        let y = if pos.top == 0 {
            sb.content_y
        } else {
            grid_origin.y - (cell_height / 2.0) + (pos.top as f32 * cell_height)
        };
        let height_delta = if pos.top == 0 {
            (grid_origin.y - sb.content_y) + (cell_height / 2.0)
        } else {
            cell_height
        };

        let background_rect = euclid::rect(
            x,
            y,
            if pos.left + pos.width >= self.terminal_size.cols as usize {
                self.dimensions.pixel_width as f32 - x
            } else {
                (pos.width as f32 * cell_width) + width_delta
            },
            if pos.top + pos.height >= self.terminal_size.rows as usize {
                self.dimensions.pixel_height as f32 - y
            } else {
                (pos.height as f32 * cell_height) + height_delta
            },
        );

        let content_rect = euclid::rect(
            grid_origin.x - (cell_width / 2.0) + (pos.left as f32 * cell_width),
            grid_origin.y - (cell_height / 2.0) + (pos.top as f32 * cell_height),
            pos.width as f32 * cell_width,
            pos.height as f32 * cell_height,
        );

        PanePixelRects {
            background_rect,
            content_rect,
            grid_origin,
            content_top: sb.content_y,
        }
    }

    pub(crate) fn scroll_bar_track_geometry_for_pane(
        &self,
        pane_id: u64,
    ) -> Option<(usize, usize, usize, usize)> {
        let pos = self
            .get_panes_to_render()
            .into_iter()
            .find(|pos| terminal_ui_key_for_pane(&*pos.pane) == pane_id)?;
        let pane_rects = self.pane_pixel_rects(&pos);
        let padding = self.effective_right_padding(&self.config);
        let thumb_x = pane_rects
            .background_rect
            .max_x()
            .round()
            .max(padding as f32) as usize
            - padding;
        let thumb_y = pane_rects.content_rect.min_y().max(pane_rects.content_top) as usize;
        let track_height = pane_rects.content_rect.height().max(0.0).round() as usize;
        Some((thumb_x, thumb_y, padding, track_height))
    }

    fn paint_pane_box_model(&mut self, pos: &TerminalPaneLayout) -> anyhow::Result<()> {
        let computed = self.build_pane(pos)?;
        let mut ui_items = computed.ui_items();
        self.ui_items.append(&mut ui_items);
        let gl_state = self.render_state.as_ref().unwrap();
        self.render_element(&computed, gl_state, None)
    }

    pub fn paint_pane(
        &mut self,
        pos: &TerminalPaneLayout,
        layers: &mut TripleLayerQuadAllocator,
    ) -> anyhow::Result<()> {
        if self.config.use_box_model_render {
            return self.paint_pane_box_model(pos);
        }

        self.check_for_dirty_lines_and_invalidate_selection(&pos.pane);
        /*
        let zone = {
            let dims = pos.pane.get_dimensions();
            let position = self
                .get_viewport(terminal_ui_key_for_pane(&*pos.pane))
                .unwrap_or(dims.physical_top);

            let zones = self.get_semantic_zones(&pos.pane);
            let idx = match zones.binary_search_by(|zone| zone.start_y.cmp(&position)) {
                Ok(idx) | Err(idx) => idx,
            };
            let idx = ((idx as isize) - 1).max(0) as usize;
            zones.get(idx).cloned()
        };
        */

        let global_cursor_fg = self.palette().cursor_fg;
        let global_cursor_bg = self.palette().cursor_bg;
        let config = self.config.clone();
        let palette = pos.pane.palette();
        let pane_rects = self.pane_pixel_rects(pos);

        let top_pixel_y = pane_rects.grid_origin.y
            + (pos.top as f32 * self.render_metrics.cell_size.height as f32);

        let cursor = pos.pane.get_cursor_position();
        if pos.is_active {
            self.prev_cursor.update(&cursor);
        }

        let pane_id = terminal_ui_key_for_pane(&*pos.pane);
        let current_viewport = self.get_viewport(pane_id);
        let dims = clamp_renderable_dimensions_to_layout(pos.pane.get_dimensions(), pos);

        let gl_state = self.render_state.as_ref().unwrap();

        let cursor_border_color = palette.cursor_border.to_linear();
        let foreground = palette.foreground.to_linear();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();

        let window_is_transparent =
            !self.window_background.is_empty() || config.window_background_opacity != 1.0;

        let default_bg = palette
            .resolve_bg(ColorAttribute::Default)
            .to_linear()
            .mul_alpha(if window_is_transparent {
                0.
            } else {
                config.text_background_opacity
            });

        let background_rect = pane_rects.background_rect;

        if self.window_background.is_empty() {
            // Per-pane, palette-specified background

            let mut quad = self
                .filled_rectangle(
                    layers,
                    0,
                    background_rect,
                    palette
                        .background
                        .to_linear()
                        .mul_alpha(config.window_background_opacity),
                )
                .context("filled_rectangle")?;
            quad.set_hsv(if pos.is_active {
                None
            } else {
                Some(config.inactive_pane_hsb)
            });
        }

        {
            // If the bell is ringing, we draw another background layer over the
            // top of this in the configured bell color
            if let Some(intensity) = self.get_intensity_if_bell_target_ringing(
                &pos.pane,
                &config,
                VisualBellTarget::BackgroundColor,
            ) {
                // target background color
                let LinearRgba(r, g, b, _) = config
                    .resolved_palette
                    .visual_bell
                    .as_deref()
                    .unwrap_or(&palette.foreground)
                    .to_linear();

                let background = if window_is_transparent {
                    // for transparent windows, we fade in the target color
                    // by adjusting its alpha
                    LinearRgba::with_components(r, g, b, intensity)
                } else {
                    // otherwise We'll interpolate between the background color
                    // and the the target color
                    let (r1, g1, b1, a) = palette
                        .background
                        .to_linear()
                        .mul_alpha(config.window_background_opacity)
                        .tuple();
                    LinearRgba::with_components(
                        r1 + (r - r1) * intensity,
                        g1 + (g - g1) * intensity,
                        b1 + (b - b1) * intensity,
                        a,
                    )
                };
                log::trace!("bell color is {:?}", background);

                let mut quad = self
                    .filled_rectangle(layers, 0, background_rect, background)
                    .context("filled_rectangle")?;

                quad.set_hsv(if pos.is_active {
                    None
                } else {
                    Some(config.inactive_pane_hsb)
                });
            }
        }

        if self.show_scroll_bar {
            let thumb_y_offset =
                pane_rects.content_rect.min_y().max(pane_rects.content_top) as usize;
            let track_height = pane_rects.content_rect.height().max(0.0).round() as usize;
            if track_height > 0 {
                let min_height = self.min_scroll_bar_height();

                let info = ScrollHit::thumb(
                    &*pos.pane,
                    current_viewport,
                    track_height,
                    min_height as usize,
                );
                let abs_thumb_top = thumb_y_offset + info.top;
                let thumb_size = info.height;
                let color = palette.scrollbar_thumb.to_linear();

                let config = &self.config;
                let padding = self.effective_right_padding(&config) as f32;
                let track_right = pane_rects.background_rect.max_x().round().max(padding) as usize;
                let thumb_x = track_right.saturating_sub(padding.round() as usize);
                let track_bottom = thumb_y_offset + track_height;
                let below_thumb_y = abs_thumb_top + thumb_size;
                let (r, g, b, _) = color.tuple();

                self.ui_items.push(UIItem {
                    x: thumb_x,
                    width: padding as usize,
                    y: thumb_y_offset,
                    height: info.top,
                    item_type: UIItemType::AboveScrollThumb(pane_id),
                });
                self.ui_items.push(UIItem {
                    x: thumb_x,
                    width: padding as usize,
                    y: abs_thumb_top,
                    height: thumb_size,
                    item_type: UIItemType::ScrollThumb(pane_id),
                });
                self.ui_items.push(UIItem {
                    x: thumb_x,
                    width: padding as usize,
                    y: abs_thumb_top + thumb_size,
                    height: self
                        .dimensions
                        .pixel_height
                        .min(track_bottom)
                        .saturating_sub(below_thumb_y),
                    item_type: UIItemType::BelowScrollThumb(pane_id),
                });

                self.filled_rectangle(
                    layers,
                    1,
                    euclid::rect(
                        thumb_x as f32,
                        thumb_y_offset as f32,
                        padding,
                        track_height as f32,
                    ),
                    LinearRgba::with_components(r, g, b, 0.16),
                )
                .context("filled_rectangle")?;

                self.filled_rectangle(
                    layers,
                    2,
                    euclid::rect(
                        thumb_x as f32,
                        abs_thumb_top as f32,
                        padding,
                        thumb_size as f32,
                    ),
                    color,
                )
                .context("filled_rectangle")?;
            }
        }

        let (selrange, rectangular) = {
            let sel = self.selection(pane_id);
            (sel.range.clone(), sel.rectangular)
        };

        let start = Instant::now();
        let selection_fg = palette.selection_fg.to_linear();
        let selection_bg = palette.selection_bg.to_linear();
        let cursor_fg = palette.cursor_fg.to_linear();
        let cursor_bg = palette.cursor_bg.to_linear();
        let cursor_is_default_color =
            palette.cursor_fg == global_cursor_fg && palette.cursor_bg == global_cursor_bg;

        {
            let stable_range = match current_viewport {
                Some(top) => top..top + dims.viewport_rows as StableRowIndex,
                None => dims.physical_top..dims.physical_top + dims.viewport_rows as StableRowIndex,
            };

            pos.pane
                .apply_hyperlinks(stable_range.clone(), &self.config.hyperlink_rules);

            struct LineRender<'a, 'b> {
                term_window: &'a mut crate::TermWindow,
                selrange: Option<SelectionRange>,
                rectangular: bool,
                dims: RenderableDimensions,
                top_pixel_y: f32,
                left_pixel_x: f32,
                pos: &'a TerminalPaneLayout,
                pane_id: TerminalUiKey,
                cursor: &'a StableCursorPosition,
                palette: &'a ColorPalette,
                default_bg: LinearRgba,
                cursor_border_color: LinearRgba,
                selection_fg: LinearRgba,
                selection_bg: LinearRgba,
                cursor_fg: LinearRgba,
                cursor_bg: LinearRgba,
                foreground: LinearRgba,
                cursor_is_default_color: bool,
                white_space: TextureRect,
                filled_box: TextureRect,
                window_is_transparent: bool,
                layers: &'a mut TripleLayerQuadAllocator<'b>,
                error: Option<anyhow::Error>,
            }

            let left_pixel_x = pane_rects.grid_origin.x
                + (pos.left as f32 * self.render_metrics.cell_size.width as f32);

            let mut render = LineRender {
                term_window: self,
                selrange,
                rectangular,
                dims,
                top_pixel_y,
                left_pixel_x,
                pos,
                pane_id,
                cursor: &cursor,
                palette: &palette,
                cursor_border_color,
                selection_fg,
                selection_bg,
                cursor_fg,
                default_bg,
                cursor_bg,
                foreground,
                cursor_is_default_color,
                white_space,
                filled_box,
                window_is_transparent,
                layers,
                error: None,
            };

            impl<'a, 'b> LineRender<'a, 'b> {
                fn render_line(
                    &mut self,
                    stable_top: StableRowIndex,
                    line_idx: usize,
                    line: &&mut Line,
                ) -> anyhow::Result<()> {
                    let stable_row = stable_top + line_idx as StableRowIndex;
                    let selrange = self
                        .selrange
                        .map_or(0..0, |sel| sel.cols_for_row(stable_row, self.rectangular));
                    // Constrain to the pane width!
                    let selrange = selrange.start..selrange.end.min(self.dims.cols);

                    let (cursor, composing, password_input) = if self.cursor.y == stable_row {
                        (
                            Some(CursorProperties {
                                position: StableCursorPosition {
                                    y: 0,
                                    ..*self.cursor
                                },
                                dead_key_or_leader: self.term_window.dead_key_status
                                    != DeadKeyStatus::None
                                    || self.term_window.leader_is_active(),
                                cursor_fg: self.cursor_fg,
                                cursor_bg: self.cursor_bg,
                                cursor_border_color: self.cursor_border_color,
                                cursor_is_default_color: self.cursor_is_default_color,
                            }),
                            match (self.pos.is_active, &self.term_window.dead_key_status) {
                                (true, DeadKeyStatus::Composing(composing)) => {
                                    Some(composing.to_string())
                                }
                                _ => None,
                            },
                            if self.term_window.config.detect_password_input {
                                match self.pos.pane.get_metadata() {
                                    Value::Object(obj) => {
                                        match obj.get(&Value::String("password_input".to_string()))
                                        {
                                            Some(Value::Bool(b)) => *b,
                                            _ => false,
                                        }
                                    }
                                    _ => false,
                                }
                            } else {
                                false
                            },
                        )
                    } else {
                        (None, None, false)
                    };

                    let shape_hash = self.term_window.shape_hash_for_line(line);

                    let quad_key = LineQuadCacheKey {
                        pane_id: self.pane_id,
                        password_input,
                        pane_is_active: self.pos.is_active,
                        config_generation: self.term_window.config.generation(),
                        shape_generation: self.term_window.shape_generation,
                        quad_generation: self.term_window.quad_generation,
                        composing: composing.clone(),
                        selection: selrange.clone(),
                        cursor,
                        shape_hash,
                        top_pixel_y: NotNan::new(self.top_pixel_y).unwrap()
                            + line_idx as f32
                                * self.term_window.render_metrics.cell_size.height as f32,
                        left_pixel_x: NotNan::new(self.left_pixel_x).unwrap(),
                        phys_line_idx: line_idx,
                        reverse_video: self.dims.reverse_video,
                    };

                    if let Some(cached_quad) =
                        self.term_window.line_quad_cache.borrow_mut().get(&quad_key)
                    {
                        let expired = cached_quad
                            .expires
                            .map(|i| Instant::now() >= i)
                            .unwrap_or(false);
                        let hover_changed = if cached_quad.invalidate_on_hover_change {
                            !same_hyperlink(
                                cached_quad.current_highlight.as_ref(),
                                self.term_window.current_highlight.as_ref(),
                            )
                        } else {
                            false
                        };
                        if !expired && !hover_changed {
                            cached_quad
                                .layers
                                .apply_to(self.layers)
                                .context("cached_quad.layers.apply_to")?;
                            self.term_window.update_next_frame_time(cached_quad.expires);
                            return Ok(());
                        }
                    }

                    let mut buf = HeapQuadAllocator::default();
                    let next_due = self.term_window.has_animation.borrow_mut().take();

                    let shape_key = LineToEleShapeCacheKey {
                        shape_hash,
                        shape_generation: quad_key.shape_generation,
                        composing: if self.cursor.y == stable_row && self.pos.is_active {
                            if let DeadKeyStatus::Composing(composing) =
                                &self.term_window.dead_key_status
                            {
                                Some((self.cursor.x, composing.to_string()))
                            } else {
                                None
                            }
                        } else {
                            None
                        },
                    };

                    let render_result = self
                        .term_window
                        .render_screen_line(
                            RenderScreenLineParams {
                                top_pixel_y: *quad_key.top_pixel_y,
                                left_pixel_x: self.left_pixel_x,
                                pixel_width: self.dims.cols as f32
                                    * self.term_window.render_metrics.cell_size.width as f32,
                                stable_line_idx: Some(stable_row),
                                line: &line,
                                selection: selrange.clone(),
                                cursor: &self.cursor,
                                palette: &self.palette,
                                dims: &self.dims,
                                config: &self.term_window.config,
                                cursor_border_color: self.cursor_border_color,
                                foreground: self.foreground,
                                is_active: self.pos.is_active,
                                pane: Some(&self.pos.pane),
                                selection_fg: self.selection_fg,
                                selection_bg: self.selection_bg,
                                cursor_fg: self.cursor_fg,
                                cursor_bg: self.cursor_bg,
                                cursor_is_default_color: self.cursor_is_default_color,
                                white_space: self.white_space,
                                filled_box: self.filled_box,
                                window_is_transparent: self.window_is_transparent,
                                default_bg: self.default_bg,
                                font: None,
                                style: None,
                                use_pixel_positioning: self
                                    .term_window
                                    .config
                                    .experimental_pixel_positioning,
                                render_metrics: self.term_window.render_metrics,
                                shape_key: Some(shape_key),
                                password_input,
                            },
                            &mut TripleLayerQuadAllocator::Heap(&mut buf),
                        )
                        .context("render_screen_line")?;

                    let expires = self.term_window.has_animation.borrow().as_ref().cloned();
                    self.term_window.update_next_frame_time(next_due);

                    buf.apply_to(self.layers)
                        .context("HeapQuadAllocator::apply_to")?;

                    let quad_value = LineQuadCacheValue {
                        layers: buf,
                        expires,
                        invalidate_on_hover_change: render_result.invalidate_on_hover_change,
                        current_highlight: if render_result.invalidate_on_hover_change {
                            self.term_window.current_highlight.clone()
                        } else {
                            None
                        },
                    };

                    self.term_window
                        .line_quad_cache
                        .borrow_mut()
                        .put(quad_key, quad_value);

                    Ok(())
                }
            }

            impl<'a, 'b> WithPaneLines for LineRender<'a, 'b> {
                fn with_lines_mut(&mut self, stable_top: StableRowIndex, lines: &mut [&mut Line]) {
                    for (line_idx, line) in lines.iter().enumerate() {
                        if let Err(err) = self.render_line(stable_top, line_idx, line) {
                            self.error.replace(err);
                            return;
                        }
                    }
                }
            }

            pos.pane.with_lines_mut(stable_range.clone(), &mut render);
            if let Some(error) = render.error.take() {
                return Err(error).context("error while calling with_lines_mut");
            }
        }

        /*
        if let Some(zone) = zone {
            // TODO: render a thingy to jump to prior prompt
        }
        */
        metrics::histogram!("paint_pane.lines").record(start.elapsed());
        log::trace!("lines elapsed {:?}", start.elapsed());

        Ok(())
    }

    pub fn build_pane(&mut self, pos: &TerminalPaneLayout) -> anyhow::Result<ComputedElement> {
        let pane_rects = self.pane_pixel_rects(pos);

        let palette = pos.pane.palette();

        // TODO: visual bell background layer
        // TODO: scrollbar

        Ok(ComputedElement {
            item_type: None,
            zindex: 0,
            bounds: pane_rects.background_rect,
            border: PixelDimension::default(),
            border_rect: pane_rects.background_rect,
            border_corners: None,
            colors: ElementColors {
                border: BorderColor::default(),
                bg: if self.window_background.is_empty() {
                    palette
                        .background
                        .to_linear()
                        .mul_alpha(self.config.window_background_opacity)
                        .into()
                } else {
                    InheritableColor::Inherited
                },
                text: InheritableColor::Inherited,
            },
            hover_colors: None,
            padding: pane_rects.background_rect,
            content_rect: pane_rects.content_rect,
            baseline: 1.0,
            content: ComputedElementContent::Children(vec![]),
        })
    }
}
