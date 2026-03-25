use crate::chatminal_sidebar::{
    SidebarProfile, SidebarSession, SidebarSessionDragState, SidebarSessionDropTarget,
    SidebarSnapshot,
};
use crate::customglyph::{BlockAlpha, BlockCoord, Poly, PolyCommand, PolyStyle};
use crate::lucide_icons::LucideIcon;
use crate::termwindow::box_model::{
    BorderColor, BoxDimension, Corners, Element, ElementCell, ElementColors, ElementContent, Float,
    SizedPoly, VerticalAlign,
};
use crate::termwindow::render::corners::{
    BOTTOM_LEFT_ROUNDED_CORNER, BOTTOM_RIGHT_ROUNDED_CORNER, TOP_LEFT_ROUNDED_CORNER,
    TOP_RIGHT_ROUNDED_CORNER,
};
use crate::termwindow::UIItemType;
use config::{Dimension, FontAttributes, FontWeight, TextStyle};
use window::color::LinearRgba;

const SIDEBAR_HEADER_HEIGHT_PX: f32 = 32.0;
const SIDEBAR_HEADER_TREE_GAP_PX: f32 = 24.0;
const SIDEBAR_BODY_BOTTOM_PADDING_PX: f32 = 8.0;
const SIDEBAR_SCROLLBAR_WIDTH_PX: f32 = 4.0;
const SIDEBAR_SCROLLBAR_MIN_THUMB_HEIGHT_PX: f32 = 28.0;
const SIDEBAR_COMPACT_TITLE_HIDE_WIDTH_PX: f32 = 120.0;
const SIDEBAR_TOOLTIP_GAP_PX: f32 = 6.0;
const SIDEBAR_CONTEXT_MENU_WIDTH_PX: f32 = 228.0;
const SIDEBAR_CONTEXT_MENU_ITEM_CONTENT_WIDTH_PX: f32 = 204.0;
const SIDEBAR_JOIN_CONNECTOR_WIDTH_PX: f32 = 14.0;
const SIDEBAR_JOIN_CONNECTOR_SLOT_WIDTH_PX: f32 = 18.0;
const SIDEBAR_JOIN_CONNECTOR_HEIGHT_OVERLAP_PX: f32 = 10.0;

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
            Self::Error(_) | Self::EmptyHint(_) => line_height + 10.0,
            Self::Profile { .. } | Self::Session(_) => line_height + 8.0,
            Self::EmptyNestedHint(_) => line_height + 6.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JoinedSessionMarker {
    Start,
    Middle,
    End,
}

impl JoinedSessionMarker {
    fn poly(self) -> &'static [Poly] {
        match self {
            Self::Start => SIDEBAR_JOIN_CONNECTOR_START,
            Self::Middle => SIDEBAR_JOIN_CONNECTOR_MIDDLE,
            Self::End => SIDEBAR_JOIN_CONNECTOR_END,
        }
    }
}

const SIDEBAR_JOIN_CONNECTOR_START: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::Frac(1, 2), BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

const SIDEBAR_JOIN_CONNECTOR_MIDDLE: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::Frac(1, 2), BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

const SIDEBAR_JOIN_CONNECTOR_END: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

fn ordered_sidebar_snapshot(
    term_window: &crate::TermWindow,
    mut snapshot: SidebarSnapshot,
) -> SidebarSnapshot {
    let active_session_id = term_window.active_session_id();

    for session in &mut snapshot.sessions {
        session.is_active = active_session_id.as_deref() == Some(session.session_id.as_str());
    }
    snapshot.active_session_id = active_session_id;
    snapshot
}

impl crate::TermWindow {
    fn sidebar_text_font(&self) -> anyhow::Result<std::rc::Rc<engine_font::LoadedFont>> {
        let style = sidebar_text_style();
        self.fonts.resolve_font(&style).or_else(|err| {
            log::warn!(
                "sidebar font fallback to default_font: {:#}; requested={:?}",
                err,
                style.font
            );
            self.fonts.default_font()
        })
    }

    fn sidebar_header_font(&self) -> anyhow::Result<std::rc::Rc<engine_font::LoadedFont>> {
        let style = sidebar_header_text_style();
        self.fonts.resolve_font(&style).or_else(|err| {
            log::warn!(
                "sidebar header font fallback to sidebar_text_font: {:#}; requested={:?}",
                err,
                style.font
            );
            self.sidebar_text_font()
        })
    }

    pub fn paint_chatminal_sidebar(&mut self) -> anyhow::Result<()> {
        if !self.chatminal_sidebar.is_enabled() {
            return Ok(());
        }
        let bounds = self.shell_bounds();
        let sidebar_background = self.build_chatminal_sidebar(&bounds)?;
        let sidebar_header = self.build_chatminal_sidebar_header(&bounds)?;
        let (sidebar_tree, tree_clip_rect) = self.build_chatminal_sidebar_tree(&bounds)?;
        let footer_background = self.build_chatminal_terminal_footer_background(&bounds)?;
        let footer_content = self.build_chatminal_terminal_footer_content(&bounds)?;
        let sidebar_tooltip = self.build_chatminal_sidebar_header_tooltip()?;
        let sidebar_context_menu = self.build_chatminal_sidebar_session_context_menu(&bounds)?;

        self.append_and_render_overlay(&sidebar_background)?;
        if let Some(tree) = sidebar_tree.as_ref() {
            self.append_and_render_overlay_clipped(tree, tree_clip_rect)?;
        }
        self.append_and_render_overlay(&sidebar_header)?;
        if let Some(tooltip) = sidebar_tooltip.as_ref() {
            self.append_and_render_overlay(tooltip)?;
        }
        if let Some(context_menu) = sidebar_context_menu.as_ref() {
            self.append_and_render_overlay(context_menu)?;
        }
        self.render_chatminal_sidebar_scrollbar(&bounds)?;
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
        self.render_element_clipped(computed, gl_state, None, clip_rect)
    }

    fn build_chatminal_sidebar(
        &mut self,
        sb: &crate::shell_bounds::ShellBounds,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let sidebar_width = sb.sidebar_width;
        let sidebar_height = sb.sidebar_height;
        let body_font = self.sidebar_text_font()?;

        let panel_bg = LinearRgba::with_components(0.007, 0.007, 0.007, 1.0);
        let root_border = LinearRgba::with_components(0.053, 0.053, 0.053, 1.0);
        let text = LinearRgba::with_components(0.800, 0.800, 0.800, 1.0);
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
                bounds: euclid::rect(sb.sidebar_x, sb.sidebar_y, sidebar_width, sidebar_height),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 1,
            },
            &root,
        )
    }

    fn build_chatminal_sidebar_header(
        &mut self,
        sb: &crate::shell_bounds::ShellBounds,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let sidebar_width = sb.sidebar_width;
        let body_font = self.sidebar_text_font()?;
        let title_font = self.sidebar_header_font()?;
        let text = LinearRgba::with_components(0.800, 0.800, 0.800, 1.0);
        let muted = LinearRgba::with_components(0.616, 0.616, 0.616, 1.0);
        let hover_bg = LinearRgba::with_components(0.353, 0.365, 0.369, 0.314);

        let header = self.build_chatminal_sidebar_header_row(
            &body_font,
            &title_font,
            sidebar_width,
            text,
            muted,
            hover_bg,
        )?;
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
                    sb.sidebar_x,
                    sb.sidebar_y,
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
        sb: &crate::shell_bounds::ShellBounds,
    ) -> anyhow::Result<(
        Option<crate::termwindow::box_model::ComputedElement>,
        ::window::RectF,
    )> {
        let snapshot = ordered_sidebar_snapshot(self, self.chatminal_sidebar.snapshot());
        let sidebar_width = sb.sidebar_width;
        let sidebar_height = sb.sidebar_height;
        let tree_viewport_height = (sidebar_height
            - SIDEBAR_HEADER_HEIGHT_PX
            - SIDEBAR_HEADER_TREE_GAP_PX
            - SIDEBAR_BODY_BOTTOM_PADDING_PX)
            .max(0.0);
        let tree_clip_rect = euclid::rect(
            sb.sidebar_x,
            sb.sidebar_y + SIDEBAR_HEADER_HEIGHT_PX + SIDEBAR_HEADER_TREE_GAP_PX,
            sidebar_width,
            tree_viewport_height,
        );
        if tree_viewport_height <= 1.0 {
            return Ok((None, tree_clip_rect));
        }

        let body_font = self.sidebar_text_font()?;
        let status_font = self.sidebar_text_font()?;
        let line_height = self.render_metrics.cell_size.height as f32;
        let text = LinearRgba::with_components(0.800, 0.800, 0.800, 1.0);
        let muted = LinearRgba::with_components(0.549, 0.549, 0.549, 1.0);
        let accent = LinearRgba::with_components(0.212, 0.580, 0.196, 1.0);
        let session_active_bg = LinearRgba::with_components(0.016, 0.224, 0.369, 1.0);
        let session_selected_bg = LinearRgba::with_components(0.071, 0.102, 0.141, 1.0);
        let hover_bg = LinearRgba::with_components(0.165, 0.176, 0.180, 1.0);
        let offline = LinearRgba::with_components(0.549, 0.549, 0.549, 1.0);
        let error_fg = LinearRgba::with_components(0.973, 0.502, 0.439, 1.0);

        let tree_rows = sidebar_tree_rows(self, &snapshot);
        let total_tree_height = total_tree_height(&tree_rows, line_height);
        let max_scroll_offset = (total_tree_height - tree_viewport_height).max(0.0);
        self.chatminal_sidebar.set_scroll_bounds(max_scroll_offset);

        let tree_children = self.build_chatminal_sidebar_tree_row_elements(
            &tree_rows,
            &body_font,
            &status_font,
            text,
            muted,
            accent,
            session_active_bg,
            session_selected_bg,
            hover_bg,
            offline,
            error_fg,
        )?;
        let tree = Element::new(&body_font, ElementContent::Children(tree_children))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(6.0),
                right: Dimension::Pixels(14.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(8.0),
            })
            .colors(text_colors(text))
            .min_width(Some(Dimension::Pixels(sidebar_width.max(200.0))))
            .min_height(Some(Dimension::Pixels(
                total_tree_height.max(tree_viewport_height),
            )));
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
                    sb.sidebar_x,
                    sb.sidebar_y + SIDEBAR_HEADER_HEIGHT_PX + SIDEBAR_HEADER_TREE_GAP_PX,
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

    fn render_chatminal_sidebar_scrollbar(
        &mut self,
        sb: &crate::shell_bounds::ShellBounds,
    ) -> anyhow::Result<()> {
        if !self.chatminal_sidebar.is_enabled() {
            return Ok(());
        }

        let snapshot = ordered_sidebar_snapshot(self, self.chatminal_sidebar.snapshot());
        let sidebar_width = sb.sidebar_width;
        let sidebar_height = sb.sidebar_height;
        let tree_viewport_height = (sidebar_height
            - SIDEBAR_HEADER_HEIGHT_PX
            - SIDEBAR_HEADER_TREE_GAP_PX
            - SIDEBAR_BODY_BOTTOM_PADDING_PX)
            .max(0.0);
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
            (sidebar_width - SIDEBAR_SCROLLBAR_WIDTH_PX - 4.0).max(0.0),
            sb.sidebar_y + SIDEBAR_HEADER_HEIGHT_PX + SIDEBAR_HEADER_TREE_GAP_PX + thumb_offset,
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
            LinearRgba::with_components(0.430, 0.430, 0.430, 0.650),
        )?;
        Ok(())
    }

    fn sidebar_icon_element(&mut self, icon: LucideIcon, size_px: u32) -> anyhow::Result<Element> {
        let sprite = {
            let gl_state = self.render_state.as_ref().unwrap();
            let mut glyph_cache = gl_state.glyph_cache.borrow_mut();
            let (rgba, width, height) = crate::lucide_icons::rasterize_icon_mask(icon, size_px)?;
            glyph_cache.cached_named_sprite(
                &icon.cache_key(size_px),
                width,
                height,
                &rgba,
            )?
        };
        let body_font = self.sidebar_text_font()?;
        Ok(Element::new(
            &body_font,
            ElementContent::Cells(vec![ElementCell::Sprite(sprite)]),
        ))
    }

    fn sidebar_icon_size_px(&self) -> u32 {
        let scale = (self.dimensions.dpi as f32 / 96.0).max(1.0);
        (16.0 * scale).round().clamp(16.0, 32.0) as u32
    }

    fn sidebar_twistie_icon_size_px(&self) -> u32 {
        let scale = (self.dimensions.dpi as f32 / 96.0).max(1.0);
        (12.0 * scale).round().clamp(10.0, 20.0) as u32
    }

    fn build_chatminal_sidebar_header_row(
        &mut self,
        body_font: &std::rc::Rc<engine_font::LoadedFont>,
        title_font: &std::rc::Rc<engine_font::LoadedFont>,
        body_width: f32,
        text: LinearRgba,
        muted: LinearRgba,
        hover_bg: LinearRgba,
    ) -> anyhow::Result<Element> {
        let actions = Element::new(
            body_font,
            ElementContent::Children(vec![
                icon_button(
                    body_font,
                    self.sidebar_icon_element(LucideIcon::Settings, self.sidebar_icon_size_px())?,
                    Some(UIItemType::ChatminalSidebarSettings),
                    muted,
                    hover_bg,
                ),
                icon_button(
                    body_font,
                    self.sidebar_icon_element(
                        LucideIcon::SquareTerminal,
                        self.sidebar_icon_size_px(),
                    )?,
                    Some(UIItemType::ChatminalSidebarCreateSession),
                    muted,
                    hover_bg,
                ),
                icon_button(
                    body_font,
                    self.sidebar_icon_element(LucideIcon::Plus, self.sidebar_icon_size_px())?,
                    Some(UIItemType::ChatminalSidebarCreateProfile),
                    muted,
                    hover_bg,
                ),
            ]),
        )
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .float(Float::Right)
        .colors(text_colors(muted));

        let mut children = Vec::new();
        if body_width >= SIDEBAR_COMPACT_TITLE_HIDE_WIDTH_PX {
            children.push(
                Element::new(title_font, ElementContent::Text("Profiles".to_string()))
                    .display(crate::termwindow::box_model::DisplayType::Inline)
                    .colors(text_colors(text)),
            );
        }
        children.push(actions);

        Ok(
            Element::new(body_font, ElementContent::Children(children))
                .display(crate::termwindow::box_model::DisplayType::Block)
                .padding(BoxDimension {
                    left: Dimension::Pixels(8.0),
                    right: Dimension::Pixels(8.0),
                    top: Dimension::Pixels(8.0),
                    bottom: Dimension::Pixels(0.0),
                })
                .min_width(Some(Dimension::Pixels((body_width - 16.0).max(120.0))))
                .min_height(Some(Dimension::Pixels(32.0)))
                .colors(text_colors(text)),
        )
    }

    fn build_chatminal_sidebar_header_tooltip(
        &mut self,
    ) -> anyhow::Result<Option<crate::termwindow::box_model::ComputedElement>> {
        let Some(item) = self.last_ui_item.as_ref() else {
            return Ok(None);
        };
        let label = match &item.item_type {
            UIItemType::ChatminalSidebarSettings => "Settings",
            UIItemType::ChatminalSidebarCreateSession => "New session",
            UIItemType::ChatminalSidebarCreateProfile => "New profile",
            _ => return Ok(None),
        };
        let Some(event) = self.current_mouse_event.as_ref() else {
            return Ok(None);
        };
        if !item.hit_test(event.coords.x, event.coords.y) {
            return Ok(None);
        }

        let body_font = self.sidebar_text_font()?;
        let fg = LinearRgba::with_components(0.92, 0.92, 0.92, 1.0);
        let bg = LinearRgba::with_components(0.10, 0.10, 0.10, 0.98);
        let border = LinearRgba::with_components(0.24, 0.24, 0.24, 1.0);
        let tooltip = Element::new(&body_font, ElementContent::Text(label.to_string()))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(8.0),
                right: Dimension::Pixels(8.0),
                top: Dimension::Pixels(5.0),
                bottom: Dimension::Pixels(5.0),
            })
            .border(BoxDimension::new(Dimension::Pixels(1.0)))
            .colors(ElementColors {
                border: BorderColor::new(border),
                bg: bg.into(),
                text: fg.into(),
            });

        let width_context = config::DimensionContext {
            dpi: self.dimensions.dpi as f32,
            pixel_max: self.dimensions.pixel_width as f32,
            pixel_cell: self.render_metrics.cell_size.width as f32,
        };
        let height_context = config::DimensionContext {
            dpi: self.dimensions.dpi as f32,
            pixel_max: self.dimensions.pixel_height as f32,
            pixel_cell: self.render_metrics.cell_size.height as f32,
        };
        let estimated_width = (label.len() as f32 * self.render_metrics.cell_size.width as f32)
            + 18.0;
        let tooltip_x = ((item.x as f32) + (item.width as f32 - estimated_width.max(60.0)) * 0.5)
            .clamp(4.0, (self.dimensions.pixel_width as f32 - estimated_width - 4.0).max(4.0));
        let tooltip_y = (item.y + item.height) as f32 + SIDEBAR_TOOLTIP_GAP_PX;

        Ok(Some(self.compute_element(
            &crate::termwindow::box_model::LayoutContext {
                width: width_context,
                height: height_context,
                bounds: euclid::rect(tooltip_x, tooltip_y, estimated_width.max(60.0), 28.0),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 6,
            },
            &tooltip,
        )?))
    }

    fn build_chatminal_sidebar_session_context_menu(
        &mut self,
        sb: &crate::shell_bounds::ShellBounds,
    ) -> anyhow::Result<Option<crate::termwindow::box_model::ComputedElement>> {
        let Some(menu) = self.chatminal_sidebar.session_context_menu() else {
            return Ok(None);
        };
        let snapshot = self.chatminal_sidebar.snapshot();
        let Some(session) = snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == menu.session_id)
        else {
            return Ok(None);
        };
        let selected_session_ids = self.chatminal_sidebar.selected_session_ids();
        let selected_session_count = selected_session_ids.len();
        let can_join_selected_sessions =
            selected_session_count >= 2 && selected_sessions_share_profile(&snapshot, &selected_session_ids);
        let is_joined = joined_session_markers(self).contains_key(&session.session_id);

        let body_font = self.sidebar_text_font()?;
        let text = LinearRgba::with_components(0.92, 0.92, 0.92, 1.0);
        let hover_bg = LinearRgba::with_components(0.20, 0.24, 0.28, 1.0);
        let bg = LinearRgba::with_components(0.03, 0.03, 0.03, 0.995);
        let border = bg;
        let menu_item = |label: &str, item_type| {
            Element::new(&body_font, ElementContent::Text(label.to_string()))
                .display(crate::termwindow::box_model::DisplayType::Block)
                .item_type(item_type)
                .padding(BoxDimension {
                    left: Dimension::Pixels(10.0),
                    right: Dimension::Pixels(10.0),
                    top: Dimension::Pixels(6.0),
                    bottom: Dimension::Pixels(6.0),
                })
                .min_width(Some(Dimension::Pixels(
                    SIDEBAR_CONTEXT_MENU_ITEM_CONTENT_WIDTH_PX,
                )))
                .max_width(Some(Dimension::Pixels(
                    SIDEBAR_CONTEXT_MENU_ITEM_CONTENT_WIDTH_PX,
                )))
                .colors(filled_colors(LinearRgba::TRANSPARENT, text))
                .hover_colors(Some(filled_colors(hover_bg, text)))
        };
        let mut entries: Vec<(&str, UIItemType)> = Vec::new();
        if can_join_selected_sessions {
            entries.push((
                "Join session",
                UIItemType::ChatminalSidebarSessionMenuJoin(session.session_id.clone()),
            ));
        }
        if is_joined {
            entries.push((
                "Unjoin session",
                UIItemType::ChatminalSidebarSessionMenuUnjoin(session.session_id.clone()),
            ));
        }
        entries.push((
            "Đổi tên",
            UIItemType::ChatminalSidebarSessionMenuRename(session.session_id.clone()),
        ));
        entries.push((
            "Xoá",
            UIItemType::ChatminalSidebarSessionMenuDelete(session.session_id.clone()),
        ));

        let item_count = entries.len();
        let children = entries
            .into_iter()
            .map(|(label, item_type)| menu_item(label, item_type))
            .collect();

        let root = Element::new(&body_font, ElementContent::Children(children))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .item_type(UIItemType::ChatminalSidebarSessionMenu)
        .padding(BoxDimension::default())
        .border(BoxDimension::new(Dimension::Pixels(1.0)))
        .border_corners(Some(rounded_corners(7.0)))
        .colors(ElementColors {
            border: BorderColor::new(border),
            bg: bg.into(),
            text: text.into(),
        })
        .min_width(Some(Dimension::Pixels(SIDEBAR_CONTEXT_MENU_WIDTH_PX)))
        .max_width(Some(Dimension::Pixels(SIDEBAR_CONTEXT_MENU_WIDTH_PX)));

        let width = SIDEBAR_CONTEXT_MENU_WIDTH_PX;
        let estimated_height = 8.0 + item_count as f32 * 32.0;
        let x = menu
            .anchor_x_px
            .clamp(
                4.0,
                (self.dimensions.pixel_width as f32 - width - 4.0).max(4.0),
            );
        let y = menu
            .anchor_y_px
            .clamp(
                sb.sidebar_y + 4.0,
                (self.dimensions.pixel_height as f32 - estimated_height - 4.0)
                    .max(sb.sidebar_y + 4.0),
            );

        Ok(Some(self.compute_element(
            &crate::termwindow::box_model::LayoutContext {
                width: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: self.dimensions.pixel_width as f32,
                    pixel_cell: self.render_metrics.cell_size.width as f32,
                },
                height: config::DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: self.dimensions.pixel_height as f32,
                    pixel_cell: self.render_metrics.cell_size.height as f32,
                },
                bounds: euclid::rect(x, y, width, estimated_height),
                metrics: &self.render_metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 7,
            },
            &root,
        )?))
    }

    #[allow(clippy::too_many_arguments)]
    fn build_chatminal_sidebar_tree_row_elements(
        &mut self,
        rows: &[SidebarTreeRow],
        body_font: &std::rc::Rc<engine_font::LoadedFont>,
        status_font: &std::rc::Rc<engine_font::LoadedFont>,
        text: LinearRgba,
        muted: LinearRgba,
        accent: LinearRgba,
        session_active_bg: LinearRgba,
        session_selected_bg: LinearRgba,
        hover_bg: LinearRgba,
        offline: LinearRgba,
        error_fg: LinearRgba,
    ) -> anyhow::Result<Vec<Element>> {
        let suppress_hover = self.chatminal_sidebar.session_context_menu().is_some();
        let drag_state = self.chatminal_sidebar.session_drag_state();
        rows.iter()
            .map(|row| match row {
                SidebarTreeRow::Error(message) => Ok(Element::new(
                    body_font,
                    ElementContent::Text(message.clone()),
                )
                .display(crate::termwindow::box_model::DisplayType::Block)
                .margin(block_margin(14.0, 0.0))
                .colors(text_colors(error_fg))),
                SidebarTreeRow::EmptyHint(label) => Ok(empty_hint(body_font, label, muted)),
                SidebarTreeRow::Profile {
                    profile,
                    is_expanded,
                } => self.build_chatminal_sidebar_profile_row(
                    body_font,
                    profile,
                    *is_expanded,
                    drag_state.as_ref(),
                    text,
                    muted,
                    hover_bg,
                    suppress_hover,
                ),
                SidebarTreeRow::EmptyNestedHint(label) => {
                    Ok(empty_nested_hint(body_font, label, muted))
                }
                SidebarTreeRow::Session(session) => self.build_chatminal_sidebar_session_row(
                    body_font,
                    status_font,
                    session,
                    text,
                    muted,
                    accent,
                    session_active_bg,
                    session_selected_bg,
                    drag_state.as_ref(),
                    hover_bg,
                    offline,
                    suppress_hover,
                ),
            })
            .collect()
    }

    fn build_chatminal_sidebar_profile_row(
        &mut self,
        body_font: &std::rc::Rc<engine_font::LoadedFont>,
        profile: &SidebarProfile,
        is_expanded: bool,
        drag_state: Option<&SidebarSessionDragState>,
        text: LinearRgba,
        muted: LinearRgba,
        hover_bg: LinearRgba,
        suppress_hover: bool,
    ) -> anyhow::Result<Element> {
        let fg = if profile.is_active {
            text
        } else {
            LinearRgba::with_components(text.0, text.1, text.2, 0.92)
        };
        let chevron_fg = LinearRgba::with_components(muted.0, muted.1, muted.2, 0.95);
        let folder_fg = LinearRgba::with_components(0.773, 0.773, 0.773, 0.92);
        let is_append_target = drag_state
            .and_then(|drag| drag.drop_target.as_ref())
            .is_some_and(|target| {
                matches!(
                    target,
                    SidebarSessionDropTarget::ProfileAppend { profile_id }
                        if profile_id == &profile.profile_id
                )
            });
        let row_bg = if is_append_target {
            LinearRgba::with_components(0.038, 0.086, 0.129, 0.98)
        } else {
            LinearRgba::TRANSPARENT
        };
        let row_border = if is_append_target {
            LinearRgba::with_components(0.329, 0.608, 0.941, 1.0)
        } else {
            LinearRgba::TRANSPARENT
        };
        let chevron = self.sidebar_icon_element(
            if is_expanded {
                LucideIcon::ChevronDown
            } else {
                LucideIcon::ChevronRight
            },
            self.sidebar_twistie_icon_size_px(),
        )?;
        let folder = self.sidebar_icon_element(
            if is_expanded {
                LucideIcon::FolderOpen
            } else {
                LucideIcon::Folder
            },
            self.sidebar_icon_size_px(),
        )?;

        Ok(Element::new(
            body_font,
            ElementContent::Children(vec![
                inline_icon_with_margin(
                    chevron.colors(text_colors(chevron_fg)),
                    0.0,
                    7.0,
                    if is_expanded { 0.0 } else { 1.0 },
                    0.0,
                ),
                inline_icon(folder.colors(text_colors(folder_fg)), 7.0),
                Element::new(body_font, ElementContent::Text(profile.name.clone()))
                    .display(crate::termwindow::box_model::DisplayType::Inline)
                    .vertical_align(VerticalAlign::Middle)
                    .colors(text_colors(fg)),
            ]),
        )
        .display(crate::termwindow::box_model::DisplayType::Block)
        .item_type(UIItemType::ChatminalSidebarProfile(
            profile.profile_id.clone(),
        ))
        .padding(BoxDimension {
            left: Dimension::Pixels(6.0),
            right: Dimension::Pixels(8.0),
            top: Dimension::Pixels(4.0),
            bottom: Dimension::Pixels(4.0),
        })
        .border(BoxDimension {
            left: Dimension::Pixels(if is_append_target { 2.0 } else { 0.0 }),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .margin(block_margin(0.0, 0.0))
        .colors(ElementColors {
            border: BorderColor::new(row_border),
            bg: row_bg.into(),
            text: fg.into(),
        })
        .hover_colors((!suppress_hover).then_some(filled_colors(hover_bg, text))))
    }

    #[allow(clippy::too_many_arguments)]
    fn build_chatminal_sidebar_session_row(
        &mut self,
        body_font: &std::rc::Rc<engine_font::LoadedFont>,
        status_font: &std::rc::Rc<engine_font::LoadedFont>,
        session: &SidebarSession,
        text: LinearRgba,
        _muted: LinearRgba,
        accent: LinearRgba,
        session_active_bg: LinearRgba,
        session_selected_bg: LinearRgba,
        drag_state: Option<&SidebarSessionDragState>,
        hover_bg: LinearRgba,
        offline: LinearRgba,
        suppress_hover: bool,
    ) -> anyhow::Result<Element> {
        let is_running = session.status == "running";
        let is_selected = self.chatminal_sidebar.is_session_selected(&session.session_id);
        let row_bg = if session.is_active {
            session_active_bg
        } else if is_selected {
            session_selected_bg
        } else {
            LinearRgba::with_components(0.0, 0.0, 0.0, 0.0)
        };
        let name_color = if session.is_active {
            LinearRgba::with_components(1.0, 1.0, 1.0, 1.0)
        } else {
            LinearRgba::with_components(text.0, text.1, text.2, 0.92)
        };
        let status_color = if is_running {
            LinearRgba::with_components(accent.0, accent.1, accent.2, 0.8)
        } else {
            offline
        };
        let is_insert_target = drag_state
            .and_then(|drag| drag.drop_target.as_ref())
            .is_some_and(|target| {
                matches!(
                    target,
                    SidebarSessionDropTarget::SessionInsertBefore {
                        session_id,
                        ..
                    } if session_id == &session.session_id
                )
            });
        let is_dragged = drag_state
            .map(|drag| {
                drag.session_ids
                    .iter()
                    .any(|session_id| session_id == &session.session_id)
            })
            .unwrap_or(false);
        let terminal_fg = if session.is_active {
            LinearRgba::with_components(1.0, 1.0, 1.0, 0.96)
        } else {
            LinearRgba::with_components(0.773, 0.773, 0.773, 0.90)
        };
        let terminal = self
            .sidebar_icon_element(LucideIcon::SquareTerminal, self.sidebar_icon_size_px())?;
        let joined_markers = joined_session_markers(self);
        let joined_marker = joined_markers.get(&session.session_id).copied();
        let inline_rename = self.chatminal_sidebar.inline_rename_state();
        let is_inline_rename = inline_rename
            .as_ref()
            .map(|rename| rename.session_id == session.session_id)
            .unwrap_or(false);
        let is_inline_rename_selected = inline_rename
            .as_ref()
            .map(|rename| rename.session_id == session.session_id && rename.select_all)
            .unwrap_or(false);
        let session_label = if is_inline_rename {
            format!("{}_", inline_rename.as_ref().map(|rename| rename.input.as_str()).unwrap_or(""))
        } else {
            session.name.clone()
        };
        let status_suffix = if is_inline_rename {
            None
        } else {
            Some(format!(" ({})", if is_running { "Online" } else { "Offline" }))
        };
        let rename_bg = if is_inline_rename_selected {
            LinearRgba::with_components(0.086, 0.322, 0.620, 1.0)
        } else {
            LinearRgba::with_components(0.12, 0.12, 0.12, 1.0)
        };
        let rename_border = if is_inline_rename_selected {
            LinearRgba::with_components(0.184, 0.510, 0.855, 1.0)
        } else {
            LinearRgba::with_components(0.28, 0.28, 0.28, 1.0)
        };
        let rename_text = if is_inline_rename_selected {
            LinearRgba::with_components(1.0, 1.0, 1.0, 1.0)
        } else {
            name_color
        };

        let connector_color = if session.is_active {
            LinearRgba::with_components(0.80, 0.80, 0.80, 0.92)
        } else {
            LinearRgba::with_components(0.46, 0.46, 0.46, 0.92)
        };
        let indicator_border = if is_insert_target {
            LinearRgba::with_components(0.329, 0.608, 0.941, 1.0)
        } else {
            LinearRgba::TRANSPARENT
        };
        let row_bg = if is_insert_target && !session.is_active && !is_selected {
            LinearRgba::with_components(0.030, 0.072, 0.108, 0.72)
        } else {
            row_bg
        };
        let row_bg = if is_dragged {
            LinearRgba::with_components(row_bg.0, row_bg.1, row_bg.2, 0.55)
        } else {
            row_bg
        };
        let mut children = vec![
            match joined_marker {
                Some(marker) => Element::new(
                    body_font,
                    ElementContent::Poly {
                        line_width: 2,
                        poly: SizedPoly {
                            poly: marker.poly(),
                            width: Dimension::Pixels(SIDEBAR_JOIN_CONNECTOR_WIDTH_PX),
                            height: Dimension::Pixels(
                                self.render_metrics.cell_size.height as f32
                                    + SIDEBAR_JOIN_CONNECTOR_HEIGHT_OVERLAP_PX,
                            ),
                        },
                    },
                )
                .display(crate::termwindow::box_model::DisplayType::Inline)
                .vertical_align(VerticalAlign::Middle)
                .min_width(Some(Dimension::Pixels(
                    SIDEBAR_JOIN_CONNECTOR_SLOT_WIDTH_PX,
                )))
                .margin(BoxDimension {
                    left: Dimension::Pixels(0.0),
                    right: Dimension::Pixels(2.0),
                    top: Dimension::Pixels(0.0),
                    bottom: Dimension::Pixels(0.0),
                })
                .colors(text_colors(connector_color)),
                None => Element::new(body_font, ElementContent::Text(String::new()))
                    .display(crate::termwindow::box_model::DisplayType::Inline)
                    .vertical_align(VerticalAlign::Middle)
                    .min_width(Some(Dimension::Pixels(
                        SIDEBAR_JOIN_CONNECTOR_SLOT_WIDTH_PX,
                    )))
                    .margin(BoxDimension {
                        left: Dimension::Pixels(0.0),
                        right: Dimension::Pixels(2.0),
                        top: Dimension::Pixels(0.0),
                        bottom: Dimension::Pixels(0.0),
                    })
                    .colors(text_colors(connector_color)),
            },
            inline_icon(terminal.colors(text_colors(terminal_fg)), 7.0),
            Element::new(body_font, ElementContent::Text(session_label))
                .display(crate::termwindow::box_model::DisplayType::Inline)
                .vertical_align(VerticalAlign::Middle)
                .padding(if is_inline_rename {
                    BoxDimension {
                        left: Dimension::Pixels(6.0),
                        right: Dimension::Pixels(6.0),
                        top: Dimension::Pixels(1.0),
                        bottom: Dimension::Pixels(1.0),
                    }
                } else {
                    BoxDimension::default()
                })
                .border(if is_inline_rename {
                    BoxDimension::new(Dimension::Pixels(1.0))
                } else {
                    BoxDimension::default()
                })
                .colors(if is_inline_rename {
                    ElementColors {
                        border: BorderColor::new(rename_border),
                        bg: rename_bg.into(),
                        text: rename_text.into(),
                    }
                } else {
                    text_colors(name_color)
                }),
        ];
        if let Some(label) = status_suffix {
            children.push(
                Element::new(status_font, ElementContent::Text(label))
                    .display(crate::termwindow::box_model::DisplayType::Inline)
                    .vertical_align(VerticalAlign::Middle)
                    .colors(text_colors(status_color)),
            );
        }

        Ok(Element::new(body_font, ElementContent::Children(children))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .item_type(UIItemType::ChatminalSidebarSession(
            session.session_id.clone(),
        ))
        .padding(BoxDimension {
            left: Dimension::Pixels(6.0),
            right: Dimension::Pixels(8.0),
            top: Dimension::Pixels(4.0),
            bottom: Dimension::Pixels(4.0),
        })
        .border(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(if is_insert_target { 3.0 } else { 0.0 }),
            bottom: Dimension::Pixels(0.0),
        })
        .margin(BoxDimension {
            left: Dimension::Pixels(22.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(1.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(ElementColors {
            border: BorderColor::new(indicator_border),
            bg: row_bg.into(),
            text: name_color.into(),
        })
        .hover_colors((!suppress_hover).then_some(filled_colors(hover_bg, text))))
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
        sb: &crate::shell_bounds::ShellBounds,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let x = sb.footer_x;
        let width = sb.footer_width;
        let height = sb.footer_height;
        let y = sb.footer_y;
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
        sb: &crate::shell_bounds::ShellBounds,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let snapshot = ordered_sidebar_snapshot(self, self.chatminal_sidebar.snapshot());
        let sidebar_width = sb.sidebar_width;
        let x = 0.0;
        let width = sb.window_width;
        let height = sb.footer_height;
        let y = sb.footer_y;
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
                rows.push(SidebarTreeRow::EmptyNestedHint(
                    "No sessions yet".to_string(),
                ));
            } else {
                for session in profile_sessions {
                    rows.push(SidebarTreeRow::Session(session.clone()));
                }
            }
        }
    }
    rows
}

fn joined_session_markers(
    _term_window: &crate::TermWindow,
) -> std::collections::BTreeMap<String, JoinedSessionMarker> {
    let snapshot = _term_window.chatminal_sidebar.snapshot();
    let active_profile_id = snapshot.active_profile_id.as_deref();
    let mut groups = Vec::new();
    let mut seen_group_keys = std::collections::BTreeSet::new();

    for profile in &snapshot.profiles {
        let mut workspace_ids = std::collections::BTreeSet::from([
            crate::chatminal_layout::workspace_store::DesktopWorkspaceLayoutStore::profile_workspace_id(
                &profile.profile_id,
            ),
        ]);
        for session in snapshot
            .sessions
            .iter()
            .filter(|session| session.profile_id == profile.profile_id)
        {
            workspace_ids.insert(
                crate::chatminal_layout::workspace_store::DesktopWorkspaceLayoutStore::profile_group_workspace_id(
                    &profile.profile_id,
                    &session.session_id,
                ),
            );
        }
        if active_profile_id == Some(profile.profile_id.as_str()) {
            workspace_ids.insert(
                crate::chatminal_layout::workspace_store::DEFAULT_LAYOUT_WORKSPACE_ID.to_string(),
            );
        }
        for workspace_id in workspace_ids {
            let Some(layout) = crate::chatminal_layout::workspace_store::DesktopWorkspaceLayoutStore::new(
                workspace_id,
            )
            .snapshot_or_restore() else {
                continue;
            };
            if layout.views.len() <= 1 {
                continue;
            }
            let joined_session_ids = layout
                .views
                .into_iter()
                .map(|view| view.session_id)
                .collect::<Vec<_>>();
            let mut dedupe_key = joined_session_ids.clone();
            dedupe_key.sort();
            if !seen_group_keys.insert(dedupe_key.join("\u{1f}")) {
                continue;
            }

            let joined_members = joined_session_ids
                .iter()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>();
            let ordered_group = snapshot
                .sessions
                .iter()
                .filter(|session| session.profile_id == profile.profile_id)
                .filter(|session| joined_members.contains(session.session_id.as_str()))
                .map(|session| session.session_id.clone())
                .collect::<Vec<_>>();
            if ordered_group.len() > 1 {
                groups.push(ordered_group);
            }
        }
    }

    joined_session_markers_from_groups(&groups)
}

fn joined_session_markers_from_groups(
    groups: &[Vec<String>],
) -> std::collections::BTreeMap<String, JoinedSessionMarker> {
    let mut markers = std::collections::BTreeMap::new();

    for group in groups {
        if group.len() <= 1 {
            continue;
        }
        for (index, session_id) in group.iter().enumerate() {
            let marker = if index == 0 {
                JoinedSessionMarker::Start
            } else if index + 1 == group.len() {
                JoinedSessionMarker::End
            } else {
                JoinedSessionMarker::Middle
            };
            markers.insert(session_id.clone(), marker);
        }
    }

    markers
}

fn selected_sessions_share_profile(
    snapshot: &SidebarSnapshot,
    selected_session_ids: &[String],
) -> bool {
    let mut profile_id: Option<&str> = None;
    for session_id in selected_session_ids {
        let Some(session) = snapshot
            .sessions
            .iter()
            .find(|session| session.session_id == *session_id)
        else {
            return false;
        };
        match profile_id {
            Some(current) if current != session.profile_id => return false,
            Some(_) => {}
            None => profile_id = Some(session.profile_id.as_str()),
        }
    }
    profile_id.is_some()
}

#[cfg(test)]
mod tests {
    use super::{joined_session_markers_from_groups, JoinedSessionMarker};

    #[test]
    fn joined_session_markers_assigns_visual_positions_per_group() {
        let markers = joined_session_markers_from_groups(&[
            vec!["session-1".to_string(), "session-2".to_string()],
            vec![
                "session-3".to_string(),
                "session-4".to_string(),
                "session-5".to_string(),
            ],
        ]);

        assert_eq!(markers.get("session-1"), Some(&JoinedSessionMarker::Start));
        assert_eq!(markers.get("session-2"), Some(&JoinedSessionMarker::End));
        assert_eq!(markers.get("session-3"), Some(&JoinedSessionMarker::Start));
        assert_eq!(markers.get("session-4"), Some(&JoinedSessionMarker::Middle));
        assert_eq!(markers.get("session-5"), Some(&JoinedSessionMarker::End));
    }
}

fn clamp_ui_item_to_rect(
    item: crate::termwindow::UIItem,
    rect: ::window::RectF,
) -> Option<crate::termwindow::UIItem> {
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

fn inline_icon(icon: Element, right_margin: f32) -> Element {
    icon.display(crate::termwindow::box_model::DisplayType::Inline)
        .vertical_align(VerticalAlign::Middle)
        .margin(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(right_margin),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
}

fn inline_icon_with_margin(
    icon: Element,
    left_margin: f32,
    right_margin: f32,
    top_margin: f32,
    bottom_margin: f32,
) -> Element {
    icon.display(crate::termwindow::box_model::DisplayType::Inline)
        .vertical_align(VerticalAlign::Middle)
        .margin(BoxDimension {
            left: Dimension::Pixels(left_margin),
            right: Dimension::Pixels(right_margin),
            top: Dimension::Pixels(top_margin),
            bottom: Dimension::Pixels(bottom_margin),
        })
}

fn icon_button(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    icon: Element,
    item_type: Option<UIItemType>,
    text: LinearRgba,
    hover_bg: LinearRgba,
) -> Element {
    let mut element = Element::new(body_font, ElementContent::Children(vec![icon]))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .padding(BoxDimension {
            left: Dimension::Pixels(3.0),
            right: Dimension::Pixels(3.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .margin(BoxDimension {
            left: Dimension::Pixels(2.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(1.0),
        })
        .border(BoxDimension::new(Dimension::Pixels(1.0)))
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

fn sidebar_text_style() -> TextStyle {
    TextStyle {
        foreground: None,
        font: sidebar_font_stack(FontWeight::REGULAR),
    }
}

fn sidebar_header_text_style() -> TextStyle {
    TextStyle {
        foreground: None,
        font: sidebar_font_stack(FontWeight::REGULAR),
    }
}

fn sidebar_font_stack(weight: FontWeight) -> Vec<FontAttributes> {
    #[cfg(target_os = "macos")]
    let families = [
        "Helvetica Neue",
        "Helvetica",
        "Arial",
    ];

    #[cfg(target_os = "windows")]
    let families = ["Segoe WPC", "Segoe UI", "Arial"];

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let families = ["Ubuntu", "Noto Sans", "Droid Sans", "Arial"];

    IntoIterator::into_iter(families)
        .enumerate()
        .map(|(idx, family)| FontAttributes {
            family: family.to_string(),
            weight,
            is_fallback: idx != 0,
            ..FontAttributes::default()
        })
        .collect()
}
