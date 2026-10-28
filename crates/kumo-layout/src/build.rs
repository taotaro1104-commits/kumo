//! DOM + 計算値からボックスツリーを生成する。

use std::sync::Arc;

use kumo_css::keywords::{Display as CssDisplay, ListStylePosition};
use kumo_css::{ComputedStyle, StyleMap};
use kumo_dom::{Document, NodeId as DomId, NodeKind};
use kumo_text::TextSystem;

use crate::boxes::{BoxKind, LayoutBox, ReplacedContent};
use crate::convert::{CalcTable, to_taffy};
use crate::inline_builder::InlineBuilder;
use crate::text_style::{marker_text, text_spec};
use crate::tree::BoxTree;

/// 画像の固有サイズを問い合わせる（デコードせずヘッダだけ読む実装を想定）。
pub trait ImageSizes {
    fn intrinsic_size(&self, src: &str) -> Option<(f32, f32)>;
}

/// 画像情報なし。
pub struct NoImages;

impl ImageSizes for NoImages {
    fn intrinsic_size(&self, _src: &str) -> Option<(f32, f32)> {
        None
    }
}

/// ボックス生成の入れ子上限。これより深い要素は描画しない。Taffy のレイアウトは
/// 入れ子1段ごとに再帰するため、Windows のメインスレッド（1MB スタック）でも溢れない深さに抑える。
const MAX_DEPTH: usize = 128;

/// 属性で指定できる画像の一辺の上限（px）。
const MAX_IMAGE_SIDE: f32 = 100_000.0;

const REPLACED: [&str; 5] = ["img", "svg", "video", "canvas", "iframe"];

pub(crate) struct Builder<'a> {
    pub(crate) doc: &'a Document,
    pub(crate) styles: &'a StyleMap,
    images: &'a dyn ImageSizes,
    pub(crate) boxes: Vec<LayoutBox>,
    pub(crate) calc: CalcTable,
    text: TextSystem,
    depth: usize,
    /// リスト項目の序数（親ごとにまとめて計算）
    ordinals: std::collections::HashMap<DomId, i32>,
    /// 開いている `<a href>` の href（内側ほど後ろ）
    links: Vec<Arc<str>>,
}

/// ボックスツリーを構築する。`text` は全タブ共有のテキストシステム。
pub fn build_box_tree(
    doc: &Document,
    styles: &StyleMap,
    text: TextSystem,
    images: &dyn ImageSizes,
) -> BoxTree {
    let mut builder = Builder {
        doc,
        styles,
        images,
        boxes: Vec::new(),
        calc: CalcTable::default(),
        text,
        depth: 0,
        ordinals: std::collections::HashMap::new(),
        links: Vec::new(),
    };
    let mut root_style = ComputedStyle::initial();
    root_style.display = CssDisplay::Block;
    builder.add_box(BoxKind::Container, None, Arc::new(root_style));
    for child in doc.children(doc.root()) {
        if let Some(style) = builder.element_style(child)
            && style.display != CssDisplay::None
        {
            let child_box = builder.build_element(child, style);
            builder.boxes[0].children.push(child_box);
        }
    }
    let mut tree = BoxTree {
        boxes: builder.boxes,
        calc: builder.calc,
        text: builder.text,
    };
    tree.mark_floats();
    tree
}

impl Builder<'_> {
    pub(crate) fn element_style(&self, node: DomId) -> Option<Arc<ComputedStyle>> {
        self.doc.element(node)?;
        self.styles.get(node).cloned()
    }

    pub(crate) fn add_box(
        &mut self,
        kind: BoxKind,
        dom: Option<DomId>,
        computed: Arc<ComputedStyle>,
    ) -> usize {
        let mut layout_box = LayoutBox::new(kind, dom, computed.clone());
        layout_box.style = to_taffy(&computed, &mut self.calc);
        layout_box.link = self.current_link();
        layout_box.anchor = dom
            .and_then(|node| self.doc.element(node))
            .and_then(|e| {
                e.id()
                    .or_else(|| e.is_html("a").then(|| e.attr("name")).flatten())
            })
            .filter(|id| !id.is_empty())
            .map(Arc::from);
        self.boxes.push(layout_box);
        self.boxes.len() - 1
    }

    fn is_replaced(&self, node: DomId) -> bool {
        let element = self.doc.element(node).expect("要素");
        REPLACED.iter().any(|t| element.is_html(t))
            || &*element.local == "svg"
            || (element.is_html("input") && crate::forms::input_type(self.doc, node) == "image")
    }

    pub(crate) fn current_link(&self) -> Option<Arc<str>> {
        self.links.last().cloned()
    }

    /// `<a href>` なら href を積んで `true` を返す（呼び出し側が抜けるときに pop する）。
    fn enter_link(&mut self, node: DomId) -> bool {
        let element = self.doc.element(node);
        let href = element
            .filter(|e| e.is_html("a") || e.is_html("area"))
            .and_then(|e| e.attr("href"));
        if let Some(href) = href {
            self.links.push(Arc::from(href));
            return true;
        }
        false
    }

    /// 要素1つ分のボックス（とその子孫）を作る。
    pub(crate) fn build_element(&mut self, node: DomId, style: Arc<ComputedStyle>) -> usize {
        self.depth += 1;
        let entered = self.enter_link(node);
        let index = self.build_element_inner(node, style);
        if entered {
            self.links.pop();
        }
        self.depth -= 1;
        index
    }

    fn build_element_inner(&mut self, node: DomId, style: Arc<ComputedStyle>) -> usize {
        if self.depth > MAX_DEPTH {
            // 深すぎる部分木は空のボックスにする
            return self.add_box(BoxKind::Container, Some(node), style);
        }
        if self.is_replaced(node) {
            return self.build_replaced(node, style);
        }
        if let Some(control) = self.build_form_control(node, &style) {
            return control;
        }
        if matches!(style.display, CssDisplay::Table | CssDisplay::InlineTable) {
            return self.build_table(node, style);
        }
        let index = self.add_box(BoxKind::Container, Some(node), style.clone());
        if self.doc.element(node).is_some_and(|e| e.is_html("button")) {
            self.boxes[index].form_control = true;
        }
        match style.display {
            CssDisplay::Flex
            | CssDisplay::InlineFlex
            | CssDisplay::Grid
            | CssDisplay::InlineGrid => {
                self.build_item_children(index, node, &style);
            }
            _ => {
                let mut ifc = InlineBuilder::new(style.clone(), Some(node), self.current_link());
                if style.display == CssDisplay::ListItem {
                    self.add_marker(index, node, &style, &mut ifc);
                }
                self.emit_pseudo(index, node, false, &mut ifc);
                for child in self.doc.children(node).collect::<Vec<_>>() {
                    self.flow_child(index, child, &mut ifc);
                }
                self.emit_pseudo(index, node, true, &mut ifc);
                self.flush(index, &mut ifc, &style);
            }
        }
        index
    }

    fn build_replaced(&mut self, node: DomId, style: Arc<ComputedStyle>) -> usize {
        let element = self.doc.element(node).expect("要素");
        let src = element.attr("src").map(Arc::from);
        let attr_size = |name: &str| {
            element
                .attr(name)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|v| v.is_finite() && *v >= 0.0 && *v <= MAX_IMAGE_SIDE)
        };
        let from_attrs = attr_size("width").zip(attr_size("height"));
        // img と input[type=image] は画像として、その他（svg・video など）は既定の大きさ
        let is_image = element.is_html("img") || element.is_html("input");
        let intrinsic = if is_image {
            from_attrs.or_else(|| src.as_deref().and_then(|s| self.images.intrinsic_size(s)))
        } else {
            from_attrs.or(Some((300.0, 150.0)))
        };
        let kind = BoxKind::Replaced(ReplacedContent { src, intrinsic });
        self.add_box(kind, Some(node), style)
    }

    /// flex / grid コンテナの子：要素はそれぞれアイテム、連続テキストは匿名アイテム。
    fn build_item_children(&mut self, parent: usize, node: DomId, style: &Arc<ComputedStyle>) {
        let mut ifc = InlineBuilder::new(style.clone(), Some(node), self.current_link());
        self.emit_pseudo(parent, node, false, &mut ifc);
        let mut stack: Vec<DomId> = self.doc.children(node).collect();
        stack.reverse();
        while let Some(child) = stack.pop() {
            match self.doc.node(child).kind() {
                NodeKind::Text(_) => ifc.push_text(self.doc.text(child).unwrap_or("")),
                NodeKind::Element(_) => {
                    let Some(child_style) = self.element_style(child) else {
                        continue;
                    };
                    match child_style.display {
                        CssDisplay::None => {}
                        CssDisplay::Contents => {
                            let mut kids: Vec<DomId> = self.doc.children(child).collect();
                            kids.reverse();
                            stack.extend(kids);
                        }
                        _ => {
                            self.flush(parent, &mut ifc, style);
                            let item = self.build_element(child, child_style);
                            self.boxes[parent].children.push(item);
                        }
                    }
                }
                _ => {}
            }
        }
        self.emit_pseudo(parent, node, true, &mut ifc);
        self.flush(parent, &mut ifc, style);
    }

    /// ブロックコンテナ内の子を1つ処理する。
    fn flow_child(&mut self, parent: usize, child: DomId, ifc: &mut InlineBuilder) {
        match self.doc.node(child).kind() {
            NodeKind::Text(_) => ifc.push_text(self.doc.text(child).unwrap_or("")),
            NodeKind::Element(_) => {
                let Some(style) = self.element_style(child) else {
                    return;
                };
                let element = self.doc.element(child).expect("要素");
                match style.display {
                    CssDisplay::None => {}
                    CssDisplay::Contents => {
                        for grandchild in self.doc.children(child).collect::<Vec<_>>() {
                            self.flow_child(parent, grandchild, ifc);
                        }
                    }
                    _ if element.is_html("br") => ifc.push_break(),
                    CssDisplay::Inline if !self.is_replaced(child) && self.depth < MAX_DEPTH => {
                        self.depth += 1;
                        let entered = self.enter_link(child);
                        ifc.open(style, child, self.current_link());
                        self.emit_pseudo(parent, child, false, ifc);
                        for grandchild in self.doc.children(child).collect::<Vec<_>>() {
                            self.flow_child(parent, grandchild, ifc);
                        }
                        self.emit_pseudo(parent, child, true, ifc);
                        ifc.close();
                        if entered {
                            self.links.pop();
                        }
                        self.depth -= 1;
                    }
                    display if display.is_inline_level() || display == CssDisplay::Inline => {
                        let atomic = self.build_element(child, style);
                        ifc.push_atomic(atomic);
                    }
                    // 段落の途中のフロートは段落を分けずに中へ入れる
                    _ if style.float != kumo_css::keywords::Float::None
                        && ifc.has_inline_content() =>
                    {
                        let float = self.build_element(child, style);
                        ifc.push_float(float);
                    }
                    _ => {
                        let parent_style = self.boxes[parent].computed.clone();
                        self.flush(parent, ifc, &parent_style);
                        let block = self.build_element(child, style);
                        self.boxes[parent].children.push(block);
                    }
                }
            }
            _ => {}
        }
    }

    /// 溜まったインライン内容を匿名インラインボックスとして親に追加する。
    pub(crate) fn flush(
        &mut self,
        parent: usize,
        ifc: &mut InlineBuilder,
        parent_style: &Arc<ComputedStyle>,
    ) {
        let Some(content) = ifc.take() else { return };
        let children: Vec<usize> = content
            .atomics
            .iter()
            .chain(&content.floats)
            .copied()
            .collect();
        let mut anon = ComputedStyle::inherit_from(parent_style);
        anon.display = CssDisplay::Block;
        let index = self.add_box(BoxKind::Inline(Box::new(content)), None, Arc::new(anon));
        self.boxes[index].children = children;
        self.boxes[parent].children.push(index);
    }

    /// リストマーカー。outside は別レイアウトとして保持し、inside は先頭にテキストとして入れる。
    fn add_marker(
        &mut self,
        index: usize,
        node: DomId,
        style: &ComputedStyle,
        ifc: &mut InlineBuilder,
    ) {
        let Some(text) = marker_text(style.list_style_type, self.ordinal(node)) else {
            return;
        };
        if style.list_style_position == ListStylePosition::Inside {
            ifc.push_text(&format!("{text} "));
            return;
        }
        let spec = text_spec(style, 0);
        let mut builder = self.text.tree_builder(&spec);
        builder.push_text(&text);
        let (mut layout, _) = builder.build();
        layout.break_all_lines(None);
        self.boxes[index].marker = Some(Box::new(layout));
    }

    /// `<ol start>` を考慮したリスト項目の序数。
    fn ordinal(&mut self, node: DomId) -> i32 {
        if let Some(&n) = self.ordinals.get(&node) {
            return n;
        }
        // 親ごとに一度だけ兄弟を走査して番号を振る（リストが長くても O(n)）
        let Some(parent) = self.doc.parent(node) else {
            return 1;
        };
        let mut next = self
            .doc
            .attr(parent, "start")
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(1);
        for child in self.doc.children(parent) {
            let is_item = self
                .styles
                .get(child)
                .is_some_and(|s| s.display == CssDisplay::ListItem);
            if is_item {
                self.ordinals.insert(child, next);
                next = next.saturating_add(1);
            }
        }
        self.ordinals.get(&node).copied().unwrap_or(next)
    }
}
