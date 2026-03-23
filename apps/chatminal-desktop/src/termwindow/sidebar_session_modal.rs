use crate::termwindow::box_model::*;
use crate::termwindow::modal::Modal;
use crate::TermWindow;
use config::Dimension;
use engine_term::{KeyCode, KeyModifiers, MouseEvent};
use std::cell::{Ref, RefCell};
use window::color::LinearRgba;

enum SidebarSessionModalMode {
    Delete { session_id: String, session_name: String },
}

pub struct SidebarSessionModal {
    element: RefCell<Option<Vec<ComputedElement>>>,
    mode: SidebarSessionModalMode,
}

impl SidebarSessionModal {
    pub fn delete(session_id: String, session_name: String) -> Self {
        Self {
            element: RefCell::new(None),
            mode: SidebarSessionModalMode::Delete {
                session_id,
                session_name,
            },
        }
    }

    fn compute(&self, term_window: &mut TermWindow) -> anyhow::Result<Vec<ComputedElement>> {
        let font = term_window.fonts.command_palette_font()?;
        let metrics = crate::utilsprites::RenderMetrics::with_font_metrics(&font.metrics());
        let fg = LinearRgba::with_components(0.92, 0.92, 0.92, 1.0);
        let bg = LinearRgba::with_components(0.06, 0.06, 0.06, 0.995);
        let border = LinearRgba::with_components(0.18, 0.18, 0.18, 1.0);
        let hint = LinearRgba::with_components(0.68, 0.68, 0.68, 1.0);

        let (title, lines) = match &self.mode {
            SidebarSessionModalMode::Delete { session_name, .. } => (
                "Xoá session".to_string(),
                vec![
                    format!("Xoá \"{session_name}\"?"),
                    "Enter hoặc Y để xác nhận".to_string(),
                    "Esc hoặc N để huỷ".to_string(),
                ],
            ),
        };

        let mut children = vec![
            Element::new(&font, ElementContent::Text(title))
                .display(DisplayType::Block)
                .colors(ElementColors {
                    border: BorderColor::default(),
                    bg: LinearRgba::TRANSPARENT.into(),
                    text: fg.into(),
                }),
        ];
        for line in lines {
            children.push(
                Element::new(&font, ElementContent::Text(line))
                    .display(DisplayType::Block)
                    .margin(BoxDimension {
                        left: Dimension::Pixels(0.0),
                        right: Dimension::Pixels(0.0),
                        top: Dimension::Pixels(6.0),
                        bottom: Dimension::Pixels(0.0),
                    })
                    .colors(ElementColors {
                        border: BorderColor::default(),
                        bg: LinearRgba::TRANSPARENT.into(),
                        text: hint.into(),
                    }),
            );
        }

        let root = Element::new(&font, ElementContent::Children(children))
            .display(DisplayType::Block)
            .padding(BoxDimension::new(Dimension::Pixels(12.0)))
            .border(BoxDimension::new(Dimension::Pixels(1.0)))
            .colors(ElementColors {
                border: BorderColor::new(border),
                bg: bg.into(),
                text: fg.into(),
            })
            .min_width(Some(Dimension::Pixels(320.0)))
            .max_width(Some(Dimension::Pixels(320.0)));

        let width = 320.0;
        let height = 112.0;
        let x = ((term_window.dimensions.pixel_width as f32 - width) * 0.5).max(12.0);
        let y = ((term_window.dimensions.pixel_height as f32 - height) * 0.2).max(12.0);

        Ok(vec![term_window.compute_element(
            &LayoutContext {
                height: config::DimensionContext {
                    dpi: term_window.dimensions.dpi as f32,
                    pixel_max: term_window.dimensions.pixel_height as f32,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: config::DimensionContext {
                    dpi: term_window.dimensions.dpi as f32,
                    pixel_max: term_window.dimensions.pixel_width as f32,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(x, y, width, height),
                metrics: &metrics,
                gl_state: term_window.render_state.as_ref().unwrap(),
                zindex: 110,
            },
            &root,
        )?])
    }
}

impl Modal for SidebarSessionModal {
    fn mouse_event(&self, _event: MouseEvent, _term_window: &mut TermWindow) -> anyhow::Result<()> {
        Ok(())
    }

    fn key_down(
        &self,
        key: KeyCode,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool> {
        match &self.mode {
            SidebarSessionModalMode::Delete { session_id, .. } => match (key, mods) {
                (KeyCode::Escape, KeyModifiers::NONE)
                | (KeyCode::Char('g'), KeyModifiers::CTRL)
                | (KeyCode::Char('n' | 'N'), KeyModifiers::NONE)
                | (KeyCode::Char('n' | 'N'), KeyModifiers::SHIFT) => {
                    term_window.cancel_modal();
                }
                (KeyCode::Enter, KeyModifiers::NONE)
                | (KeyCode::Char('y' | 'Y'), KeyModifiers::NONE)
                | (KeyCode::Char('y' | 'Y'), KeyModifiers::SHIFT) => {
                    term_window.close_chatminal_session_by_id(session_id);
                    term_window.cancel_modal();
                }
                _ => return Ok(false),
            },
        }
        Ok(true)
    }

    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<Ref<'_, [ComputedElement]>> {
        if self.element.borrow().is_none() {
            self.element.borrow_mut().replace(self.compute(term_window)?);
        }
        Ok(Ref::map(self.element.borrow(), |v| v.as_ref().unwrap().as_slice()))
    }

    fn reconfigure(&self, _term_window: &mut TermWindow) {
        self.element.borrow_mut().take();
    }
}
