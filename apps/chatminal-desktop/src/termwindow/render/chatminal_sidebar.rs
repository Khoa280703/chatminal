use crate::chatminal_sidebar::{SidebarProfile, SidebarSession, SidebarSnapshot};
use crate::termwindow::box_model::{
    BorderColor, BoxDimension, Element, ElementColors, ElementContent,
};
use crate::termwindow::UIItemType;
use config::Dimension;
use window::color::LinearRgba;

impl crate::TermWindow {
    pub fn paint_chatminal_sidebar(&mut self) -> anyhow::Result<()> {
        if !self.chatminal_sidebar.is_enabled() {
            return Ok(());
        }
        let computed = self.build_chatminal_sidebar()?;
        let mut ui_items = computed.ui_items();
        self.ui_items.append(&mut ui_items);
        let gl_state = self.render_state.as_ref().unwrap();
        self.render_element(&computed, gl_state, None)
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
        let title_font = self.fonts.title_font()?;
        let body_font = self.fonts.default_font()?;
        let focused = self.focused.is_some();
        let window_frame = &self.config.window_frame;
        let frame_bg = if focused {
            window_frame.active_titlebar_bg.to_linear()
        } else {
            window_frame.inactive_titlebar_bg.to_linear()
        };
        let frame_fg = if focused {
            window_frame.active_titlebar_fg.to_linear()
        } else {
            window_frame.inactive_titlebar_fg.to_linear()
        };
        let palette = self.palette().clone();
        let separator = palette.split.to_linear();
        let active_bg = palette.selection_bg.to_linear();
        let active_fg = palette.selection_fg.to_linear();
        let hover_bg = palette.cursor_bg.to_linear();
        let hover_fg = palette.cursor_fg.to_linear();
        let muted_fg = palette.foreground.to_linear();
        let error_fg = LinearRgba::with_components(0.92, 0.38, 0.32, 1.0);

        let mut children = Vec::new();

        append_sidebar_body(
            &mut children,
            &title_font,
            &body_font,
            &snapshot,
            muted_fg,
            active_bg,
            active_fg,
            hover_bg,
            hover_fg,
            error_fg,
        );

        let root = Element::new(&body_font, ElementContent::Children(children))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .item_type(UIItemType::ChatminalSidebarBackground)
            .padding(BoxDimension {
                left: Dimension::Pixels(18.0),
                right: Dimension::Pixels(14.0),
                top: Dimension::Pixels(18.0),
                bottom: Dimension::Pixels(18.0),
            })
            .border(BoxDimension {
                left: Dimension::Pixels(0.0),
                right: Dimension::Pixels(1.0),
                top: Dimension::Pixels(0.0),
                bottom: Dimension::Pixels(0.0),
            })
            .colors(ElementColors {
                border: BorderColor::new(separator),
                bg: frame_bg.into(),
                text: frame_fg.into(),
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
}

fn append_sidebar_body(
    children: &mut Vec<Element>,
    title_font: &std::rc::Rc<engine_font::LoadedFont>,
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    snapshot: &SidebarSnapshot,
    muted_fg: LinearRgba,
    active_bg: LinearRgba,
    active_fg: LinearRgba,
    hover_bg: LinearRgba,
    hover_fg: LinearRgba,
    error_fg: LinearRgba,
) {
    if let Some(error) = &snapshot.error {
        children.push(
            Element::new(body_font, ElementContent::Text(error.clone()))
                .display(crate::termwindow::box_model::DisplayType::Block)
                .margin(BoxDimension::new(Dimension::Pixels(0.0)))
                .colors(text_colors(error_fg)),
        );
        return;
    }

    append_section_title(
        children,
        title_font,
        "Profiles",
        frame_heading_margin(0.0),
        muted_fg,
    );
    children.push(action_button(
        body_font,
        "+ New Profile",
        UIItemType::ChatminalSidebarCreateProfile,
        hover_bg,
        hover_fg,
    ));
    append_profiles(
        children,
        body_font,
        &snapshot.profiles,
        muted_fg,
        active_bg,
        active_fg,
        hover_bg,
        hover_fg,
    );
    children.push(spacer(body_font, 4.0));
    append_section_title(
        children,
        title_font,
        "Sessions",
        frame_heading_margin(6.0),
        muted_fg,
    );
    children.push(action_button(
        body_font,
        "+ New Session",
        UIItemType::ChatminalSidebarCreateSession,
        hover_bg,
        hover_fg,
    ));
    append_sessions(
        children,
        body_font,
        &snapshot.sessions,
        muted_fg,
        active_bg,
        active_fg,
        hover_bg,
        hover_fg,
    );
}

fn append_profiles(
    children: &mut Vec<Element>,
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    profiles: &[SidebarProfile],
    muted_fg: LinearRgba,
    active_bg: LinearRgba,
    active_fg: LinearRgba,
    hover_bg: LinearRgba,
    hover_fg: LinearRgba,
) {
    let max_rows = 8usize;
    for profile in profiles.iter().take(max_rows) {
        children.push(sidebar_entry(
            body_font,
            &profile.name,
            UIItemType::ChatminalSidebarProfile(profile.profile_id.clone()),
            profile.is_active,
            muted_fg,
            active_bg,
            active_fg,
            hover_bg,
            hover_fg,
        ));
    }
    if profiles.is_empty() {
        children.push(empty_hint(body_font, "No profiles", muted_fg));
    } else if profiles.len() > max_rows {
        children.push(counter_hint(
            body_font,
            format!("+{} profiles", profiles.len() - max_rows),
            muted_fg,
        ));
    }
}

fn append_sessions(
    children: &mut Vec<Element>,
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    sessions: &[SidebarSession],
    muted_fg: LinearRgba,
    active_bg: LinearRgba,
    active_fg: LinearRgba,
    hover_bg: LinearRgba,
    hover_fg: LinearRgba,
) {
    let max_rows = 12usize;
    for session in sessions.iter().take(max_rows) {
        let label = format!("{}  {}", session.name, session.status);
        children.push(sidebar_entry(
            body_font,
            &label,
            UIItemType::ChatminalSidebarSession(session.session_id.clone()),
            session.is_active,
            muted_fg,
            active_bg,
            active_fg,
            hover_bg,
            hover_fg,
        ));
    }
    if sessions.is_empty() {
        children.push(empty_hint(body_font, "No sessions", muted_fg));
    } else if sessions.len() > max_rows {
        children.push(counter_hint(
            body_font,
            format!("+{} sessions", sessions.len() - max_rows),
            muted_fg,
        ));
    }
}

fn append_section_title(
    children: &mut Vec<Element>,
    title_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    margin: BoxDimension,
    text: LinearRgba,
) {
    children.push(
        Element::new(title_font, ElementContent::Text(label.to_string()))
            .display(crate::termwindow::box_model::DisplayType::Block)
            .margin(margin)
            .colors(text_colors(text)),
    );
}

fn action_button(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    item_type: UIItemType,
    hover_bg: LinearRgba,
    hover_fg: LinearRgba,
) -> Element {
    Element::new(body_font, ElementContent::Text(label.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .item_type(item_type)
        .padding(entry_padding())
        .margin(entry_margin())
        .colors(filled_colors(
            LinearRgba::with_components(0.0, 0.0, 0.0, 0.0),
            hover_fg,
        ))
        .hover_colors(Some(filled_colors(hover_bg, hover_fg)))
}

fn sidebar_entry(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    item_type: UIItemType,
    is_active: bool,
    muted_fg: LinearRgba,
    active_bg: LinearRgba,
    active_fg: LinearRgba,
    hover_bg: LinearRgba,
    hover_fg: LinearRgba,
) -> Element {
    let (colors, hover_colors) = if is_active {
        (
            filled_colors(active_bg, active_fg),
            Some(filled_colors(hover_bg, hover_fg)),
        )
    } else {
        (
            filled_colors(LinearRgba::with_components(0.0, 0.0, 0.0, 0.0), muted_fg),
            Some(filled_colors(hover_bg, hover_fg)),
        )
    };
    Element::new(body_font, ElementContent::Text(label.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .item_type(item_type)
        .padding(entry_padding())
        .margin(entry_margin())
        .colors(colors)
        .hover_colors(hover_colors)
}

fn empty_hint(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: &str,
    muted_fg: LinearRgba,
) -> Element {
    Element::new(body_font, ElementContent::Text(label.to_string()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .colors(text_colors(muted_fg))
}

fn counter_hint(
    body_font: &std::rc::Rc<engine_font::LoadedFont>,
    label: String,
    muted_fg: LinearRgba,
) -> Element {
    Element::new(body_font, ElementContent::Text(label))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .margin(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(4.0),
            bottom: Dimension::Pixels(0.0),
        })
        .colors(text_colors(muted_fg))
}

fn spacer(body_font: &std::rc::Rc<engine_font::LoadedFont>, bottom: f32) -> Element {
    Element::new(body_font, ElementContent::Text(String::new()))
        .display(crate::termwindow::box_model::DisplayType::Block)
        .margin(BoxDimension {
            left: Dimension::Pixels(0.0),
            right: Dimension::Pixels(0.0),
            top: Dimension::Pixels(0.0),
            bottom: Dimension::Pixels(bottom),
        })
        .colors(text_colors(LinearRgba::with_components(0.0, 0.0, 0.0, 0.0)))
}

fn entry_padding() -> BoxDimension {
    BoxDimension {
        left: Dimension::Pixels(10.0),
        right: Dimension::Pixels(10.0),
        top: Dimension::Pixels(8.0),
        bottom: Dimension::Pixels(8.0),
    }
}

fn entry_margin() -> BoxDimension {
    BoxDimension {
        left: Dimension::Pixels(0.0),
        right: Dimension::Pixels(0.0),
        top: Dimension::Pixels(0.0),
        bottom: Dimension::Pixels(6.0),
    }
}

fn frame_heading_margin(top: f32) -> BoxDimension {
    BoxDimension {
        left: Dimension::Pixels(0.0),
        right: Dimension::Pixels(0.0),
        top: Dimension::Pixels(top),
        bottom: Dimension::Pixels(10.0),
    }
}

fn text_colors(text: LinearRgba) -> ElementColors {
    ElementColors {
        border: BorderColor::default(),
        bg: LinearRgba::with_components(0.0, 0.0, 0.0, 0.0).into(),
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
