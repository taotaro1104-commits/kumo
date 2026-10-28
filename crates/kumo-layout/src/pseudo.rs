//! 生成コンテンツ：`::before` / `::after`。`content` が作る文字列を、要素の内容の前後に差し込む。
//! `display: inline`（既定）なら親のインライン内容に連ねて、それ以外（クリアフィックスの
//! `display: table; clear: both` など）は子のボックスとして置く。

use std::sync::Arc;

use kumo_css::ComputedStyle;
use kumo_css::keywords::{Display as CssDisplay, Float};
use kumo_dom::NodeId as DomId;

use crate::boxes::BoxKind;
use crate::build::Builder;
use crate::inline_builder::InlineBuilder;

impl Builder<'_> {
    /// `node` の ::before（`after` が偽）または ::after を、`parent` ボックスの子として差し込む。
    pub(crate) fn emit_pseudo(
        &mut self,
        parent: usize,
        node: DomId,
        after: bool,
        ifc: &mut InlineBuilder,
    ) {
        let Some((style, text)) = self.styles.pseudo(node, after).cloned() else {
            return;
        };
        if style.display == CssDisplay::None {
            return;
        }
        if style.display == CssDisplay::Inline {
            ifc.open(style, node, self.current_link());
            ifc.push_text(&text);
            ifc.close();
            return;
        }
        let boxed = self.pseudo_box(&style, &text);
        if style.float != Float::None && ifc.has_inline_content() {
            ifc.push_float(boxed);
        } else if style.display.is_inline_level() {
            ifc.push_atomic(boxed);
        } else {
            let parent_style = self.boxes[parent].computed.clone();
            self.flush(parent, ifc, &parent_style);
            self.boxes[parent].children.push(boxed);
        }
    }

    /// 疑似要素のボックス（生成元の要素とは結びつけない匿名の箱）。
    fn pseudo_box(&mut self, style: &Arc<ComputedStyle>, text: &str) -> usize {
        let index = self.add_box(BoxKind::Container, None, style.clone());
        if !text.is_empty() {
            let mut inner = InlineBuilder::new(style.clone(), None, self.current_link());
            inner.push_text(text);
            self.flush(index, &mut inner, style);
        }
        index
    }
}
