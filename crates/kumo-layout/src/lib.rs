//! Kumo のレイアウト：ボックスツリー生成 → Taffy（block/flex/grid/float）+ Parley（インライン）。
#![forbid(unsafe_code)]

mod boxes;
mod build;
mod convert;
mod forms;
mod grid;
mod inline;
mod inline_break;
mod inline_builder;
mod table;
mod text_style;
mod tree;

pub use boxes::{
    BoxKind, BreakPlan, InlineContent, InlineSpan, LayoutBox, LineSlot, ReplacedContent, TextPaint,
};
pub use build::{ImageSizes, NoImages, build_box_tree};
pub use forms::{ControlText, FOCUS_ATTR, control_text, input_type, selected_option_text};
pub use tree::BoxTree;

use taffy::prelude::*;

/// ビューポートの大きさでレイアウトを計算する（結果は各ボックスの `layout()`）。
pub fn layout(tree: &mut BoxTree, viewport_width: f32, viewport_height: f32) {
    let root = &mut tree.boxes[0].style;
    root.size = Size {
        width: Dimension::length(viewport_width),
        height: Dimension::length(viewport_height),
    };
    tree.clear_caches();
    taffy::compute_root_layout(
        tree,
        NodeId::from(0usize),
        Size {
            width: AvailableSpace::Definite(viewport_width),
            height: AvailableSpace::Definite(viewport_height),
        },
    );
    // 計測で別の幅に折り返されたままの段落を、確定した改行に戻す
    inline::finalize_breaks(tree);
    taffy::round_layout(tree, NodeId::from(0usize));
}

/// 文書全体の高さ（スクロール範囲の計算用）。
pub fn document_height(tree: &BoxTree) -> f32 {
    fn bottom(tree: &BoxTree, index: usize, offset_y: f32) -> f32 {
        let b = &tree.boxes[index];
        let y = offset_y + b.layout().location.y;
        let own = y + b.layout().size.height;
        b.children
            .iter()
            .map(|&c| bottom(tree, c, y))
            .fold(own, f32::max)
    }
    bottom(tree, 0, 0.0)
}
