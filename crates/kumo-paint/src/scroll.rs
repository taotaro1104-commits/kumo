//! スクロール再利用レンダラ。前フレームの画素をずらして使い回し、新しく見えた帯だけを描く。
//!
//! タイルキャッシュ（画面1.5枚分）を持たず、作業領域は高さ `STRIP_ROWS` の帯1本だけで済む。
//! 低メモリ環境向けの既定の描画経路（タイルキャッシュはヘッドレスや将来の GPU 合成用に残す）。

use tiny_skia::Pixmap;

use crate::display_list::{DisplayList, RectF};
use crate::raster::{ImageProvider, Rasterizer};

/// 一度にラスタライズする帯の高さ（px）。
pub const STRIP_ROWS: u32 = 64;

/// 描き直した行範囲 `[start, end)`（ビューポート座標）。露出は最大1か所なので `Option` で返す。
pub type DamagedRows = std::ops::Range<usize>;

#[derive(Default)]
pub struct ScrollRenderer {
    strip: Option<Pixmap>,
    /// 前回描いたビューポート（整数化済み）
    last: Option<(i64, i64, usize, usize)>,
}

impl ScrollRenderer {
    /// レイアウトや画面サイズが変わったら呼ぶ（次回は全面を描く）。
    pub fn invalidate(&mut self) {
        self.last = None;
    }

    pub fn bytes(&self) -> usize {
        self.strip.as_ref().map_or(0, |p| p.data().len())
    }

    /// `out`（0RGB、幅 `viewport.w`）を `viewport` の内容に更新し、描き直した行範囲を返す。
    /// `buffer_valid` は `out` に前回の結果が残っているか（softbuffer の `age() > 0`）。
    pub fn render(
        &mut self,
        list: &DisplayList,
        raster: &mut dyn Rasterizer,
        images: &mut dyn ImageProvider,
        viewport: RectF,
        out: &mut [u32],
        buffer_valid: bool,
    ) -> Option<DamagedRows> {
        let (w, h) = (viewport.w as usize, viewport.h as usize);
        if w == 0 || h == 0 || out.len() < w * h {
            return None;
        }
        let current = (viewport.x.round() as i64, viewport.y.round() as i64, w, h);
        let damage = match self.last.filter(|_| buffer_valid) {
            Some((x, y, lw, lh)) if (x, lw, lh) == (current.0, w, h) => {
                shift_rows(out, w, h, current.1 - y)
            }
            _ => Some(0..h),
        };
        if let Some(rows) = &damage {
            self.draw_rows(list, raster, images, viewport, rows.clone(), out);
        }
        self.last = Some(current);
        damage
    }

    fn draw_rows(
        &mut self,
        list: &DisplayList,
        raster: &mut dyn Rasterizer,
        images: &mut dyn ImageProvider,
        viewport: RectF,
        rows: DamagedRows,
        out: &mut [u32],
    ) {
        let w = viewport.w as usize;
        let needs_new = self.strip.as_ref().is_none_or(|p| p.width() as usize != w);
        if needs_new {
            self.strip = Pixmap::new(w as u32, STRIP_ROWS);
        }
        let Some(strip) = self.strip.as_mut() else {
            return;
        };
        let mut y = rows.start;
        while y < rows.end {
            let count = (rows.end - y).min(STRIP_ROWS as usize);
            let region = RectF::new(
                viewport.x,
                viewport.y.round() + y as f32,
                w as f32,
                STRIP_ROWS as f32,
            );
            raster.render(list, region, strip, images);
            let data = strip.data();
            for row in 0..count {
                let dst = &mut out[(y + row) * w..(y + row + 1) * w];
                let src = &data[row * w * 4..(row + 1) * w * 4];
                for (px, rgba) in dst.iter_mut().zip(src.as_chunks::<4>().0) {
                    *px =
                        (u32::from(rgba[0]) << 16) | (u32::from(rgba[1]) << 8) | u32::from(rgba[2]);
                }
            }
            y += count;
        }
    }
}

/// 画面内容を `dy` 行ずらし（正なら下へスクロール＝内容は上へ）、露出した行範囲を返す。
fn shift_rows(out: &mut [u32], w: usize, h: usize, dy: i64) -> Option<DamagedRows> {
    let shift = dy.unsigned_abs() as usize;
    if dy == 0 {
        return None;
    }
    if shift >= h {
        return Some(0..h);
    }
    let keep = (h - shift) * w;
    if dy > 0 {
        out.copy_within(shift * w..h * w, 0);
        Some(h - shift..h)
    } else {
        out.copy_within(0..keep, shift * w);
        Some(0..shift)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_reports_exposed_rows() {
        let mut buf: Vec<u32> = (0..12).collect(); // 3x4
        assert_eq!(shift_rows(&mut buf, 3, 4, 1), Some(3..4));
        assert_eq!(&buf[..9], &[3, 4, 5, 6, 7, 8, 9, 10, 11]);
        let mut buf: Vec<u32> = (0..12).collect();
        assert_eq!(shift_rows(&mut buf, 3, 4, -2), Some(0..2));
        assert_eq!(&buf[6..], &[0, 1, 2, 3, 4, 5]);
        assert_eq!(shift_rows(&mut buf, 3, 4, 0), None);
        assert_eq!(shift_rows(&mut buf, 3, 4, 10), Some(0..4));
    }
}
