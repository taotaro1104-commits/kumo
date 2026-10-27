//! インライン整形コンテキストの内容を組み立てるビルダ。

use std::sync::Arc;

use kumo_css::ComputedStyle;
use kumo_css::keywords::WhiteSpace;
use kumo_dom::NodeId as DomId;

use crate::boxes::{InlineContent, InlineItem, TextPaint};
use crate::text_style::{text_spec, transform_text};

struct OpenSpan {
    style: Arc<ComputedStyle>,
    node: DomId,
    decorated: bool,
}

/// ブロックコンテナ内のインライン内容を集める。ブロックが割り込んだら `take` で区切る。
pub struct InlineBuilder {
    root: Arc<ComputedStyle>,
    root_node: Option<DomId>,
    stack: Vec<OpenSpan>,
    items: Vec<InlineItem>,
    paints: Vec<TextPaint>,
    pub atomics: Vec<usize>,
    floats: Vec<usize>,
}

/// 背景や枠を持つインライン要素か（区間を記録して描画する）。
fn is_decorated(style: &ComputedStyle) -> bool {
    !style.background_color.is_transparent()
        || kumo_css::Side::ALL
            .iter()
            .any(|&s| style.used_border_width(s) > 0.0)
}

impl InlineBuilder {
    pub fn new(root: Arc<ComputedStyle>, root_node: Option<DomId>) -> Self {
        let mut builder = Self {
            root,
            root_node,
            stack: Vec::new(),
            items: Vec::new(),
            paints: Vec::new(),
            atomics: Vec::new(),
            floats: Vec::new(),
        };
        builder.reset();
        builder
    }

    /// 現在のテキストに効いているスタイル。
    pub fn current_style(&self) -> &Arc<ComputedStyle> {
        self.stack.last().map_or(&self.root, |s| &s.style)
    }

    pub fn open(&mut self, style: Arc<ComputedStyle>, node: DomId) {
        let decorated = is_decorated(&style);
        if decorated {
            self.items.push(InlineItem::SpanStart(style.clone()));
        }
        let brush = self.push_paint(style.clone(), Some(node));
        self.items
            .push(InlineItem::PushStyle(text_spec(&style, brush)));
        self.stack.push(OpenSpan {
            style,
            node,
            decorated,
        });
    }

    pub fn close(&mut self) {
        if let Some(span) = self.stack.pop() {
            self.items.push(InlineItem::PopStyle);
            if span.decorated {
                self.items.push(InlineItem::SpanEnd);
            }
        }
    }

    pub fn push_text(&mut self, text: &str) {
        let transformed = transform_text(text, self.current_style().text_transform);
        self.items.push(InlineItem::Text(transformed));
    }

    /// `<br>`：保持モードの改行を入れて強制改行する。
    pub fn push_break(&mut self) {
        let mut style = (**self.current_style()).clone();
        style.white_space = WhiteSpace::Pre;
        let brush = self.paints.len() as u32;
        self.paints.push(TextPaint {
            style: Arc::new(style.clone()),
            node: None,
        });
        self.items
            .push(InlineItem::PushStyle(text_spec(&style, brush)));
        self.items.push(InlineItem::Text("\n".to_owned()));
        self.items.push(InlineItem::PopStyle);
    }

    pub fn push_atomic(&mut self, child: usize) {
        self.items.push(InlineItem::Atomic(child));
        self.atomics.push(child);
    }

    /// 段落の途中のフロート。段落を分けずに、行分割の途中で配置する。
    pub fn push_float(&mut self, child: usize) {
        self.items.push(InlineItem::Float(child));
        self.floats.push(child);
    }

    /// 空白以外のテキストやアトミックをすでに含むか（＝段落の途中か）。
    pub fn has_inline_content(&self) -> bool {
        self.items.iter().any(|item| match item {
            InlineItem::Text(t) => !t.trim().is_empty(),
            InlineItem::Atomic(_) => true,
            _ => false,
        })
    }

    fn push_paint(&mut self, style: Arc<ComputedStyle>, node: Option<DomId>) -> u32 {
        self.paints.push(TextPaint { style, node });
        (self.paints.len() - 1) as u32
    }

    /// 内容を取り出し、開いているインライン要素を新しい区間で開き直す。
    /// 空白だけなら `None`。
    pub fn take(&mut self) -> Option<InlineContent> {
        let open: Vec<(Arc<ComputedStyle>, DomId)> = self
            .stack
            .iter()
            .map(|s| (s.style.clone(), s.node))
            .collect();
        // 開いたままの区間を閉じて整合させる
        for _ in 0..self.stack.len() {
            self.close();
        }
        let content = InlineContent {
            root_style: text_spec(&self.root, 0),
            items: std::mem::take(&mut self.items),
            paints: std::mem::take(&mut self.paints),
            text_align: self.root.text_align,
            text_indent: self.root.text_indent.px,
            layout: None,
            text: String::new(),
            spans: Vec::new(),
            atomics: std::mem::take(&mut self.atomics),
            floats: std::mem::take(&mut self.floats),
            breaks: None,
            final_breaks: None,
        };
        self.reset();
        for (style, node) in open {
            self.open(style, node);
        }
        content.has_content().then_some(content)
    }

    fn reset(&mut self) {
        self.items.clear();
        self.paints.clear();
        self.stack.clear();
        let root = self.root.clone();
        self.push_paint(root, self.root_node);
    }
}
