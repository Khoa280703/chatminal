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

const RAIL_WIDTH_PX: f32 = 48.0;

impl crate::TermWindow {
    pub fn paint_chatminal_sidebar(&mut self) -> anyhow::Result<()> {
        if !self.chatminal_sidebar.is_enabled() {
            return Ok(());
        }
        let footer = self.build_chatminal_terminal_footer()?;
        let sidebar = self.build_chatminal_sidebar()?;
        // Footer rendered first (bottom layer) — full window width
        // Sidebar rendered last to cover the left portion of footer
        self.append_and_render_overlay(&footer)?;
        self.append_and_render_overlay(&sidebar)
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

    fn build_chatminal_sidebar(
        &mut self,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let snapshot = self.chatminal_sidebar.snapshot();
        let border = self.get_os_border();
        let sidebar_width = self.chatminal_sidebar_width() as f32;
        let sidebar_height =
            self.dimensions
                .pixel_height
                .saturating_sub((border.top + border.bottom).get() as usize) as f32;
        let body_font = self.fonts.title_font()?;
        let title_font = self.fonts.title_font()?;
        let status_font = self.fonts.title_font()?;

        let root_bg = LinearRgba::with_components(0.0, 0.0, 0.0, 1.0);
        let rail_bg = LinearRgba::with_components(0.067, 0.067, 0.067, 1.0);
        let panel_bg = LinearRgba::with_components(0.035, 0.035, 0.035, 1.0);
        let root_border = LinearRgba::with_components(0.133, 0.133, 0.133, 1.0);
        let panel_border = LinearRgba::with_components(0.133, 0.133, 0.133, 1.0);
        let text = LinearRgba::with_components(0.867, 0.867, 0.867, 1.0);
        let muted = LinearRgba::with_components(0.533, 0.533, 0.533, 1.0);
        let accent = LinearRgba::with_components(0.318, 0.639, 0.318, 1.0);
        let active_rail_bg = LinearRgba::with_components(0.145, 0.169, 0.153, 1.0);
        let session_active_bg = LinearRgba::with_components(0.122, 0.125, 0.133, 1.0);
        let hover_bg = LinearRgba::with_components(0.118, 0.118, 0.118, 1.0);
        let offline = LinearRgba::with_components(0.533, 0.533, 0.533, 1.0);
        let error_fg = LinearRgba::with_components(0.92, 0.38, 0.32, 1.0);

        let rail = build_rail(
            &body_font,
            sidebar_height,
            rail_bg,
            root_border,
            text,
            muted,
            active_rail_bg,
            hover_bg,
            accent,
        );
        let body = build_body(
            &body_font,
            &title_font,
            &status_font,
            &snapshot,
            sidebar_width - RAIL_WIDTH_PX,
            panel_bg,
            panel_border,
            text,
            muted,
            accent,
            session_active_bg,
            hover_bg,
            offline,
            error_fg,
        );

        let root = Element::new(&body_font, ElementContent::Children(vec![rail, body]))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .item_type(UIItemType::ChatminalSidebarBackground)
            .colors(ElementColors {
                border: BorderColor::new(root_border),
                bg: root_bg.into(),
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

    #[allow(dead_code)]
    fn build_chatminal_terminal_chrome(
        &mut self,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let snapshot = self.chatminal_sidebar.snapshot();
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

    fn build_chatminal_terminal_footer(
        &mut self,
    ) -> anyhow::Result<crate::termwindow::box_model::ComputedElement> {
        let snapshot = self.chatminal_sidebar.snapshot();
        let border = self.get_os_border();
        // Span full window width so background fills to right edge;
        // sidebar renders on top and covers the left portion.
        let x = 0.0;
        let width = self.dimensions.pixel_width as f32;
        let height = self.chatminal_terminal_footer_height();
        let y =
            (self.dimensions.pixel_height as f32 - border.bottom.get() as f32 - height).max(0.0);
        // Use title_font (Roboto/system UI, ~11-12pt) — smaller than terminal font
        let body_font = self.fonts.title_font()?;
        // Background matches terminal (#000000), subtle top border only
        let bg = LinearRgba::with_components(0.0, 0.0, 0.0, 1.0);
        let divider = LinearRgba::with_components(0.133, 0.133, 0.133, 1.0);
        let label = LinearRgba::with_components(0.35, 0.35, 0.35, 1.0); // muted labels
        let value = LinearRgba::with_components(0.65, 0.65, 0.65, 1.0); // brighter values
        let sep = LinearRgba::with_components(0.25, 0.25, 0.25, 1.0); // dim pipe separator

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

        // Build inline label+value pairs separated by pipes
        let items: Vec<(&str, String)> = vec![
            (
                "Session: ",
                format!("{} ({})", active_session, active_profile),
            ),
            ("CPU: ", metrics.cpu_display()),
            ("RAM: ", metrics.ram_display()),
            ("Latency: ", metrics.latency_display()),
        ];
        let mut inline_parts = Vec::new();
        for (i, (lbl, val)) in items.iter().enumerate() {
            if i > 0 {
                inline_parts.push(
                    Element::new(&body_font, ElementContent::Text("  |  ".to_string()))
                        .display(crate::termwindow::box_model::DisplayType::Inline)
                        .colors(text_colors(sep)),
                );
            }
            inline_parts.push(
                Element::new(&body_font, ElementContent::Text(lbl.to_string()))
                    .display(crate::termwindow::box_model::DisplayType::Inline)
                    .colors(text_colors(label)),
            );
            inline_parts.push(
                Element::new(&body_font, ElementContent::Text(val.clone()))
                    .display(crate::termwindow::box_model::DisplayType::Inline)
                    .colors(text_colors(value)),
            );
        }

        // Inner content row — float right to align status items to the right edge
        let content_row = Element::new(&body_font, ElementContent::Children(inline_parts))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .float(Float::Right)
            .colors(text_colors(value));

        // Outer block fills full terminal width with black background + top border
        let root = Element::new(&body_font, ElementContent::Children(vec![content_row]))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .padding(BoxDimension {
                left: Dimension::Pixels(12.0),
                right: Dimension::Pixels(12.0),
                top: Dimension::Pixels(8.0),
                bottom: Dimension::Pixels(8.0),
            })
            .border(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(0.0),
                top: Dimension::Pixels(1.0),
                bottom: Dimension::Pixels(0.0),
            })
            .colors(ElementColors {
                border: BorderColor::new(divider),
                bg: bg.into(),
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

fn build_rail(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    sidebar_height: f32,
    rail_bg: LinearRgba,
    border: LinearRgba,
    text: LinearRgba,
    muted: LinearRgba,
    button_bg: LinearRgba,
    hover_bg: LinearRgba,
    accent: LinearRgba,
) -> Element {
    let top_group = vec![
        rail_icon(body_font, "⚙", accent, button_bg, hover_bg, None, false),
        rail_icon(body_font, "◌", muted, rail_bg, hover_bg, None, false),
    ];
    let bottom_group = vec![
        rail_icon(body_font, "⚙", muted, rail_bg, hover_bg, None, false),
        rail_icon(body_font, "◌", muted, rail_bg, hover_bg, None, false),
        rail_icon(
            body_font,
            "+",
            text,
            rail_bg,
            hover_bg,
            Some(UIItemType::ChatminalSidebarCreateProfile),
            false,
        ),
    ];
    let spacer_height = (sidebar_height - 248.0).max(48.0);

    let children = vec![
        Element::new(body_font, ElementContent::Children(top_group))
            .display(crate::termwindow::box_model::DisplayType::Block),
        rail_spacer(body_font, spacer_height),
        Element::new(body_font, ElementContent::Children(bottom_group))
            .display(crate::termwindow::box_model::DisplayType::Block),
    ];

    Element::new(body_font, ElementContent::Children(children))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .padding(BoxDimension {
            left: Dimension::Pixels(4.0),
            right: Dimension::Pixels(4.0),
            top: Dimension::Pixels(14.0),
            bottom: Dimension::Pixels(14.0),
        })
        .border(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(1.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(ElementColors {
            border: BorderColor::new(border),
            bg: rail_bg.into(),
            text: text.into(),
        })
        .min_width(Some(Dimension::Pixels(RAIL_WIDTH_PX)))
}

#[allow(clippy::too_many_arguments)]
fn build_body(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    title_font: &std::rc::Rc<engine_font::LoadedFont>,
    status_font: &std::rc::Rc<engine_font::LoadedFont>,
    snapshot: &SidebarSnapshot,
    body_width: f32,
    panel_bg: LinearRgba,
    panel_border: LinearRgba,
    text: LinearRgba,
    muted: LinearRgba,
    accent: LinearRgba,
    session_active_bg: LinearRgba,
    hover_bg: LinearRgba,
    offline: LinearRgba,
    error_fg: LinearRgba,
) -> Element {
    let mut children = vec![header_row(
        body_font, title_font, body_width, text, muted, hover_bg,
    )];
    children.push(section_divider(body_font, panel_border));

    if let Some(error) = &snapshot.error {
        children.push(
            Element::new(body_font, ElementContent::Text(error.clone()))
                .display(crate::termwindow::box_model::DisplayType::Block)
                .margin(block_margin(14.0, 0.0))
                .colors(text_colors(error_fg)),
        );
    } else if snapshot.profiles.is_empty() {
        children.push(empty_hint(body_font, "No profiles yet", muted));
    } else {
        append_profile_tree(
            &mut children,
            body_font,
            status_font,
            &snapshot.profiles,
            &snapshot.sessions,
            text,
            muted,
            accent,
            session_active_bg,
            hover_bg,
            offline,
        );
    }

    Element::new(body_font, ElementContent::Children(children))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .padding(BoxDimension {
            left: Dimension::Pixels(8.0),
            right: Dimension::Pixels(8.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(8.0),
        })
        .border(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(ElementColors {
            border: BorderColor::new(panel_border),
            bg: panel_bg.into(),
            text: text.into(),
        })
        .min_width(Some(Dimension::Pixels(body_width.max(200.0))))
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
            bottom: Dimension::Pixels(8.0),
        })
        .min_width(Some(Dimension::Pixels((body_width - 16.0).max(120.0))))
        .min_height(Some(Dimension::Pixels(44.0)))
        .colors(text_colors(text))
}

#[allow(clippy::too_many_arguments)]
fn append_profile_tree(
    children: &mut Vec<Element>,
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    status_font: &std::rc::Rc<engine_font::LoadedFont>,
    profiles: &[SidebarProfile],
    sessions: &[SidebarSession],
    text: LinearRgba,
    muted: LinearRgba,
    accent: LinearRgba,
    session_active_bg: LinearRgba,
    hover_bg: LinearRgba,
    offline: LinearRgba,
) {
    for profile in profiles {
        children.push(profile_row(body_font, profile, text, muted, hover_bg));

        if profile.is_active {
            if sessions.is_empty() {
                children.push(empty_nested_hint(body_font, "No sessions yet", muted));
            } else {
                for session in sessions {
                    children.push(session_card(
                        body_font,
                        session,
                        text,
                        muted,
                        accent,
                        status_font,
                        session_active_bg,
                        hover_bg,
                        offline,
                    ));
                }
            }
        }
    }
}

fn profile_row(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    profile: &SidebarProfile,
    text: LinearRgba,
    muted: LinearRgba,
    hover_bg: LinearRgba,
) -> Element {
    let marker = if profile.is_active { "v" } else { ">" };
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
        .margin(block_margin(if profile.is_active { 8.0 } else { 3.0 }, 0.0))
        .colors(filled_colors(LinearRgba::TRANSPARENT, fg))
        .border_corners(Some(rounded_corners(6.0)))
        .hover_colors(Some(filled_colors(hover_bg, text)))
}

// Session row styled as a simple tree child item matching Stitch design:
// "● name (Online)" with colored dot, indented under active profile
#[allow(clippy::too_many_arguments)]
fn session_card(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    session: &SidebarSession,
    text: LinearRgba,
    muted: LinearRgba,
    accent: LinearRgba,
    status_font: &std::rc::Rc<engine_font::LoadedFont>,
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

    // Active session: highlighted bg #1f2022, white text; inactive: transparent, gray-400
    let row_bg = if session.is_active {
        session_active_bg
    } else {
        LinearRgba::with_components(0.0, 0.0, 0.0, 0.0)
    };
    let name_color = if session.is_active { text } else { muted };

    // Row: "● name" dot then name then "(Online)" status
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
        .border_corners(Some(rounded_corners(4.0)))
        .colors(ElementColors {
            border: BorderColor::default(),
            bg: row_bg.into(),
            text: name_color.into(),
        })
        .hover_colors(Some(filled_colors(hover_bg, text)))
}

#[allow(dead_code)]
fn rail_badge(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    accent: LinearRgba,
    _rail_bg: LinearRgba,
) -> Element {
    Element::new(body_font, ElementContent::Text("[]".to_string()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .padding(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(6.0),
            bottom: Dimension::Pixels(6.0),
        })
        .margin(block_margin(0.0, 10.0))
        .colors(ElementColors {
            border: BorderColor::new(accent),
            bg: LinearRgba::with_components(0.02, 0.06, 0.05, 1.0).into(),
            text: accent.into(),
        })
        .border(BoxDimension::new(Dimension::Pixels(1.0)))
        .border_corners(Some(rounded_corners(8.0)))
}

fn rail_icon(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    text: LinearRgba,
    bg: LinearRgba,
    hover_bg: LinearRgba,
    item_type: Option<UIItemType>,
    outlined: bool,
) -> Element {
    let mut element = Element::new(body_font, ElementContent::Text(label.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .padding(BoxDimension {
            left: Dimension::Pixels(4.0),
            right: Dimension::Pixels(4.0),
            top: Dimension::Pixels(5.0),
            bottom: Dimension::Pixels(5.0),
        })
        .margin(block_margin(0.0, 4.0))
        .border(BoxDimension::new(Dimension::Pixels(1.0)))
        .border_corners(Some(rounded_corners(6.0)))
        .colors(ElementColors {
            border: BorderColor::new(if outlined { text } else { bg }),
            bg: bg.into(),
            text: text.into(),
        })
        .hover_colors(Some(filled_colors(hover_bg, text)));
    if let Some(item_type) = item_type {
        element = element.item_type(item_type);
    }
    element
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

fn section_divider(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    divider: LinearRgba,
) -> Element {
    Element::new(body_font, ElementContent::Text(String::new()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .border(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(1.0),
        })
        .margin(block_margin(0.0, 8.0))
        .colors(ElementColors {
            border: BorderColor::new(divider),
            bg: LinearRgba::TRANSPARENT.into(),
            text: LinearRgba::TRANSPARENT.into(),
        })
}

// Spacer that pushes bottom group toward the end of the rail
fn rail_spacer(body_font: &std::rc::Rc<engine_font::LoadedFont>, height: f32) -> Element {
    Element::new(body_font, ElementContent::Text(String::new()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .min_height(Some(Dimension::Pixels(height)))
        .margin(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(text_colors(LinearRgba::TRANSPARENT))
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
