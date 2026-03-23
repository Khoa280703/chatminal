use crate::desktop_termwindow_types::{TerminalSplit, TerminalSplitDirection};
use crate::chatminal_runtime::overlay_compat::OverlayPane;
use crate::termwindow::render::TripleLayerQuadAllocator;
use crate::termwindow::{UIItem, UIItemType};
use std::sync::Arc;

impl crate::TermWindow {
    pub fn paint_split(
        &mut self,
        layers: &mut TripleLayerQuadAllocator,
        split: &TerminalSplit,
        pane: &Arc<dyn OverlayPane>,
    ) -> anyhow::Result<()> {
        let palette = pane.palette();
        let foreground = palette.split.to_linear();
        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;
        let grid_origin = self.terminal_grid_origin();

        let pos_y = split.top as f32 * cell_height + grid_origin.y;
        let pos_x = split.left as f32 * cell_width + grid_origin.x;

        if split.direction == TerminalSplitDirection::Horizontal {
            self.filled_rectangle(
                layers,
                2,
                euclid::rect(
                    pos_x + (cell_width / 2.0),
                    pos_y - (cell_height / 2.0),
                    self.render_metrics.underline_height as f32,
                    (1. + split.size as f32) * cell_height,
                ),
                foreground,
            )?;
            self.ui_items.push(UIItem {
                x: grid_origin.x as usize + (split.left * cell_width as usize),
                width: cell_width as usize,
                y: grid_origin.y as usize + split.top * cell_height as usize,
                height: split.size * cell_height as usize,
                item_type: UIItemType::Split(split.clone()),
            });
        } else {
            self.filled_rectangle(
                layers,
                2,
                euclid::rect(
                    pos_x - (cell_width / 2.0),
                    pos_y + (cell_height / 2.0),
                    (1.0 + split.size as f32) * cell_width,
                    self.render_metrics.underline_height as f32,
                ),
                foreground,
            )?;
            self.ui_items.push(UIItem {
                x: grid_origin.x as usize + (split.left * cell_width as usize),
                width: split.size * cell_width as usize,
                y: grid_origin.y as usize + split.top * cell_height as usize,
                height: cell_height as usize,
                item_type: UIItemType::Split(split.clone()),
            });
        }

        Ok(())
    }
}
