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
}

/// リンクの当たり判定領域。`href` は未解決のまま。
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRegion {
    pub rect: RectF,
    pub href: Arc<str>,
}

impl DisplayList {
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
