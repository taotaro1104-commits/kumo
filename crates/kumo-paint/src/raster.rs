//! ディスプレイリストのラスタライズ（tiny-skia、CPU）。

use kumo_css::values::Rgba;
use kumo_text::GlyphCache;
use tiny_skia::{FilterQuality, Paint, Pixmap, PixmapPaint, PixmapRef, Rect, Transform};

use crate::display_list::{DisplayItem, DisplayList, GlyphRunItem, RectF};

/// 描画済み画像（プリマルチプライ済み RGBA8）。
pub struct ImageBitmap {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// 画像のデコード済みビットマップを提供する（表示サイズでデコードする実装を想定）。
pub trait ImageProvider {
    fn bitmap(&mut self, src: &str, width: u32, height: u32) -> Option<&ImageBitmap>;
}

/// 画像なし。
pub struct NoImageProvider;

impl ImageProvider for NoImageProvider {
    fn bitmap(&mut self, _src: &str, _w: u32, _h: u32) -> Option<&ImageBitmap> {
        None
    }
}

/// ラスタライザの抽象（将来 Vello 等に差し替える）。
pub trait Rasterizer {
    /// 文書座標 `region` の範囲を `out`（region と同じ大きさ）に描く。
    fn render(
        &mut self,
        list: &DisplayList,
        region: RectF,
        out: &mut Pixmap,
        images: &mut dyn ImageProvider,
    );
}

/// tiny-skia による CPU ラスタライザ。グリフキャッシュを持つ。
#[derive(Default)]
pub struct CpuRasterizer {
    pub glyphs: GlyphCache,
}

fn to_color(c: Rgba) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

impl Rasterizer for CpuRasterizer {
    fn render(
        &mut self,
        list: &DisplayList,
        region: RectF,
        out: &mut Pixmap,
        images: &mut dyn ImageProvider,
    ) {
        out.fill(to_color(list.background.unwrap_or(Rgba::WHITE)));
        for entry in list.items_in(&region) {
            let clip = entry.clip.map(|c| c.translate(-region.x, -region.y));
            match &entry.item {
                DisplayItem::Fill { rect, color } => {
                    fill(out, rect.translate(-region.x, -region.y), *color, clip);
                }
                DisplayItem::Glyphs(run) => {
                    self.draw_glyphs(out, run, region, clip);
                }
                DisplayItem::Image { rect, src } => {
                    let dest = rect.translate(-region.x, -region.y);
                    let (w, h) = (rect.w.round() as u32, rect.h.round() as u32);
                    if let Some(bitmap) = images.bitmap(src, w.max(1), h.max(1)) {
                        draw_image(out, bitmap, dest, clip);
                    }
                }
            }
        }
    }
}

fn fill(out: &mut Pixmap, rect: RectF, color: Rgba, clip: Option<RectF>) {
    let rect = clip.map_or(rect, |c| rect.intersect(&c));
    let Some(r) = Rect::from_xywh(rect.x, rect.y, rect.w, rect.h) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color(to_color(color));
    paint.anti_alias = false;
    out.fill_rect(r, &paint, Transform::identity(), None);
}

fn draw_image(out: &mut Pixmap, bitmap: &ImageBitmap, dest: RectF, clip: Option<RectF>) {
    let Some(src) = PixmapRef::from_bytes(&bitmap.data, bitmap.width, bitmap.height) else {
        return;
    };
    let visible = clip.map_or(dest, |c| dest.intersect(&c));
    if visible.is_empty() {
        return;
    }
    let scale_x = dest.w / bitmap.width as f32;
    let scale_y = dest.h / bitmap.height as f32;
    let paint = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..PixmapPaint::default()
    };
    let transform = Transform::from_row(scale_x, 0.0, 0.0, scale_y, dest.x, dest.y);
    let mask = (visible != dest).then(|| {
        let mut mask = tiny_skia::Mask::new(out.width(), out.height())?;
        let path = tiny_skia::PathBuilder::from_rect(Rect::from_xywh(
            visible.x, visible.y, visible.w, visible.h,
        )?);
        mask.fill_path(
            &path,
            tiny_skia::FillRule::Winding,
            false,
            Transform::identity(),
        );
        Some(mask)
    });
    out.draw_pixmap(0, 0, src, &paint, transform, mask.flatten().as_ref());
}

impl CpuRasterizer {
    fn draw_glyphs(
        &mut self,
        out: &mut Pixmap,
        run: &GlyphRunItem,
        region: RectF,
        clip: Option<RectF>,
    ) {
        let (width, height) = (out.width() as i32, out.height() as i32);
        let bounds = clip.unwrap_or(RectF::new(0.0, 0.0, width as f32, height as f32));
        let (cx0, cy0) = (bounds.x.max(0.0) as i32, bounds.y.max(0.0) as i32);
        let (cx1, cy1) = (
            (bounds.right().ceil() as i32).min(width),
            (bounds.bottom().ceil() as i32).min(height),
        );
        for glyph in &run.glyphs {
            let (gx, subpixel) = GlyphCache::quantize_x(glyph.x - region.x);
            let gy = (glyph.y - region.y).round() as i32;
            let Some(bitmap) =
                self.glyphs
                    .get(&run.font, run.size, glyph.id, subpixel, &run.coords)
            else {
                continue;
            };
            let x0 = gx + bitmap.left;
            let y0 = gy - bitmap.top;
            blend_mask(out, bitmap, x0, y0, run.color, (cx0, cy0, cx1, cy1));
        }
    }
}

/// グリフのアルファマスク（またはカラー絵文字）をプリマルチプライ済みピクセルへ合成する。
fn blend_mask(
    out: &mut Pixmap,
    bitmap: &kumo_text::GlyphBitmap,
    x0: i32,
    y0: i32,
    color: Rgba,
    (cx0, cy0, cx1, cy1): (i32, i32, i32, i32),
) {
    let stride = out.width() as usize;
    let pixels = out.data_mut();
    let bpp = if bitmap.color { 4 } else { 1 };
    for row in 0..bitmap.height as i32 {
        let y = y0 + row;
        if y < cy0 || y >= cy1 {
            continue;
        }
        for col in 0..bitmap.width as i32 {
            let x = x0 + col;
            if x < cx0 || x >= cx1 {
                continue;
            }
            let src = (row as usize * bitmap.width as usize + col as usize) * bpp;
            let (r, g, b, a) = if bitmap.color {
                let p = &bitmap.data[src..src + 4];
                (p[0], p[1], p[2], p[3])
            } else {
                let coverage = u32::from(bitmap.data[src]);
                (
                    color.r,
                    color.g,
                    color.b,
                    (coverage * u32::from(color.a) / 255) as u8,
                )
            };
            if a == 0 {
                continue;
            }
            let dst = (y as usize * stride + x as usize) * 4;
            let a32 = u32::from(a);
            let inv = 255 - a32;
            let px = &mut pixels[dst..dst + 4];
            px[0] = ((u32::from(r) * a32 + u32::from(px[0]) * inv) / 255) as u8;
            px[1] = ((u32::from(g) * a32 + u32::from(px[1]) * inv) / 255) as u8;
            px[2] = ((u32::from(b) * a32 + u32::from(px[2]) * inv) / 255) as u8;
            px[3] = (a32 + u32::from(px[3]) * inv / 255) as u8;
        }
    }
}
