//! ボックスツリーに Taffy の低レベルAPI（各種トレイト）を実装する。

use taffy::prelude::*;
use taffy::{
    BlockContext, Cache, CacheTree, LayoutInput, LayoutOutput, compute_block_layout,
    compute_cached_layout, compute_flexbox_layout, compute_grid_layout, compute_hidden_layout,
    compute_leaf_layout, compute_oof_layout,
};

use std::sync::Arc;

use crate::boxes::{BoxKind, LayoutBox};
use crate::convert::CalcTable;
use crate::inline;
use kumo_text::TextSystem;

type TStyle = Style<Arc<str>>;

/// レイアウト対象のボックスツリー。`boxes[0]` がルート（初期包含ブロック）。
pub struct BoxTree {
    pub boxes: Vec<LayoutBox>,
    pub(crate) calc: CalcTable,
    pub(crate) text: TextSystem,
}

pub struct ChildIter<'a>(std::slice::Iter<'a, usize>);

impl Iterator for ChildIter<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        self.0.next().map(|&i| NodeId::from(i))
    }
}

impl BoxTree {
    fn node(&self, id: NodeId) -> &LayoutBox {
        &self.boxes[usize::from(id)]
    }

    fn node_mut(&mut self, id: NodeId) -> &mut LayoutBox {
        &mut self.boxes[usize::from(id)]
    }

    /// テキストシステムを返す（グリフ描画などで共有するため）。
    pub fn text_system(&mut self) -> &mut TextSystem {
        &mut self.text
    }

    /// ツリーを破棄してテキストシステム（フォントDB）だけを取り出す。再レイアウト時に使い回す。
    pub fn into_text_system(self) -> TextSystem {
        self.text
    }

    /// 子孫にフロートを含むかを後順で記録する（構築直後に1回呼ぶ）。
    pub(crate) fn mark_floats(&mut self) {
        for index in (0..self.boxes.len()).rev() {
            let has = self.boxes[index].children.iter().any(|&c| {
                let child = &self.boxes[c];
                child.style.float != taffy::style::Float::None || child.has_float_inside
            });
            self.boxes[index].has_float_inside = has;
        }
    }

    /// 全ノードのキャッシュを消す（ビューポート変更時など）。
    pub fn clear_caches(&mut self) {
        for b in &mut self.boxes {
            b.cache = Cache::new();
            if let BoxKind::Inline(content) = &mut b.kind {
                content.breaks = None;
            }
        }
    }
}

impl taffy::TraversePartialTree for BoxTree {
    type ChildIter<'a> = ChildIter<'a>;

    fn child_ids(&self, id: NodeId) -> ChildIter<'_> {
        ChildIter(self.node(id).children.iter())
    }

    fn child_count(&self, id: NodeId) -> usize {
        self.node(id).children.len()
    }

    fn get_child_id(&self, id: NodeId, index: usize) -> NodeId {
        NodeId::from(self.node(id).children[index])
    }
}

impl taffy::TraverseTree for BoxTree {}

impl taffy::LayoutPartialTree for BoxTree {
    type CoreContainerStyle<'a> = &'a TStyle;
    type CustomIdent = Arc<str>;

    fn get_core_container_style(&self, id: NodeId) -> &TStyle {
        &self.node(id).style
    }

    fn resolve_calc_value(&self, val: *const (), basis: f32) -> f32 {
        self.calc.resolve(val, basis)
    }

    fn set_unrounded_layout(&mut self, id: NodeId, layout: &Layout) {
        self.node_mut(id).unrounded = *layout;
    }

    fn compute_child_layout(&mut self, id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        compute_cached_layout(self, id, inputs, |tree, id, inputs| {
            let node = tree.node(id);
            if node.style.display == Display::None {
                return compute_hidden_layout(tree, id);
            }
            let mut output = match &node.kind {
                BoxKind::Inline(_) => inline::compute_inline_layout(tree, id, inputs, None),
                BoxKind::Replaced(content) => {
                    let intrinsic = content.intrinsic;
                    let style = &tree.node(id).style;
                    let calc = &tree.calc;
                    compute_leaf_layout(
                        inputs,
                        style,
                        |v, b| calc.resolve(v, b),
                        |known, _| replaced_size(known, intrinsic),
                    )
                }
                BoxKind::Container => match node.style.display {
                    Display::Flex => compute_flexbox_layout(tree, id, inputs),
                    Display::Grid => compute_grid_layout(tree, id, inputs),
                    _ => compute_block_layout(tree, id, inputs, None),
                },
            };
            if inputs.run_mode == taffy::RunMode::PerformLayout {
                compute_oof_layout(tree, id, &mut output);
            }
            output
        })
    }
}

/// 置換要素の大きさ。片方だけ既知なら縦横比を保つ。
fn replaced_size(known: Size<Option<f32>>, intrinsic: Option<(f32, f32)>) -> Size<f32> {
    let (iw, ih) = intrinsic.unwrap_or((0.0, 0.0));
    let ratio = if iw > 0.0 { ih / iw } else { 1.0 };
    match (known.width, known.height) {
        (Some(w), Some(h)) => Size {
            width: w,
            height: h,
        },
        (Some(w), None) => Size {
            width: w,
            height: w * ratio,
        },
        (None, Some(h)) => Size {
            width: if ratio > 0.0 { h / ratio } else { iw },
            height: h,
        },
        (None, None) => Size {
            width: iw,
            height: ih,
        },
    }
}

impl taffy::LayoutContainingBlock for BoxTree {
    type OofItemStyle<'a> = &'a TStyle;

    fn get_oof_item_style(&self, id: NodeId) -> &TStyle {
        &self.node(id).style
    }

    fn clear_hoisted_children(&mut self, id: NodeId) {
        self.node_mut(id).hoisted.clear();
    }

    fn add_hoisted_children(&mut self, id: NodeId, hoisted: &[NodeId]) {
        self.node_mut(id).hoisted.extend_from_slice(hoisted);
    }
}

impl CacheTree for BoxTree {
    fn cache_get(&mut self, id: NodeId, inputs: &LayoutInput) -> Option<LayoutOutput> {
        self.node_mut(id).cache.get(inputs)
    }

    fn cache_store(&mut self, id: NodeId, inputs: &LayoutInput, output: LayoutOutput) {
        self.node_mut(id).cache.store(inputs, output);
    }

    fn cache_clear(&mut self, id: NodeId) {
        self.node_mut(id).cache.clear();
    }
}

impl taffy::LayoutBlockContainer for BoxTree {
    type BlockContainerStyle<'a> = &'a TStyle;
    type BlockItemStyle<'a> = &'a TStyle;

    /// 同じブロック整形コンテキスト内の子。フロートがあれば文脈を渡して流し込む。
    fn compute_block_child_layout(
        &mut self,
        id: NodeId,
        inputs: LayoutInput,
        block_ctx: Option<&mut BlockContext<'_>>,
    ) -> LayoutOutput {
        let Some(ctx) = block_ctx else {
            return self.compute_child_layout(id, inputs);
        };
        let node = self.node(id);
        let affected_by_floats = ctx.has_floats() || node.has_float_inside;
        if !affected_by_floats || node.style.display == Display::None {
            return self.compute_child_layout(id, inputs);
        }
        match (&node.kind, node.style.display) {
            (BoxKind::Inline(_), _) => inline::compute_inline_layout(self, id, inputs, Some(ctx)),
            (BoxKind::Container, Display::Block) => {
                let mut output = compute_block_layout(self, id, inputs, Some(ctx));
                if inputs.run_mode == taffy::RunMode::PerformLayout {
                    compute_oof_layout(self, id, &mut output);
                }
                output
            }
            _ => self.compute_child_layout(id, inputs),
        }
    }

    fn get_block_container_style(&self, id: NodeId) -> &TStyle {
        &self.node(id).style
    }

    fn get_block_child_style(&self, id: NodeId) -> &TStyle {
        &self.node(id).style
    }
}

impl taffy::LayoutFlexboxContainer for BoxTree {
    type FlexboxContainerStyle<'a> = &'a TStyle;
    type FlexboxItemStyle<'a> = &'a TStyle;

    fn get_flexbox_container_style(&self, id: NodeId) -> &TStyle {
        &self.node(id).style
    }

    fn get_flexbox_child_style(&self, id: NodeId) -> &TStyle {
        &self.node(id).style
    }
}

impl taffy::LayoutGridContainer for BoxTree {
    type GridContainerStyle<'a> = &'a TStyle;
    type GridItemStyle<'a> = &'a TStyle;

    fn get_grid_container_style(&self, id: NodeId) -> &TStyle {
        &self.node(id).style
    }

    fn get_grid_child_style(&self, id: NodeId) -> &TStyle {
        &self.node(id).style
    }
}

impl taffy::RoundTree for BoxTree {
    fn get_unrounded_layout(&self, id: NodeId) -> Layout {
        self.node(id).unrounded
    }

    fn set_final_layout(&mut self, id: NodeId, layout: &Layout) {
        self.node_mut(id).final_layout = *layout;
    }

    fn is_out_of_flow(&self, id: NodeId) -> bool {
        self.node(id).style.position == taffy::style::Position::Absolute
    }

    fn hoisted_child_count(&self, id: NodeId) -> usize {
        self.node(id).hoisted.len()
    }

    fn get_hoisted_child_id(&self, id: NodeId, index: usize) -> NodeId {
        self.node(id).hoisted[index]
    }
}

impl taffy::PrintTree for BoxTree {
    fn get_debug_label(&self, id: NodeId) -> &'static str {
        match self.node(id).kind {
            BoxKind::Inline(_) => "INLINE",
            BoxKind::Replaced(_) => "REPLACED",
            BoxKind::Container => match self.node(id).style.display {
                Display::Flex => "FLEX",
                Display::Grid => "GRID",
                _ => "BLOCK",
            },
        }
    }

    fn get_final_layout(&self, id: NodeId) -> Layout {
        self.node(id).final_layout
    }
}
