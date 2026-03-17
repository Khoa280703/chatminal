use std::collections::BTreeMap;

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

#[derive(Clone)]
struct SidebarProfileGroup<'a> {
    profile: &'a SidebarProfile,
    sessions: Vec<&'a SidebarSession>,
}

fn ordered_sidebar_snapshot(mut snapshot: SidebarSnapshot) -> SidebarSnapshot {
    let active_session_id = snapshot.active_session_id.clone().or_else(|| {
        snapshot
            .sessions
            .iter()
            .find(|session| session.is_active)
            .map(|session| session.session_id.clone())
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
        let footer = self.build_chatminal_terminal_footer()?;
        let sidebar = self.build_chatminal_sidebar()?;
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
        let snapshot = ordered_sidebar_snapshot(self.chatminal_sidebar.snapshot());
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
        let tree_line = LinearRgba::with_components(0.18, 0.18, 0.18, 1.0);
        let badge_bg = LinearRgba::with_components(0.08, 0.08, 0.08, 1.0);

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
            tree_line,
            badge_bg,
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
        let snapshot = ordered_sidebar_snapshot(self.chatminal_sidebar.snapshot());
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
        let snapshot = ordered_sidebar_snapshot(self.chatminal_sidebar.snapshot());
        let border = self.get_os_border();
        let x = 0.0;
        let width = self.dimensions.pixel_width as f32;
        let height = self.chatminal_terminal_footer_height();
        let y =
            (self.dimensions.pixel_height as f32 - border.bottom.get() as f32 - height).max(0.0);
        let body_font = self.fonts.title_font()?;
        let bg = LinearRgba::with_components(0.0, 0.0, 0.0, 1.0);
        let divider = LinearRgba::with_components(0.133, 0.133, 0.133, 1.0);
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

        let content_row = Element::new(&body_font, ElementContent::Children(inline_parts))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .float(Float::Right)
            .colors(text_colors(value));

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
    tree_line: LinearRgba,
    badge_bg: LinearRgba,
) -> Element {
    let groups = grouped_profiles(snapshot);
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
    } else if groups.is_empty() {
        children.push(empty_hint(body_font, "No profiles yet", muted));
    } else {
        append_profile_tree(
            &mut children,
            body_font,
            status_font,
            &groups,
            text,
            muted,
            accent,
            session_active_bg,
            hover_bg,
            offline,
            tree_line,
            badge_bg,
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

fn grouped_profiles(snapshot: &SidebarSnapshot) -> Vec<SidebarProfileGroup<'_>> {
    let mut sessions_by_profile: BTreeMap<&str, Vec<&SidebarSession>> = BTreeMap::new();
    for session in &snapshot.sessions {
        sessions_by_profile
            .entry(session.profile_id.as_str())
            .or_default()
            .push(session);
    }

    snapshot
        .profiles
        .iter()
        .map(|profile| SidebarProfileGroup {
            profile,
            sessions: sessions_by_profile
                .remove(profile.profile_id.as_str())
                .unwrap_or_default(),
        })
        .collect()
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
    groups: &[SidebarProfileGroup<'_>],
    text: LinearRgba,
    muted: LinearRgba,
    accent: LinearRgba,
    session_active_bg: LinearRgba,
    hover_bg: LinearRgba,
    offline: LinearRgba,
    tree_line: LinearRgba,
    badge_bg: LinearRgba,
) {
    for group in groups {
        children.push(profile_row(
            body_font,
            group.profile,
            group.sessions.len(),
            text,
            muted,
            hover_bg,
            badge_bg,
            tree_line,
        ));

        if !group.profile.is_expanded {
            continue;
        }

        if group.sessions.is_empty() {
            children.push(empty_nested_hint(
                body_font,
                "No sessions yet",
                muted,
                tree_line,
            ));
            continue;
        }

        children.push(session_branch(
            body_font,
            status_font,
            &group.sessions,
            text,
            muted,
            accent,
            session_active_bg,
            hover_bg,
            offline,
            tree_line,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn profile_row(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    profile: &SidebarProfile,
    session_count: usize,
    text: LinearRgba,
    muted: LinearRgba,
    hover_bg: LinearRgba,
    badge_bg: LinearRgba,
    tree_line: LinearRgba,
) -> Element {
    let row_bg = if profile.is_active {
        LinearRgba::with_components(0.075, 0.082, 0.094, 1.0)
    } else {
        LinearRgba::TRANSPARENT
    };
    let row_border = if profile.is_active {
        tree_line
    } else {
        LinearRgba::TRANSPARENT
    };
    let fg = if profile.is_active { text } else { muted };
    let toggle = Element::new(
        body_font,
        ElementContent::Text(if profile.is_expanded { "v" } else { ">" }.to_string()),
    )
    .display(crate::termwindow::box_model::DisplayType::Inline)
    .item_type(UIItemType::ChatminalSidebarToggleProfile(
        profile.profile_id.clone(),
    ))
    .padding(BoxDimension {
        left: Dimension::Pixels(2.0),
        right: Dimension::Pixels(6.0),
        top: Dimension::Pixels(0.0),
        bottom: Dimension::Pixels(0.0),
    })
    .colors(text_colors(fg))
    .hover_colors(Some(text_colors(text)));

    let label = Element::new(body_font, ElementContent::Text(profile.name.clone()))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .colors(text_colors(fg));

    let count = Element::new(body_font, ElementContent::Text(session_count.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .float(Float::Right)
        .padding(BoxDimension {
            left: Dimension::Pixels(6.0),
            right: Dimension::Pixels(6.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .border(BoxDimension::new(Dimension::Pixels(1.0)))
        .border_corners(Some(rounded_corners(999.0)))
        .colors(ElementColors {
            border: BorderColor::new(if profile.is_active {
                row_border
            } else {
                tree_line
            }),
            bg: badge_bg.into(),
            text: muted.into(),
        });

    Element::new(
        body_font,
        ElementContent::Children(vec![toggle, label, count]),
    )
    .display(crate::termwindow::box_model::DisplayType::Block)
    .item_type(UIItemType::ChatminalSidebarProfile(
        profile.profile_id.clone(),
    ))
    .padding(BoxDimension {
        left: Dimension::Pixels(8.0),
        right: Dimension::Pixels(8.0),
        top: Dimension::Pixels(7.0),
        bottom: Dimension::Pixels(7.0),
    })
    .margin(block_margin(8.0, 0.0))
    .border(BoxDimension {
        left: Dimension::Pixels(1.0),
        right: Dimension::Pixels(0.0),
        top: Dimension::Pixels(0.0),
        bottom: Dimension::Pixels(0.0),
    })
    .border_corners(Some(rounded_corners(7.0)))
    .colors(ElementColors {
        border: BorderColor::new(row_border),
        bg: row_bg.into(),
        text: fg.into(),
    })
    .hover_colors(Some(filled_colors(hover_bg, text)))
}

#[allow(clippy::too_many_arguments)]
fn session_branch(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    status_font: &std::rc::Rc<engine_font::LoadedFont>,
    sessions: &[&SidebarSession],
    text: LinearRgba,
    muted: LinearRgba,
    accent: LinearRgba,
    session_active_bg: LinearRgba,
    hover_bg: LinearRgba,
    offline: LinearRgba,
    tree_line: LinearRgba,
) -> Element {
    let children = sessions
        .iter()
        .map(|session| {
            session_row(
                body_font,
                status_font,
                session,
                text,
                muted,
                accent,
                session_active_bg,
                hover_bg,
                offline,
            )
        })
        .collect();

    Element::new(body_font, ElementContent::Children(children))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .margin(BoxDimension {
            left: Dimension::Pixels(20.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(2.0),
            bottom: Dimension::Pixels(0.0),
        })
        .padding(BoxDimension {
            left: Dimension::Pixels(10.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(4.0),
        })
        .border(BoxDimension {
            left: Dimension::Pixels(1.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(ElementColors {
            border: BorderColor::new(tree_line),
            bg: LinearRgba::TRANSPARENT.into(),
            text: text.into(),
        })
}

#[allow(clippy::too_many_arguments)]
fn session_row(
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
    let dot_color = if is_running { accent } else { offline };
    let status_text = if is_running { "online" } else { "offline" };
    let row_bg = if session.is_active {
        session_active_bg
    } else {
        LinearRgba::TRANSPARENT
    };
    let row_fg = if session.is_active { text } else { muted };
    let badge_border = if session.is_active { accent } else { offline };

    let status_badge = Element::new(status_font, ElementContent::Text(status_text.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .float(Float::Right)
        .padding(BoxDimension {
            left: Dimension::Pixels(6.0),
            right: Dimension::Pixels(6.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .border(BoxDimension::new(Dimension::Pixels(1.0)))
        .border_corners(Some(rounded_corners(999.0)))
        .colors(ElementColors {
            border: BorderColor::new(badge_border),
            bg: LinearRgba::with_components(0.05, 0.05, 0.05, 1.0).into(),
            text: badge_border.into(),
        });

    let dot = Element::new(body_font, ElementContent::Text("o".to_string()))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .padding(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(8.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(text_colors(dot_color));

    let label = Element::new(body_font, ElementContent::Text(session.name.clone()))
        .display(crate::termwindow::box_model::DisplayType::Inline)
        .colors(text_colors(row_fg));

    Element::new(
        body_font,
        ElementContent::Children(vec![status_badge, dot, label]),
    )
    .display(crate::termwindow::box_model::DisplayType::Block)
    .item_type(UIItemType::ChatminalSidebarSession(
        session.session_id.clone(),
    ))
    .padding(BoxDimension {
        left: Dimension::Pixels(8.0),
        right: Dimension::Pixels(8.0),
        top: Dimension::Pixels(6.0),
        bottom: Dimension::Pixels(6.0),
    })
    .margin(BoxDimension {
        left: Dimension::Pixels(0.0),
        right: Dimension::Pixels(0.0),
        top: Dimension::Pixels(2.0),
        bottom: Dimension::Pixels(0.0),
    })
    .border(BoxDimension {
        left: Dimension::Pixels(if session.is_active { 2.0 } else { 0.0 }),
        right: Dimension::Pixels(0.0),
        top: Dimension::Pixels(0.0),
        bottom: Dimension::Pixels(0.0),
    })
    .border_corners(Some(rounded_corners(6.0)))
    .colors(ElementColors {
        border: BorderColor::new(accent),
        bg: row_bg.into(),
        text: row_fg.into(),
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
    tree_line: LinearRgba,
) -> Element {
    Element::new(body_font, ElementContent::Text(label.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .margin(BoxDimension {
            left: Dimension::Pixels(20.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(2.0),
            bottom: Dimension::Pixels(6.0),
        })
        .padding(BoxDimension {
            left: Dimension::Pixels(10.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(4.0),
            bottom: Dimension::Pixels(0.0),
        })
        .border(BoxDimension {
            left: Dimension::Pixels(1.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(ElementColors {
            border: BorderColor::new(tree_line),
            bg: LinearRgba::TRANSPARENT.into(),
            text: muted_fg.into(),
        })
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
