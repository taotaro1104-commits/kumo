//! ディスプレイリスト。レイアウト結果を描画命令の平坦な列にしたもの（座標は文書座標）。

use std::sync::Arc;

use kumo_css::values::Rgba;
use kumo_text::parley::FontData;

/// 浮動小数の矩形。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectF {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        Self::new(x, y, (r - x).max(0.0), (b - y).max(0.0))
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    pub fn translate(&self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    /// 原点を中心に `k` 倍する。
    pub fn scaled(&self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k, self.w * k, self.h * k)
    }
}

/// 位置決め済みグリフ（ベースライン上のペン位置）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

/// 同じフォント・サイズ・色のグリフ列。
#[derive(Clone, Debug)]
pub struct GlyphRunItem {
    pub font: FontData,
    pub size: f32,
    pub coords: Vec<i16>,
    pub glyphs: Vec<PlacedGlyph>,
    pub color: Rgba,
}

#[derive(Clone, Debug)]
pub enum DisplayItem {
    Fill {
        rect: RectF,
        color: Rgba,
    },
    /// 角の丸い背景（半径は左上・右上・右下・左下、px）
    RoundedFill {
        rect: RectF,
        color: Rgba,
        radii: [f32; 4],
    },
    /// 角の丸い枠線（全辺同じ色。`widths` は上・右・下・左）
    RoundedBorder {
        rect: RectF,
        widths: [f32; 4],
        color: Rgba,
        radii: [f32; 4],
    },
    Glyphs(Box<GlyphRunItem>),
    Image {
        rect: RectF,
        src: Arc<str>,
    },
}

/// 描画命令1つ。`bounds` はタイル選別用、`clip` は overflow によるクリップ。
#[derive(Clone, Debug)]
pub struct DisplayEntry {
    pub item: DisplayItem,
    pub bounds: RectF,
    pub clip: Option<RectF>,
}

#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    pub entries: Vec<DisplayEntry>,
    /// 文書（キャンバス）の大きさ
    pub width: f32,
    pub height: f32,
    /// キャンバス背景色（ルート要素/body から伝播）
    pub background: Option<Rgba>,
    /// クリックできるリンクの領域（描画順。後ろほど手前）
    pub links: Vec<LinkRegion>,
    /// ページ内リンクの飛び先（名前と文書上の y 座標）
    pub anchors: Vec<(Arc<str>, f32)>,
    /// フォーム部品の領域（DOM ノードのインデックス付き）
    pub controls: Vec<(RectF, usize)>,
    /// 要素の当たり判定の領域（描画順。後ろほど手前）。JS のイベント対象を決めるため、JS を使うページだけ記録する
    pub hits: Vec<(RectF, usize)>,
    /// 要素ごとの境界ボックス（文書座標、クリップなし）。`getBoundingClientRect` などの幾何情報用。`record_hits` のときだけ記録する
    pub geometry: Vec<(usize, RectF)>,
    /// `hits` を記録するか
    pub record_hits: bool,
}

/// リンクの当たり判定領域。`href` は未解決のまま。
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRegion {
    pub rect: RectF,
    pub href: Arc<str>,
}

impl DisplayList {
    /// 全体を `k` 倍する（ページの拡大・縮小）。レイアウトは `1/k` の幅で行い、できた描画命令をここで拡大する。
    pub fn scale(&mut self, k: f32) {
        let radii = |r: &mut [f32; 4]| r.iter_mut().for_each(|v| *v *= k);
        for entry in &mut self.entries {
            entry.bounds = entry.bounds.scaled(k);
            entry.clip = entry.clip.map(|c| c.scaled(k));
            match &mut entry.item {
                DisplayItem::Fill { rect, .. } | DisplayItem::Image { rect, .. } => {
                    *rect = rect.scaled(k);
                }
                DisplayItem::RoundedFill { rect, radii: r, .. } => {
                    *rect = rect.scaled(k);
                    radii(r);
                }
                DisplayItem::RoundedBorder {
                    rect,
                    widths,
                    radii: r,
                    ..
                } => {
                    *rect = rect.scaled(k);
                    radii(widths);
                    radii(r);
                }
                DisplayItem::Glyphs(run) => {
                    run.size *= k;
                    for g in &mut run.glyphs {
                        g.x *= k;
                        g.y *= k;
                    }
                }
            }
        }
        self.width *= k;
        self.height *= k;
        for link in &mut self.links {
            link.rect = link.rect.scaled(k);
        }
        for (_, y) in &mut self.anchors {
            *y *= k;
        }
        for (rect, _) in self.controls.iter_mut().chain(&mut self.hits) {
            *rect = rect.scaled(k);
        }
        for (_, rect) in &mut self.geometry {
            *rect = rect.scaled(k);
        }
    }

    /// リンク領域を追加する（クリップの外は除く）。
    pub fn add_link(&mut self, rect: RectF, href: &Arc<str>, clip: Option<RectF>) {
        let rect = clip.map_or(rect, |c| rect.intersect(&c));
        if rect.is_empty() {
            return;
        }
        // 同じリンクの隣り合う領域（同じ行の連続したグリフ列）はまとめる
        if let Some(last) = self.links.last_mut()
            && Arc::ptr_eq(&last.href, href)
            && (last.rect.y - rect.y).abs() < 0.5
            && (last.rect.h - rect.h).abs() < 0.5
            && (last.rect.right() - rect.x).abs() < 1.0
        {
            last.rect.w = rect.right() - last.rect.x;
            return;
        }
        self.links.push(LinkRegion {
            rect,
            href: href.clone(),
        });
    }

    /// 名前 `name` の飛び先の y 座標（最初に現れたもの）。
    pub fn anchor_y(&self, name: &str) -> Option<f32> {
        self.anchors
            .iter()
            .find(|(n, _)| &**n == name)
            .map(|&(_, y)| y)
    }

    /// 文書座標 (x, y) にある最も手前のフォーム部品（DOM ノードのインデックス）。
    pub fn control_at(&self, x: f32, y: f32) -> Option<usize> {
        self.controls
            .iter()
            .rev()
            .find(|(r, _)| x >= r.x && x < r.right() && y >= r.y && y < r.bottom())
            .map(|&(_, node)| node)
    }

    /// 文書座標 (x, y) にある最も手前の要素（DOM ノードのインデックス）。
    pub fn hit_at(&self, x: f32, y: f32) -> Option<usize> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| x >= r.x && x < r.right() && y >= r.y && y < r.bottom())
            .map(|&(_, node)| node)
    }

    /// 要素の境界ボックスを記録する（同じ要素の複数のボックスは範囲を合わせる）。
    pub fn add_geometry(&mut self, node: usize, rect: RectF) {
        match self
            .geometry
            .iter_mut()
            .rev()
            .take(8)
            .find(|(n, _)| *n == node)
        {
            Some((_, existing)) => {
                let x = existing.x.min(rect.x);
                let y = existing.y.min(rect.y);
                let r = existing.right().max(rect.right());
                let b = existing.bottom().max(rect.bottom());
                *existing = RectF::new(x, y, r - x, b - y);
            }
            None => self.geometry.push((node, rect)),
        }
    }

    /// 当たり判定の領域を記録する（`record_hits` のときだけ）。
    pub fn add_hit(&mut self, rect: RectF, node: usize, clip: Option<RectF>) {
        if !self.record_hits {
            return;
        }
        let rect = clip.map_or(rect, |c| rect.intersect(&c));
        if !rect.is_empty() {
            self.hits.push((rect, node));
        }
    }

    /// 文書座標 (x, y) にある最も手前のリンク。
    pub fn link_at(&self, x: f32, y: f32) -> Option<&Arc<str>> {
        self.links
            .iter()
            .rev()
            .find(|l| x >= l.rect.x && x < l.rect.right() && y >= l.rect.y && y < l.rect.bottom())
            .map(|l| &l.href)
    }

    pub fn push(&mut self, item: DisplayItem, bounds: RectF, clip: Option<RectF>) {
        let visible = match clip {
            Some(c) => bounds.intersect(&c),
            None => bounds,
        };
        if visible.is_empty() {
            return;
        }
        self.entries.push(DisplayEntry { item, bounds, clip });
    }

    /// 指定領域にかかる命令を順に返す。
    pub fn items_in<'a>(
        &'a self,
        region: &'a RectF,
    ) -> impl Iterator<Item = &'a DisplayEntry> + 'a {
        self.entries
            .iter()
            .filter(move |e| e.bounds.intersects(region))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_math() {
        let a = RectF::new(0.0, 0.0, 10.0, 10.0);
        let b = RectF::new(5.0, 5.0, 10.0, 10.0);
        assert_eq!(a.intersect(&b), RectF::new(5.0, 5.0, 5.0, 5.0));
        assert!(a.intersects(&b));
        assert!(!a.intersects(&RectF::new(10.0, 0.0, 5.0, 5.0)));
    }

    #[test]
    fn scaling_moves_and_resizes_everything() {
        let mut list = DisplayList {
            width: 100.0,
            height: 200.0,
            ..DisplayList::default()
        };
        let rect = RectF::new(10.0, 20.0, 30.0, 40.0);
        list.push(
            DisplayItem::RoundedFill {
                rect,
                color: Rgba::BLACK,
                radii: [1.0, 2.0, 3.0, 4.0],
            },
            rect,
            Some(RectF::new(0.0, 0.0, 50.0, 60.0)),
        );
        list.add_link(rect, &Arc::from("a"), None);
        list.anchors.push((Arc::from("x"), 15.0));
        list.controls.push((rect, 3));
        list.scale(2.0);
        assert_eq!((list.width, list.height), (200.0, 400.0));
        let entry = &list.entries[0];
        assert_eq!(entry.bounds, RectF::new(20.0, 40.0, 60.0, 80.0));
        assert_eq!(entry.clip, Some(RectF::new(0.0, 0.0, 100.0, 120.0)));
        match &entry.item {
            DisplayItem::RoundedFill { rect, radii, .. } => {
                assert_eq!(*rect, RectF::new(20.0, 40.0, 60.0, 80.0));
                assert_eq!(*radii, [2.0, 4.0, 6.0, 8.0]);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(list.links[0].rect, RectF::new(20.0, 40.0, 60.0, 80.0));
        assert_eq!(list.anchor_y("x"), Some(30.0));
        assert_eq!(list.control_at(30.0, 50.0), Some(3), "拡大後の座標で当たる");
    }

    #[test]
    fn clipped_out_items_are_dropped() {
        let mut list = DisplayList::default();
        let clip = Some(RectF::new(0.0, 0.0, 10.0, 10.0));
        list.push(
            DisplayItem::Fill {
                rect: RectF::new(20.0, 20.0, 5.0, 5.0),
                color: Rgba::BLACK,
            },
            RectF::new(20.0, 20.0, 5.0, 5.0),
            clip,
        );
        assert!(list.entries.is_empty());
    }
}
