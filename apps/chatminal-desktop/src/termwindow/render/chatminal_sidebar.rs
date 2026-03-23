use crate::chatminal_sidebar::{SidebarProfile, SidebarSession, SidebarSnapshot};
use crate::termwindow::box_model::{
    BorderColor, BoxDimension, Corners, Element, ElementColors, ElementContent, Float, SizedPoly,
};
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::UIItemType;
use config::Dimension;
use window::color::LinearRgba;

const SIDEBAR_HEADER_HEIGHT_PX: f32 = 34.0;
const SIDEBAR_BODY_BOTTOM_PADDING_PX: f32 = 8.0;
const SIDEBAR_SCROLLBAR_WIDTH_PX: f32 = 6.0;
const SIDEBAR_SCROLLBAR_MIN_THUMB_HEIGHT_PX: f32 = 28.0;

#[derive(Clone)]
enum SidebarTreeRow {
    Error(String),
    EmptyHint(String),
    Profile {
        profile: SidebarProfile,
        is_expanded: bool,
    },
    EmptyNestedHint(String),
    Session(SidebarSession),
}

impl SidebarTreeRow {
    fn estimated_height_px(&self, line_height: f32) -> f32 {
        match self {
            Self::Error(_) | Self::EmptyHint(_) => line_height + 14.0,
            Self::Profile { .. } | Self::Session(_) => line_height + 12.0,
            Self::EmptyNestedHint(_) => line_height + 8.0,
        }
    }
}

fn ordered_sidebar_snapshot(
    term_window: &crate::TermWindow,
    mut snapshot: SidebarSnapshot,
) -> SidebarSnapshot {
    let ordered_session_ids = term_window.ordered_chatminal_session_ids();
    let active_session_id = term_window.active_session_id();

    snapshot.sessions.sort_by_key(|session| {
        ordered_session_ids
            .iter()
            .position(|session_id| session_id == &session.session_id)
            .unwrap_or(usize::MAX)
    });
    for session in &mut snapshot.sessions {
        session.is_active = active_session_id.as_deref() == Some(session.session_id.as_str());
    }
    snapshot.active_session_id = active_session_id;
    snapshot
}

impl crate::TermWindow {
    pub fn paint_chatminal_sidebar(&mut self) -> anyhow::Result<()> {
        if !self.chatminal_sidebar.is_enabled() {
            return Ok(());
        }
        let sidebar_background = self.build_chatminal_sidebar()?;
        let sidebar_header = self.build_chatminal_sidebar_header()?;
        let (sidebar_tree, tree_clip_rect) = self.build_chatminal_sidebar_tree()?;
        let footer_background = self.build_chatminal_terminal_footer_background()?;
        let footer_content = self.build_chatminal_terminal_footer_content()?;

        self.append_and_render_overlay(&sidebar_background)?;
        if let Some(tree) = sidebar_tree.as_ref() {
            self.append_and_render_overlay_clipped(tree, tree_clip_rect)?;
        }
        self.append_and_render_overlay(&sidebar_header)?;
        self.render_chatminal_sidebar_scrollbar()?;
        self.append_and_render_overlay(&footer_background)?;
        self.append_and_render_overlay(&footer_content)
    }

    fn append_and_render_overlay(
        &mut self,
        computed: &crate::termwindow::box_model::ComputedElement,
    ) -> anyhow::Result<()> {
        let mut ui_items = computed.ui_items();
        self.ui_items.append(&mut ui_items);
        let gl_state = self.render_state.as_ref().unwrap();
        self.render_element(computed, gl_state, None)
    }

    fn append_and_render_overlay_clipped(
        &mut self,
        computed: &crate::termwindow::box_model::ComputedElement,
        clip_rect: ::window::RectF,
    ) -> anyhow::Result<()> {
        let mut ui_items: Vec<_> = computed
            .ui_items()
            .into_iter()
            .filter_map(|item| clamp_ui_item_to_rect(item, clip_rect))
            .collect();
        self.ui_items.append(&mut ui_items);
        let gl_state = self.render_state.as_ref().unwrap();
        self.render_element(computed, gl_state, None)
    }

    fn build_chatminal_sidebar(
        &mut self,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let border = self.get_os_border();
        let sidebar_width = self.chatminal_sidebar_width() as f32;
        let sidebar_height =
            self.dimensions
                .pixel_height
                .saturating_sub((border.top + border.bottom).get() as usize) as f32;
        let body_font = self.fonts.default_font()?;

        let panel_bg = LinearRgba::with_components(0.008, 0.008, 0.008, 1.0);
        let root_border = LinearRgba::with_components(0.133, 0.133, 0.133, 1.0);
        let text = LinearRgba::with_components(0.867, 0.867, 0.867, 1.0);
        let root = Element::new(&body_font, ElementContent::Children(vec![]))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .item_type(UIItemType::ChatminalSidebarBackground)
            .colors(ElementColors {
                border: BorderColor::new(root_border),
                bg: panel_bg.into(),
                text: text.into(),
            })
            .border(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(1.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(0.0),
            })
            .min_width(Some(Dimension::Pixels(sidebar_width)))
            .min_height(Some(Dimension::Pixels(sidebar_height)));

        self.compute_element(
            &crate::termwindow::box_model::LayoutContext {
                width: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: sidebar_width,
                    pixel_cell: self.render_metrics.cell_size.width as f32,
                },
                height: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: sidebar_height,
                    pixel_cell: self.render_metrics.cell_size.height as f32,
                },
                bounds: euclid::rect(0.0, border.top.get() as f32, sidebar_width, sidebar_height),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 1,
            },
            &root,
        )
    }

    fn build_chatminal_sidebar_header(
        &mut self,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let border = self.get_os_border();
        let sidebar_width = self.chatminal_sidebar_width() as f32;
        let body_font = self.fonts.default_font()?;
        let title_font = self.fonts.default_font()?;
        let text = LinearRgba::with_components(0.867, 0.867, 0.867, 1.0);
        let muted = LinearRgba::with_components(0.533, 0.533, 0.533, 1.0);
        let hover_bg = LinearRgba::with_components(0.118, 0.118, 0.118, 1.0);

        let header = header_row(&body_font, &title_font, sidebar_width, text, muted, hover_bg);
        self.compute_element(
            &crate::termwindow::box_model::LayoutContext {
                width: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: sidebar_width,
                    pixel_cell: self.render_metrics.cell_size.width as f32,
                },
                height: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: SIDEBAR_HEADER_HEIGHT_PX,
                    pixel_cell: self.render_metrics.cell_size.height as f32,
                },
                bounds: euclid::rect(
                    0.0,
                    border.top.get() as f32,
                    sidebar_width,
                    SIDEBAR_HEADER_HEIGHT_PX,
                ),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 2,
            },
            &header,
        )
    }

    fn build_chatminal_sidebar_tree(
        &mut self,
    ) -> anyhow::Result<(Option<crate::termwindow::box_model::ComputedElement>, ::window::RectF)> {
        let snapshot = ordered_sidebar_snapshot(self, self.chatminal_sidebar.snapshot());
        let border = self.get_os_border();
        let sidebar_width = self.chatminal_sidebar_width() as f32;
        let sidebar_height =
            self.dimensions
                .pixel_height
                .saturating_sub((border.top + border.bottom).get() as usize) as f32;
        let tree_viewport_height =
            (sidebar_height - SIDEBAR_HEADER_HEIGHT_PX - SIDEBAR_BODY_BOTTOM_PADDING_PX).max(0.0);
        let tree_clip_rect = euclid::rect(
            0.0,
            border.top.get() as f32 + SIDEBAR_HEADER_HEIGHT_PX,
            sidebar_width,
            tree_viewport_height,
        );
        if tree_viewport_height <= 1.0 {
            return Ok((None, tree_clip_rect));
        }

        let body_font = self.fonts.default_font()?;
        let status_font = self.fonts.default_font()?;
        let line_height = self.render_metrics.cell_size.height as f32;
        let text = LinearRgba::with_components(0.867, 0.867, 0.867, 1.0);
        let muted = LinearRgba::with_components(0.533, 0.533, 0.533, 1.0);
        let accent = LinearRgba::with_components(0.318, 0.639, 0.318, 1.0);
        let session_active_bg = LinearRgba::with_components(0.122, 0.125, 0.133, 1.0);
        let hover_bg = LinearRgba::with_components(0.118, 0.118, 0.118, 1.0);
        let offline = LinearRgba::with_components(0.533, 0.533, 0.533, 1.0);
        let error_fg = LinearRgba::with_components(0.92, 0.38, 0.32, 1.0);

        let tree_rows = sidebar_tree_rows(self, &snapshot);
        let total_tree_height = total_tree_height(&tree_rows, line_height);
        let max_scroll_offset = (total_tree_height - tree_viewport_height).max(0.0);
        self.chatminal_sidebar.set_scroll_bounds(max_scroll_offset);

        let tree_children = tree_row_elements(
            &tree_rows,
            &body_font,
            &status_font,
            text,
            muted,
            accent,
            session_active_bg,
            hover_bg,
            offline,
            error_fg,
        );
        let tree = Element::new(&body_font, ElementContent::Children(tree_children))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(8.0),
                right: Dimension::Pixels(18.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(8.0),
            })
            .colors(text_colors(text))
            .min_width(Some(Dimension::Pixels(sidebar_width.max(200.0))))
            .min_height(Some(Dimension::Pixels(total_tree_height.max(tree_viewport_height))));
        let mut computed = self.compute_element(
            &crate::termwindow::box_model::LayoutContext {
                width: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: sidebar_width,
                    pixel_cell: self.render_metrics.cell_size.width as f32,
                },
                height: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: total_tree_height.max(tree_viewport_height),
                    pixel_cell: self.render_metrics.cell_size.height as f32,
                },
                bounds: euclid::rect(
                    0.0,
                    border.top.get() as f32 + SIDEBAR_HEADER_HEIGHT_PX,
                    sidebar_width,
                    total_tree_height.max(tree_viewport_height),
                ),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 1,
            },
            &tree,
        )?;
        let scroll_offset_px = self.chatminal_sidebar.scroll_offset_px();
        computed.translate(euclid::vec2(0.0, -scroll_offset_px));
        Ok((Some(computed), tree_clip_rect))
    }

    fn render_chatminal_sidebar_scrollbar(&mut self) -> anyhow::Result<()> {
        if !self.chatminal_sidebar.is_enabled() {
            return Ok(());
        }

        let snapshot = ordered_sidebar_snapshot(self, self.chatminal_sidebar.snapshot());
        let border = self.get_os_border();
        let sidebar_width = self.chatminal_sidebar_width() as f32;
        let sidebar_height =
            self.dimensions
                .pixel_height
                .saturating_sub((border.top + border.bottom).get() as usize) as f32;
        let tree_viewport_height =
            (sidebar_height - SIDEBAR_HEADER_HEIGHT_PX - SIDEBAR_BODY_BOTTOM_PADDING_PX).max(0.0);
        let line_height = self.render_metrics.cell_size.height as f32;
        let tree_rows = sidebar_tree_rows(self, &snapshot);
        let total_tree_height = total_tree_height(&tree_rows, line_height);
        let max_scroll_offset = (total_tree_height - tree_viewport_height).max(0.0);
        if max_scroll_offset <= 0.0 || tree_viewport_height <= 1.0 {
            return Ok(());
        }

        let scroll_offset_px = self.chatminal_sidebar.scroll_offset_px();
        let thumb_height = ((tree_viewport_height / total_tree_height) * tree_viewport_height)
            .clamp(SIDEBAR_SCROLLBAR_MIN_THUMB_HEIGHT_PX, tree_viewport_height);
        let max_thumb_offset = (tree_viewport_height - thumb_height).max(0.0);
        let thumb_offset = if max_scroll_offset <= 0.0 {
            0.0
        } else {
            (scroll_offset_px / max_scroll_offset) * max_thumb_offset
        };
        let rect = euclid::rect(
            (sidebar_width - SIDEBAR_SCROLLBAR_WIDTH_PX - 6.0).max(0.0),
            border.top.get() as f32 + SIDEBAR_HEADER_HEIGHT_PX + thumb_offset,
            SIDEBAR_SCROLLBAR_WIDTH_PX,
            thumb_height,
        );
        let gl_state = self.render_state.as_ref().unwrap();
        let layer = gl_state.layer_for_zindex(3)?;
        let mut layers = layer.quad_allocator();
        self.filled_rectangle(
            &mut layers,
            0,
            rect,
            LinearRgba::with_components(0.38, 0.38, 0.38, 0.88),
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    fn build_chatminal_terminal_chrome(
        &mut self,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let snapshot = ordered_sidebar_snapshot(self, self.chatminal_sidebar.snapshot());
        let border = self.get_os_border();
        let x = self.chatminal_sidebar_width() as f32;
        let y = border.top.get() as f32;
        let width = (self.dimensions.pixel_width as f32 - x).max(0.0);
        let height = self.chatminal_terminal_chrome_height();
        let body_font = self.fonts.default_font()?;
        let bg = LinearRgba::with_components(0.035, 0.035, 0.035, 0.98);
        let divider = LinearRgba::with_components(0.133, 0.133, 0.133, 1.0);
        let text = LinearRgba::with_components(0.867, 0.867, 0.867, 1.0);
        let muted = LinearRgba::with_components(0.533, 0.533, 0.533, 1.0);
        let accent = LinearRgba::with_components(0.318, 0.639, 0.318, 1.0);

        let mut tabs = Vec::new();
        for session in snapshot.sessions.iter().take(2) {
            tabs.push(session_pill(
                &body_font,
                &session.name,
                Some(UIItemType::ChatminalSidebarSession(
                    session.session_id.clone(),
                )),
                session.is_active,
                text,
                muted,
                accent,
            ));
        }
        tabs.push(
            Element::new(&body_font, ElementContent::Text("+".to_string()))
                .display(crate::termwindow::box_model::DisplayType::Inline)
                .float(Float::Right)
                .item_type(UIItemType::ChatminalSidebarCreateSession)
                .padding(BoxDimension {
                    left: Dimension::Pixels(6.0),
                    right: Dimension::Pixels(6.0),
                    top: Dimension::Pixels(1.0),
                    bottom: Dimension::Pixels(1.0),
                })
                .margin(BoxDimension {
                    left: Dimension::Pixels(4.0),
                    right: Dimension::Pixels(0.0),
                    top: Dimension::Pixels(0.0),
                    bottom: Dimension::Pixels(0.0),
                })
                .border(BoxDimension::new(Dimension::Pixels(1.0)))
                .border_corners(Some(rounded_corners(7.0)))
                .colors(ElementColors {
                    border: BorderColor::new(divider),
                    bg: LinearRgba::with_components(0.060, 0.068, 0.082, 1.0).into(),
                    text: text.into(),
                })
                .hover_colors(Some(ElementColors {
                    border: BorderColor::new(accent),
                    bg: LinearRgba::with_components(0.075, 0.085, 0.102, 1.0).into(),
                    text: text.into(),
                })),
        );

        let root = Element::new(&body_font, ElementContent::Children(tabs))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(8.0),
                right: Dimension::Pixels(8.0),
                top: Dimension::Pixels(2.0),
                bottom: Dimension::Pixels(2.0),
            })
            .border(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(0.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(1.0),
            })
            .colors(ElementColors {
                border: BorderColor::new(divider),
                bg: bg.into(),
                text: text.into(),
            })
            .min_width(Some(Dimension::Pixels(width)))
            .min_height(Some(Dimension::Pixels(height)));

        self.compute_element(
            &crate::termwindow::box_model::LayoutContext {
                width: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: width,
                    pixel_cell: self.render_metrics.cell_size.width as f32,
                },
                height: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: height,
                    pixel_cell: self.render_metrics.cell_size.height as f32,
                },
                bounds: euclid::rect(x, y, width, height),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 2,
            },
            &root,
        )
    }

    fn build_chatminal_terminal_footer_background(
        &mut self,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let border = self.get_os_border();
        let x = self.chatminal_sidebar_width() as f32;
        let width = (self.dimensions.pixel_width as f32 - x).max(0.0);
        let height = self.chatminal_terminal_footer_height();
        let y =
            (self.dimensions.pixel_height as f32 - border.bottom.get() as f32 - height).max(0.0);
        let body_font = self.fonts.title_font()?;
        let bg = LinearRgba::with_components(0.0, 0.0, 0.0, 1.0);
        let divider = LinearRgba::with_components(0.133, 0.133, 0.133, 1.0);

        let root = Element::new(&body_font, ElementContent::Children(vec![]))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(0.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(0.0),
            })
            .border(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(0.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(0.0),
            })
            .colors(ElementColors {
                border: BorderColor::new(divider),
                bg: bg.into(),
                text: LinearRgba::TRANSPARENT.into(),
            })
            .min_width(Some(Dimension::Pixels(width)))
            .min_height(Some(Dimension::Pixels(height)));

        self.compute_element(
            &crate::termwindow::box_model::LayoutContext {
                width: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: width,
                    pixel_cell: self.render_metrics.cell_size.width as f32,
                },
                height: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: height,
                    pixel_cell: self.render_metrics.cell_size.height as f32,
                },
                bounds: euclid::rect(x, y, width, height),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 2,
            },
            &root,
        )
    }

    fn build_chatminal_terminal_footer_content(
        &mut self,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let snapshot = ordered_sidebar_snapshot(self, self.chatminal_sidebar.snapshot());
        let border = self.get_os_border();
        let sidebar_width = self.chatminal_sidebar_width() as f32;
        let x = 0.0;
        let width = self.dimensions.pixel_width as f32;
        let height = self.chatminal_terminal_footer_height();
        let y =
            (self.dimensions.pixel_height as f32 - border.bottom.get() as f32 - height).max(0.0);
        let body_font = self.fonts.title_font()?;
        let label = LinearRgba::with_components(0.35, 0.35, 0.35, 1.0);
        let value = LinearRgba::with_components(0.65, 0.65, 0.65, 1.0);
        let sep = LinearRgba::with_components(0.25, 0.25, 0.25, 1.0);

        let active_profile = snapshot
            .profiles
            .iter()
            .find(|p| p.is_active)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Profile".to_string());
        let active_session = snapshot
            .sessions
            .iter()
            .find(|s| s.is_active)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Session".to_string());

        let metrics = self.system_metrics.snapshot();

        let items: Vec<(&str, String)> = vec![
            (
                "Session: ",
                format!("{} ({})", active_session, active_profile),
            ),
            ("CPU: ", metrics.cpu_display()),
            ("RAM: ", metrics.ram_display()),
            ("Latency: ", metrics.latency_display()),
        ];
        let mut footer_parts = Vec::new();
        for (i, (lbl, val)) in items.iter().rev().enumerate() {
            footer_parts.push(
                Element::new(&body_font, ElementContent::Text(val.clone()))
                    .display(crate::termwindow::box_model::DisplayType::Inline)
                    .float(Float::Right)
                    .colors(text_colors(value)),
            );
            footer_parts.push(
                Element::new(&body_font, ElementContent::Text(lbl.to_string()))
                    .display(crate::termwindow::box_model::DisplayType::Inline)
                    .float(Float::Right)
                    .colors(text_colors(label)),
            );
            if i + 1 < items.len() {
                footer_parts.push(
                    Element::new(&body_font, ElementContent::Text("  |  ".to_string()))
                        .display(crate::termwindow::box_model::DisplayType::Inline)
                        .float(Float::Right)
                        .colors(text_colors(sep)),
                );
            }
        }

        let root = Element::new(&body_font, ElementContent::Children(footer_parts))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(sidebar_width + 12.0),
                right: Dimension::Pixels(12.0),
                top: Dimension::Pixels(8.0),
                bottom: Dimension::Pixels(8.0),
            })
            .border(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(0.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(0.0),
            })
            .colors(ElementColors {
                border: BorderColor::default(),
                bg: LinearRgba::TRANSPARENT.into(),
                text: value.into(),
            })
            .min_width(Some(Dimension::Pixels(width)))
            .min_height(Some(Dimension::Pixels(height)));

        self.compute_element(
            &crate::termwindow::box_model::LayoutContext {
                width: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: width,
                    pixel_cell: self.render_metrics.cell_size.width as f32,
                },
                height: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: height,
                    pixel_cell: self.render_metrics.cell_size.height as f32,
                },
                bounds: euclid::rect(x, y, width, height),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 2,
            },
            &root,
        )
    }
}

fn total_tree_height(rows: &[SidebarTreeRow], line_height: f32) -> f32 {
    rows.iter()
        .map(|row| row.estimated_height_px(line_height))
        .sum()
}

fn header_row(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    title_font: &std::rc::Rc<engine_font::LoadedFont>,
    body_width: f32,
    text: LinearRgba,
    muted: LinearRgba,
    hover_bg: LinearRgba,
) -> Element {
    let actions = Element::new(
        body_font,
        ElementContent::Children(vec![
            mini_button(body_font, "⚙", None, muted, hover_bg),
            mini_button(
                body_font,
                "+",
                Some(UIItemType::ChatminalSidebarCreateProfile),
                muted,
                hover_bg,
            ),
        ]),
    )
    .display(crate::termwindow::box_model::DisplayType::Inline)
    .float(Float::Right)
    .colors(text_colors(muted));

    let title = Element::new(title_font, ElementContent::Text("Profiles".to_string()))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .colors(text_colors(text));

    let children = vec![title, actions];

    Element::new(body_font, ElementContent::Children(children))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .padding(BoxDimension {
            left: Dimension::Pixels(8.0),
            right: Dimension::Pixels(8.0),
            top: Dimension::Pixels(12.0),
            bottom: Dimension::Pixels(1.0),
        })
        .min_width(Some(Dimension::Pixels((body_width - 16.0).max(120.0))))
        .min_height(Some(Dimension::Pixels(34.0)))
        .colors(text_colors(text))
}

fn sidebar_tree_rows(
    term_window: &crate::TermWindow,
    snapshot: &SidebarSnapshot,
) -> Vec<SidebarTreeRow> {
    if let Some(error) = &snapshot.error {
        return vec![SidebarTreeRow::Error(error.clone())];
    }
    if snapshot.profiles.is_empty() {
        return vec![SidebarTreeRow::EmptyHint("No profiles yet".to_string())];
    }

    let mut rows = Vec::new();
    for profile in &snapshot.profiles {
        let is_expanded = term_window
            .chatminal_sidebar
            .is_profile_expanded(&profile.profile_id);
        rows.push(SidebarTreeRow::Profile {
            profile: profile.clone(),
            is_expanded,
        });

        if is_expanded {
            let profile_sessions: Vec<&SidebarSession> = snapshot
                .sessions
                .iter()
                .filter(|session| session.profile_id == profile.profile_id)
                .collect();
            if profile_sessions.is_empty() {
                rows.push(SidebarTreeRow::EmptyNestedHint("No sessions yet".to_string()));
            } else {
                for session in profile_sessions {
                    rows.push(SidebarTreeRow::Session(session.clone()));
                }
            }
        }
    }
    rows
}

#[allow(clippy::too_many_arguments)]
fn tree_row_elements(
    rows: &[SidebarTreeRow],
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    status_font: &std::rc::Rc<engine_font::LoadedFont>,
    text: LinearRgba,
    muted: LinearRgba,
    accent: LinearRgba,
    session_active_bg: LinearRgba,
    hover_bg: LinearRgba,
    offline: LinearRgba,
    error_fg: LinearRgba,
) -> Vec<Element> {
    rows.iter()
        .map(|row| match row {
            SidebarTreeRow::Error(message) => Element::new(
                body_font,
                ElementContent::Text(message.clone()),
            )
            .display(crate::termwindow::box_model::DisplayType::Block)
            .margin(block_margin(14.0, 0.0))
            .colors(text_colors(error_fg)),
            SidebarTreeRow::EmptyHint(label) => empty_hint(body_font, label, muted),
            SidebarTreeRow::Profile {
                profile,
                is_expanded,
            } => profile_row(body_font, profile, *is_expanded, text, muted, hover_bg),
            SidebarTreeRow::EmptyNestedHint(label) => empty_nested_hint(body_font, label, muted),
            SidebarTreeRow::Session(session) => session_card(
                body_font,
                status_font,
                session,
                text,
                muted,
                accent,
                session_active_bg,
                hover_bg,
                offline,
            ),
        })
        .collect()
}

fn clamp_ui_item_to_rect(item: crate::termwindow::UIItem, rect: ::window::RectF) -> Option<crate::termwindow::UIItem> {
    let item_left = item.x as f32;
    let item_top = item.y as f32;
    let item_right = item_left + item.width as f32;
    let item_bottom = item_top + item.height as f32;

    let clipped_left = item_left.max(rect.min_x());
    let clipped_top = item_top.max(rect.min_y());
    let clipped_right = item_right.min(rect.max_x());
    let clipped_bottom = item_bottom.min(rect.max_y());

    if clipped_right <= clipped_left || clipped_bottom <= clipped_top {
        return None;
    }

    Some(crate::termwindow::UIItem {
        x: clipped_left.max(0.0) as usize,
        y: clipped_top.max(0.0) as usize,
        width: (clipped_right - clipped_left).max(0.0) as usize,
        height: (clipped_bottom - clipped_top).max(0.0) as usize,
        item_type: item.item_type,
    })
}

fn profile_row(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    profile: &SidebarProfile,
    is_expanded: bool,
    text: LinearRgba,
    muted: LinearRgba,
    hover_bg: LinearRgba,
) -> Element {
    let marker = if is_expanded { "v" } else { ">" };
    let label = format!("{marker} {}", profile.name);
    let fg = if profile.is_active { text } else { muted };

    Element::new(body_font, ElementContent::Text(label))
    .display(crate::termwindow::box_model::DisplayType::Block)
    .item_type(UIItemType::ChatminalSidebarProfile(
        profile.profile_id.clone(),
    ))
    .padding(BoxDimension {
        left: Dimension::Pixels(8.0),
        right: Dimension::Pixels(8.0),
        top: Dimension::Pixels(6.0),
        bottom: Dimension::Pixels(6.0),
    })
    .margin(block_margin(0.0, 0.0))
    .colors(filled_colors(LinearRgba::TRANSPARENT, fg))
    .border_corners(Some(rounded_corners(7.0)))
    .hover_colors(Some(filled_colors(hover_bg, text)))
}

#[allow(clippy::too_many_arguments)]
fn session_card(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    status_font: &std::rc::Rc<engine_font::LoadedFont>,
    session: &SidebarSession,
    text: LinearRgba,
    muted: LinearRgba,
    accent: LinearRgba,
    session_active_bg: LinearRgba,
    hover_bg: LinearRgba,
    offline: LinearRgba,
) -> Element {
    let is_running = session.status == "running";
    let (status_text, dot_color) = if is_running {
        ("Online", accent)
    } else {
        ("Offline", offline)
    };
    let row_bg = if session.is_active {
        session_active_bg
    } else {
        LinearRgba::with_components(0.0, 0.0, 0.0, 0.0)
    };
    let name_color = if session.is_active { text } else { muted };
    let row = vec![
        Element::new(body_font, ElementContent::Text("● ".to_string()))
            .display(crate::termwindow::box_model::DisplayType::Inline)
            .colors(text_colors(dot_color)),
        Element::new(body_font, ElementContent::Text(session.name.clone()))
            .display(crate::termwindow::box_model::DisplayType::Inline)
            .colors(text_colors(name_color)),
        Element::new(
            status_font,
            ElementContent::Text(format!(" ({})", status_text)),
        )
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .colors(text_colors(if is_running {
            LinearRgba::with_components(0.318, 0.639, 0.318, 0.8)
        } else {
            offline
        })),
    ];

    Element::new(body_font, ElementContent::Children(row))
    .display(crate::termwindow::box_model::DisplayType::Block)
    .item_type(UIItemType::ChatminalSidebarSession(
        session.session_id.clone(),
    ))
    .padding(BoxDimension {
        left: Dimension::Pixels(8.0),
        right: Dimension::Pixels(8.0),
        top: Dimension::Pixels(5.0),
        bottom: Dimension::Pixels(5.0),
    })
    .margin(BoxDimension {
        left: Dimension::Pixels(16.0),
        right: Dimension::Pixels(0.0),
        top: Dimension::Pixels(2.0),
        bottom: Dimension::Pixels(0.0),
    })
    .border_corners(Some(rounded_corners(6.0)))
    .colors(ElementColors {
        border: BorderColor::default(),
        bg: row_bg.into(),
        text: name_color.into(),
    })
    .hover_colors(Some(filled_colors(hover_bg, text)))
}

#[allow(dead_code)]
fn mini_button(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    item_type: Option<UIItemType>,
    text: LinearRgba,
    hover_bg: LinearRgba,
) -> Element {
    let mut element = Element::new(body_font, ElementContent::Text(label.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .padding(BoxDimension {
            left: Dimension::Pixels(5.0),
            right: Dimension::Pixels(5.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .margin(BoxDimension {
            left: Dimension::Pixels(4.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(2.0),
        })
        .border(BoxDimension::new(Dimension::Pixels(1.0)))
        .border_corners(Some(rounded_corners(4.0)))
        .colors(ElementColors {
            border: BorderColor::new(LinearRgba::with_components(0.0, 0.0, 0.0, 0.0)),
            bg: LinearRgba::with_components(0.0, 0.0, 0.0, 0.0).into(),
            text: text.into(),
        })
        .hover_colors(Some(ElementColors {
            border: BorderColor::new(hover_bg),
            bg: hover_bg.into(),
            text: text.into(),
        }));
    if let Some(item_type) = item_type {
        element = element.item_type(item_type);
    }
    element
}

#[allow(dead_code)]
fn session_pill(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    item_type: Option<UIItemType>,
    is_active: bool,
    text: LinearRgba,
    muted: LinearRgba,
    accent: LinearRgba,
) -> Element {
    let bg = if is_active {
        LinearRgba::with_components(0.09, 0.11, 0.12, 1.0)
    } else {
        LinearRgba::with_components(0.055, 0.061, 0.072, 1.0)
    };
    let border = if is_active {
        BorderColor {
            left: bg,
            top: bg,
            right: bg,
            bottom: accent,
        }
    } else {
        BorderColor::new(bg)
    };

    let mut element = Element::new(body_font, ElementContent::Text(format!("[{}]", label)))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .padding(BoxDimension {
            left: Dimension::Pixels(6.0),
            right: Dimension::Pixels(6.0),
            top: Dimension::Pixels(1.0),
            bottom: Dimension::Pixels(1.0),
        })
        .margin(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(4.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .border(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(2.0),
        })
        .colors(ElementColors {
            border,
            bg: bg.into(),
            text: if is_active { text } else { muted }.into(),
        })
        .border_corners(Some(Corners {
            top_left: SizedPoly {
                width: Dimension::Pixels(7.0),
                height: Dimension::Pixels(7.0),
                poly: TOP_LEFT_ROUNDED_CORNER,
            },
            top_right: SizedPoly {
                width: Dimension::Pixels(7.0),
                height: Dimension::Pixels(7.0),
                poly: TOP_RIGHT_ROUNDED_CORNER,
            },
            bottom_left: SizedPoly::none(),
            bottom_right: SizedPoly::none(),
        }));
    if let Some(item_type) = item_type {
        element = element.item_type(item_type);
    }
    element
}

fn empty_hint(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    muted_fg: LinearRgba,
) -> Element {
    Element::new(body_font, ElementContent::Text(label.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .margin(block_margin(12.0, 0.0))
        .colors(text_colors(muted_fg))
}

fn empty_nested_hint(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    muted_fg: LinearRgba,
) -> Element {
    Element::new(body_font, ElementContent::Text(label.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .margin(BoxDimension {
            left: Dimension::Pixels(14.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(2.0),
            bottom: Dimension::Pixels(6.0),
        })
        .colors(text_colors(muted_fg))
}

fn block_margin(top: f32, bottom: f32) -> BoxDimension {
    BoxDimension {
        left: Dimension::Pixels(0.0),
        right: Dimension::Pixels(0.0),
        top: Dimension::Pixels(top),
        bottom: Dimension::Pixels(bottom),
    }
}

fn text_colors(text: LinearRgba) -> ElementColors {
    ElementColors {
        border: BorderColor::default(),
        bg: LinearRgba::TRANSPARENT.into(),
        text: text.into(),
    }
}

fn filled_colors(bg: LinearRgba, text: LinearRgba) -> ElementColors {
    ElementColors {
        border: BorderColor::default(),
        bg: bg.into(),
        text: text.into(),
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
