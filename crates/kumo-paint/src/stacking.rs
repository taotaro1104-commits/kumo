//! 重なり順（CSS 2.1 付録E の簡略版）でボックスツリーを描画命令に並べる。
//!
//! スタッキングコンテキスト（とフロート・位置指定要素・インラインブロックの疑似コンテキスト）ごとに
//! 次の順で描く：
//! 1. 自身の背景と枠
//! 2. z-index が負の子コンテキスト（z の昇順）
//! 3. 通常フローのブロックの背景と枠
//! 4. フロート
//! 5. インライン内容（テキスト・画像・インラインブロック）
//! 6. 位置指定要素（z-index が auto/0）を文書順に
//! 7. z-index が正の子コンテキスト（z の昇順）
//!
//! 簡略化：疑似コンテキスト内の位置指定子孫は、本来は外側の本物のコンテキストへ属するが、
//! ここでは疑似コンテキストごとまとめて描く。

use kumo_css::keywords::{Float, Overflow, Visibility};
use kumo_layout::{BoxKind, BoxTree};

use crate::builder::{inset, paint_background_and_borders, paint_content};
use crate::display_list::{DisplayList, RectF};

/// 文書座標に置いたボックス。`rect` はボーダーボックス、`clip` は先祖の overflow によるクリップ。
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placed {
    pub index: usize,
    pub rect: RectF,
    pub clip: Option<RectF>,
}

enum InlinePiece {
    /// ボックス自身のテキスト・マーカー・画像
    Content(Placed),
    /// インラインブロックなど、まとめて描く子孫
    Atomic(Placed),
}

#[derive(Default)]
struct Layers {
    negative: Vec<(i32, Placed)>,
    blocks: Vec<Placed>,
    floats: Vec<Placed>,
    inlines: Vec<InlinePiece>,
    positioned: Vec<Placed>,
    positive: Vec<(i32, Placed)>,
}

/// `placed` を（疑似）スタッキングコンテキストの根として描く。
pub(crate) fn paint_layer(tree: &BoxTree, placed: Placed, list: &mut DisplayList) {
    paint_own_background(tree, placed, list);
    let mut layers = Layers::default();
    layers.inlines.push(InlinePiece::Content(placed));
    collect(tree, placed, &mut layers);

    // sort_by_key は安定ソートなので、同じ z は文書順のまま
    layers.negative.sort_by_key(|(z, _)| *z);
    layers.positive.sort_by_key(|(z, _)| *z);
    for (_, child) in layers.negative {
        paint_layer(tree, child, list);
    }
    for block in layers.blocks {
        paint_own_background(tree, block, list);
    }
    for float in layers.floats {
        paint_layer(tree, float, list);
    }
    for piece in layers.inlines {
        match piece {
            InlinePiece::Content(p) => {
                if is_visible(tree, p.index) {
                    paint_content(tree, p.index, p.rect, p.clip, list);
                }
            }
            InlinePiece::Atomic(p) => paint_layer(tree, p, list),
        }
    }
    for child in layers.positioned {
        paint_layer(tree, child, list);
    }
    for (_, child) in layers.positive {
        paint_layer(tree, child, list);
    }
}

fn is_visible(tree: &BoxTree, index: usize) -> bool {
    tree.boxes[index].computed.visibility == Visibility::Visible
}

/// 背景と枠。匿名ボックスと、キャンバスへ伝播済みのルート要素は描かない。
fn paint_own_background(tree: &BoxTree, placed: Placed, list: &mut DisplayList) {
    let b = &tree.boxes[placed.index];
    let is_root_element = tree.boxes[0].children.first() == Some(&placed.index);
    if list.record_hits
        && let Some(node) = b.dom
    {
        list.add_geometry(node.index(), placed.rect);
    }
    if let Some(name) = &b.anchor {
        list.anchors.push((name.clone(), placed.rect.y));
    }
    if b.form_control
        && let Some(node) = b.dom
    {
        let rect = placed
            .clip
            .map_or(placed.rect, |c| placed.rect.intersect(&c));
        list.controls.push((rect, node.index()));
    }
    if let Some(node) = b.dom
        && is_visible(tree, placed.index)
    {
        list.add_hit(placed.rect, node.index(), placed.clip);
    }
    if b.dom.is_some() && !is_root_element && is_visible(tree, placed.index) {
        paint_background_and_borders(&b.computed, placed.rect, placed.clip, list);
        // ブロックや画像そのものがリンクの場合（テキストは段落側で記録する）
        if let Some(href) = &b.link
            && !matches!(b.kind, BoxKind::Inline(_))
        {
            list.add_link(placed.rect, href, placed.clip);
        }
    }
}

/// 子孫に効くクリップ（overflow が visible 以外ならパディングボックスで切る）。
fn clip_for_children(tree: &BoxTree, placed: Placed) -> Option<RectF> {
    let style = &tree.boxes[placed.index].computed;
    if style.overflow_x == Overflow::Visible && style.overflow_y == Overflow::Visible {
        return placed.clip;
    }
    let padding_box = inset(placed.rect, |s| style.used_border_width(s));
    Some(
        placed
            .clip
            .map_or(padding_box, |c| c.intersect(&padding_box)),
    )
}

/// `parent` の子孫を、別コンテキストになるものを除いて層ごとに振り分ける。
fn collect(tree: &BoxTree, parent: Placed, layers: &mut Layers) {
    let parent_box = &tree.boxes[parent.index];
    let clip = clip_for_children(tree, parent);
    let parent_is_inline = matches!(parent_box.kind, BoxKind::Inline(_));
    for &child in &parent_box.children {
        let b = &tree.boxes[child];
        let l = b.layout();
        let placed = Placed {
            index: child,
            rect: RectF::new(
                parent.rect.x + l.location.x,
                parent.rect.y + l.location.y,
                l.size.width,
                l.size.height,
            ),
            clip,
        };
        let style = &b.computed;
        if style.creates_stacking_context() {
            match style.z_index.unwrap_or(0) {
                z if z < 0 => layers.negative.push((z, placed)),
                z if z > 0 => layers.positive.push((z, placed)),
                _ => layers.positioned.push(placed),
            }
        } else if style.is_positioned() {
            layers.positioned.push(placed);
        } else if style.float != Float::None {
            layers.floats.push(placed);
        } else if parent_is_inline {
            layers.inlines.push(InlinePiece::Atomic(placed));
        } else {
            layers.blocks.push(placed);
            layers.inlines.push(InlinePiece::Content(placed));
            collect(tree, placed, layers);
        }
    }
}
