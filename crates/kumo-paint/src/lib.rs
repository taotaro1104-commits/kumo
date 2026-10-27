//! 描画：ボックスツリー → ディスプレイリスト → tiny-skia タイル。
#![forbid(unsafe_code)]

mod builder;
mod display_list;
mod raster;
mod scroll;
mod tiles;

pub use builder::build_display_list;
pub use display_list::{DisplayEntry, DisplayItem, DisplayList, GlyphRunItem, PlacedGlyph, RectF};
pub use raster::{CpuRasterizer, ImageBitmap, ImageProvider, NoImageProvider, Rasterizer};
pub use scroll::{DamagedRows, STRIP_ROWS, ScrollRenderer};
pub use tiles::{TILE_BUDGET, TILE_SIZE, TileCache};

/// 文書の一部（`region`）を1枚の Pixmap に描く（ヘッドレス出力用）。
pub fn render_region(
    list: &DisplayList,
    raster: &mut dyn Rasterizer,
    images: &mut dyn ImageProvider,
    region: RectF,
) -> Option<tiny_skia::Pixmap> {
    let mut pixmap = tiny_skia::Pixmap::new(region.w.ceil() as u32, region.h.ceil() as u32)?;
    raster.render(list, region, &mut pixmap, images);
    Some(pixmap)
}
