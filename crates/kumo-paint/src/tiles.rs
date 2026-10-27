//! タイルキャッシュ。ビューポートと前後の先読み分だけラスタタイルを保持する
//! （設計書：1080p で約12MB 上限）。

use std::collections::HashMap;

use tiny_skia::Pixmap;

use crate::display_list::{DisplayList, RectF};
use crate::raster::{ImageProvider, Rasterizer};

pub const TILE_SIZE: u32 = 256;
/// ラスタタイルの予算（設計書の値）。
pub const TILE_BUDGET: usize = 12 * 1024 * 1024;
const TILE_BYTES: usize = (TILE_SIZE * TILE_SIZE * 4) as usize;

type TileKey = (i32, i32);

pub struct TileCache {
    tiles: HashMap<TileKey, Pixmap>,
    budget: usize,
}

impl Default for TileCache {
    fn default() -> Self {
        Self::new(TILE_BUDGET)
    }
}

impl TileCache {
    pub fn new(budget: usize) -> Self {
        Self {
            tiles: HashMap::new(),
            budget,
        }
    }

    /// ビューポートの1.5画面分を予算にする（上限は `TILE_BUDGET`）。
    /// 先読みを半画面に抑え、小さな画面ほど少ないメモリで済ませる。
    pub fn for_viewport(width: u32, height: u32) -> Self {
        let screen = width as usize * height as usize * 4;
        Self::new((screen * 3 / 2).clamp(TILE_BYTES * 2, TILE_BUDGET))
    }

    pub fn bytes(&self) -> usize {
        self.tiles.len() * TILE_BYTES
    }

    /// レイアウトが変わったら全タイルを捨てる。
    pub fn invalidate(&mut self) {
        self.tiles.clear();
    }

    /// ビューポート（文書座標）を `out`（0RGB の u32、幅 `vw`）に合成する。
    pub fn compose(
        &mut self,
        list: &DisplayList,
        raster: &mut dyn Rasterizer,
        images: &mut dyn ImageProvider,
        viewport: RectF,
        out: &mut [u32],
    ) {
        let needed = tile_range(viewport);
        for &key in &needed {
            self.ensure(key, list, raster, images);
        }
        let vw = viewport.w as usize;
        for &key in &needed {
            let tile = &self.tiles[&key];
            blit(tile, key, viewport, vw, out);
        }
        self.evict_and_prefetch(viewport, list, raster, images);
    }

    fn ensure(
        &mut self,
        key: TileKey,
        list: &DisplayList,
        raster: &mut dyn Rasterizer,
        images: &mut dyn ImageProvider,
    ) {
        if self.tiles.contains_key(&key) {
            return;
        }
        let Some(mut pixmap) = Pixmap::new(TILE_SIZE, TILE_SIZE) else {
            return;
        };
        let size = TILE_SIZE as f32;
        let region = RectF::new(key.0 as f32 * size, key.1 as f32 * size, size, size);
        raster.render(list, region, &mut pixmap, images);
        self.tiles.insert(key, pixmap);
    }

    /// 予算内で上下1画面分を先読みし、遠いタイルから捨てる。
    fn evict_and_prefetch(
        &mut self,
        viewport: RectF,
        list: &DisplayList,
        raster: &mut dyn Rasterizer,
        images: &mut dyn ImageProvider,
    ) {
        let center_y = viewport.y + viewport.h / 2.0;
        let distance = |key: &TileKey| {
            let tile_center = (key.1 as f32 + 0.5) * TILE_SIZE as f32;
            (tile_center - center_y).abs()
        };
        let mut keys: Vec<TileKey> = self.tiles.keys().copied().collect();
        keys.sort_by(|a, b| distance(a).total_cmp(&distance(b)));
        let max_tiles = (self.budget / TILE_BYTES).max(1);
        for key in keys.iter().skip(max_tiles) {
            self.tiles.remove(key);
        }
        let doc_bottom = list.height.max(viewport.bottom());
        for dy in [viewport.h, -viewport.h] {
            let ahead = viewport.translate(0.0, dy);
            if ahead.bottom() <= 0.0 || ahead.y >= doc_bottom {
                continue;
            }
            for key in tile_range(ahead) {
                if self.tiles.len() >= max_tiles {
                    return;
                }
                self.ensure(key, list, raster, images);
            }
        }
    }
}

fn tile_range(viewport: RectF) -> Vec<TileKey> {
    let size = TILE_SIZE as f32;
    let (x0, y0) = (
        (viewport.x / size).floor() as i32,
        (viewport.y / size).floor() as i32,
    );
    let (x1, y1) = (
        ((viewport.right() - 1.0) / size).floor() as i32,
        ((viewport.bottom() - 1.0) / size).floor() as i32,
    );
    (y0..=y1)
        .flat_map(|ty| (x0..=x1).map(move |tx| (tx, ty)))
        .collect()
}

/// タイルの重なり部分を出力バッファへコピーする（RGBA → 0RGB）。
fn blit(tile: &Pixmap, key: TileKey, viewport: RectF, vw: usize, out: &mut [u32]) {
    let size = TILE_SIZE as i32;
    let (tx, ty) = (key.0 * size, key.1 * size);
    let (vx, vy) = (viewport.x as i32, viewport.y as i32);
    let vh = out.len() / vw.max(1);
    let data = tile.data();
    for row in 0..size {
        let y = ty + row - vy;
        if y < 0 || y as usize >= vh {
            continue;
        }
        for col in 0..size {
            let x = tx + col - vx;
            if x < 0 || x as usize >= vw {
                continue;
            }
            let src = ((row * size + col) * 4) as usize;
            let (r, g, b) = (data[src], data[src + 1], data[src + 2]);
            out[y as usize * vw + x as usize] =
                (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_range_covers_viewport() {
        let keys = tile_range(RectF::new(0.0, 300.0, 512.0, 256.0));
        assert_eq!(keys, vec![(0, 1), (1, 1), (0, 2), (1, 2)]);
    }
}
