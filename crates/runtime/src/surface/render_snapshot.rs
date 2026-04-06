use dynamic::{FromDynamic, ToDynamic};
use luahelper::impl_lua_conversion_dynamic;
use serde::{Deserialize, Serialize};
use terminal_emulator::StableRowIndex;

/// Describes the location of the cursor in stable-row space.
#[derive(
    Debug, Default, Copy, Clone, Hash, Eq, PartialEq, Deserialize, Serialize, FromDynamic, ToDynamic,
)]
pub struct StableCursorPosition {
    pub x: usize,
    pub y: StableRowIndex,
    pub shape: termwiz::surface::CursorShape,
    pub visibility: termwiz::surface::CursorVisibility,
}
impl_lua_conversion_dynamic!(StableCursorPosition);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, FromDynamic, ToDynamic,
)]
pub struct RenderableDimensions {
    pub cols: usize,
    pub viewport_rows: usize,
    pub scrollback_rows: usize,
    pub physical_top: StableRowIndex,
    pub scrollback_top: StableRowIndex,
    pub dpi: u32,
    pub pixel_width: usize,
    pub pixel_height: usize,
    pub reverse_video: bool,
}
impl_lua_conversion_dynamic!(RenderableDimensions);
