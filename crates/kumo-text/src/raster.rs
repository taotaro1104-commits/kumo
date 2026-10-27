//! グリフのラスタライズと、全タブ共通のグリフキャッシュ（バイト数上限付きLRU）。

use std::collections::HashMap;

use parley::FontData;
use swash::scale::image::Content;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::{Format, Vector};

/// グリフキャッシュの上限（設計書：全タブ共通で4MB）。
pub const GLYPH_CACHE_BUDGET: usize = 4 * 1024 * 1024;

/// サブピクセル位置の量子化段階数（横方向のみ）。
const SUBPIXEL_STEPS: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_id: u64,
    pub font_index: u32,
    pub glyph_id: u32,
    /// フォントサイズ（1/64 px 単位）
    pub size: u32,
    /// 量子化したサブピクセルオフセット
    pub subpixel: u8,
}

/// ラスタ化済みグリフ。`left`/`top` はペン位置（ベースライン）からのオフセット。
#[derive(Clone, Debug)]
pub struct GlyphBitmap {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    /// `color == false` なら 8bit アルファ、`true` なら RGBA（プリマルチプライ前）
    pub data: Vec<u8>,
    pub color: bool,
}

struct Entry {
    bitmap: Option<GlyphBitmap>,
    last_used: u64,
}

/// グリフキャッシュ。上限を超えたら最も古いものから捨てる。
pub struct GlyphCache {
    scaler: ScaleContext,
    entries: HashMap<GlyphKey, Entry>,
    bytes: usize,
    budget: usize,
    tick: u64,
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new(GLYPH_CACHE_BUDGET)
    }
}

impl GlyphCache {
    pub fn new(budget: usize) -> Self {
        Self {
            scaler: ScaleContext::new(),
            entries: HashMap::new(),
            bytes: 0,
            budget,
            tick: 0,
        }
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// サブピクセル位置 `x` を量子化し、整数部とキー用の段階を返す。
    pub fn quantize_x(x: f32) -> (i32, u8) {
        let floor = x.floor();
        let step = ((x - floor) * SUBPIXEL_STEPS).round();
        if step >= SUBPIXEL_STEPS {
            (floor as i32 + 1, 0)
        } else {
            (floor as i32, step as u8)
        }
    }

    /// グリフを取得（なければラスタライズ）。空白などビットマップのないグリフは `None`。
    pub fn get(
        &mut self,
        font: &FontData,
        size: f32,
        glyph_id: u32,
        subpixel: u8,
        coords: &[i16],
    ) -> Option<&GlyphBitmap> {
        self.tick += 1;
        let key = GlyphKey {
            font_id: font.data.id(),
            font_index: font.index,
            glyph_id,
            size: (size * 64.0).round() as u32,
            subpixel,
        };
        if !self.entries.contains_key(&key) {
            let bitmap = self.rasterize(font, size, glyph_id, subpixel, coords);
            self.bytes += bitmap.as_ref().map_or(0, |b| b.data.len());
            self.entries.insert(
                key,
                Entry {
                    bitmap,
                    last_used: 0,
                },
            );
            self.evict();
        }
        let entry = self.entries.get_mut(&key)?;
        entry.last_used = self.tick;
        entry.bitmap.as_ref()
    }

    fn rasterize(
        &mut self,
        font: &FontData,
        size: f32,
        glyph_id: u32,
        subpixel: u8,
        coords: &[i16],
    ) -> Option<GlyphBitmap> {
        let font_ref = swash::FontRef::from_index(font.data.as_ref(), font.index as usize)?;
        let mut scaler = self
            .scaler
            .builder(font_ref)
            .size(size)
            .hint(true)
            .normalized_coords(coords.iter())
            .build();
        let offset = Vector::new(f32::from(subpixel) / SUBPIXEL_STEPS, 0.0);
        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .offset(offset)
        .render(&mut scaler, u16::try_from(glyph_id).ok()?)?;
        if image.placement.width == 0 || image.placement.height == 0 {
            return None;
        }
        Some(GlyphBitmap {
            left: image.placement.left,
            top: image.placement.top,
            width: image.placement.width,
            height: image.placement.height,
            color: image.content == Content::Color,
            data: image.data,
        })
    }

    /// 上限を超えている間、最も古く使われた項目を捨てる（予算の3/4まで減らす）。
    fn evict(&mut self) {
        if self.bytes <= self.budget {
            return;
        }
        let mut by_age: Vec<(u64, GlyphKey)> = self
            .entries
            .iter()
            .map(|(k, e)| (e.last_used, *k))
            .collect();
        by_age.sort_unstable_by_key(|(age, _)| *age);
        let target = self.budget * 3 / 4;
        for (_, key) in by_age {
            if self.bytes <= target {
                break;
            }
            if let Some(entry) = self.entries.remove(&key) {
                self.bytes -= entry.bitmap.map_or(0, |b| b.data.len());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantizes_subpixel_positions() {
        assert_eq!(GlyphCache::quantize_x(10.0), (10, 0));
        assert_eq!(GlyphCache::quantize_x(10.26), (10, 1));
        assert_eq!(GlyphCache::quantize_x(10.9), (11, 0));
        assert_eq!(GlyphCache::quantize_x(-0.5), (-1, 2));
    }
}
