use anyhow::{anyhow, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LucideIcon {
    ChevronDown,
    ChevronRight,
    Folder,
    FolderOpen,
    Plus,
    Settings,
    SquareTerminal,
}

impl LucideIcon {
    pub fn cache_key(self, size: u32) -> String {
        format!("lucide:{}:{size}", self.slug())
    }

    fn slug(self) -> &'static str {
        match self {
            Self::ChevronDown => "chevron-down",
            Self::ChevronRight => "chevron-right",
            Self::Folder => "folder",
            Self::FolderOpen => "folder-open",
            Self::Plus => "plus",
            Self::Settings => "settings",
            Self::SquareTerminal => "square-terminal",
        }
    }

    fn svg_bytes(self) -> &'static [u8] {
        match self {
            Self::ChevronDown => include_bytes!("../assets/icons/lucide/chevron-down.svg"),
            Self::ChevronRight => include_bytes!("../assets/icons/lucide/chevron-right.svg"),
            Self::Folder => include_bytes!("../assets/icons/lucide/folder.svg"),
            Self::FolderOpen => include_bytes!("../assets/icons/lucide/folder-open.svg"),
            Self::Plus => include_bytes!("../assets/icons/lucide/plus.svg"),
            Self::Settings => include_bytes!("../assets/icons/lucide/settings.svg"),
            Self::SquareTerminal => include_bytes!("../assets/icons/lucide/square-terminal.svg"),
        }
    }
}

pub fn rasterize_icon_mask(icon: LucideIcon, size: u32) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    let icon_size = size.max(1);
    let mut pixmap =
        tiny_skia::Pixmap::new(icon_size, icon_size).ok_or_else(|| anyhow!("invalid icon size"))?;
    let tree = usvg::Tree::from_data(icon.svg_bytes(), &usvg::Options::default())
        .with_context(|| format!("parse lucide icon {}", icon.slug()))?;

    let source = tree.size();
    let padding = 0.0;
    let target = ((icon_size as f32) - padding * 2.0).max(1.0);
    let scale = (target / source.width()).min(target / source.height());
    let tx = ((icon_size as f32) - source.width() * scale) * 0.5;
    let ty = ((icon_size as f32) - source.height() * scale) * 0.5;
    let transform = tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, tx, ty);

    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(&tree, transform, &mut pixmap_mut);

    for rgba in pixmap.data_mut().chunks_exact_mut(4) {
        let alpha = rgba[3];
        rgba[0] = alpha;
        rgba[1] = alpha;
        rgba[2] = alpha;
    }

    crop_mask_to_alpha_bounds(pixmap.take(), icon_size as usize, icon_size as usize)
}

fn crop_mask_to_alpha_bounds(
    rgba: Vec<u8>,
    width: usize,
    height: usize,
) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut found = false;

    for y in 0..height {
        for x in 0..width {
            let alpha = rgba[(y * width + x) * 4 + 3];
            if alpha == 0 {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    if !found {
        return Ok((rgba, width, height));
    }

    let cropped_width = max_x - min_x + 1;
    let cropped_height = max_y - min_y + 1;
    let mut cropped = vec![0u8; cropped_width * cropped_height * 4];

    for y in 0..cropped_height {
        let src_start = ((min_y + y) * width + min_x) * 4;
        let src_end = src_start + cropped_width * 4;
        let dst_start = y * cropped_width * 4;
        let dst_end = dst_start + cropped_width * 4;
        cropped[dst_start..dst_end].copy_from_slice(&rgba[src_start..src_end]);
    }

    Ok((cropped, cropped_width, cropped_height))
}
