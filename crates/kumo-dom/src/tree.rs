use compact_str::CompactString;
use markup5ever::QualName;

use crate::iter::{Children, Descendants};
use crate::node::{Attr, ElementData, Node, NodeId, NodeKind};

/// 1ドキュメント = 1アリーナ。drop で全ノードを一括解放する。
#[derive(Clone, Debug)]
pub struct Document {
    nodes: Vec<Node>,
    elements: Vec<ElementData>,
    texts: Vec<CompactString>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub fn new() -> Self {
        Self {
            nodes: vec![Node::new(NodeKind::Document)],
            elements: Vec::new(),
            texts: Vec::new(),
        }
    }

    pub fn root(&self) -> NodeId {
        NodeId::from_index(0)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        false // ルートノードが常に存在する
    }

    // ---- 生成 ----

    fn push(&mut self, kind: NodeKind) -> NodeId {
        self.nodes.push(Node::new(kind));
        NodeId::from_index(self.nodes.len() - 1)
    }

    pub fn create_element(&mut self, name: QualName, attrs: Vec<Attr>) -> NodeId {
        let index = u32::try_from(self.elements.len()).expect("要素数が u32 を超えた");
        self.elements.push(ElementData {
            ns: name.ns,
            local: name.local,
            attrs: attrs.into_iter().collect(),
        });
        self.push(NodeKind::Element(index))
    }

    pub fn create_text(&mut self, text: &str) -> NodeId {
        let index = u32::try_from(self.texts.len()).expect("テキスト数が u32 を超えた");
        self.texts.push(CompactString::from(text));
        self.push(NodeKind::Text(index))
    }

    pub fn create_comment(&mut self) -> NodeId {
        self.push(NodeKind::Comment)
    }

    pub fn create_doctype(&mut self) -> NodeId {
        self.push(NodeKind::Doctype)
    }

    // ---- 参照 ----

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.index()]
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).parent
    }

    pub fn first_child(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).first_child
    }

    pub fn last_child(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).last_child
    }

    pub fn next_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).next_sibling
    }

    pub fn prev_sibling(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).prev_sibling
    }

    pub fn children(&self, id: NodeId) -> Children<'_> {
        Children {
            doc: self,
            next: self.first_child(id),
        }
    }

    pub fn descendants(&self, id: NodeId) -> Descendants<'_> {
        Descendants {
            doc: self,
            root: id,
            next: Some(id),
        }
    }

    pub fn element(&self, id: NodeId) -> Option<&ElementData> {
        match self.node(id).kind {
            NodeKind::Element(i) => Some(&self.elements[i as usize]),
            _ => None,
        }
    }

    pub fn text(&self, id: NodeId) -> Option<&str> {
        match self.node(id).kind {
            NodeKind::Text(i) => Some(self.texts[i as usize].as_str()),
            _ => None,
        }
    }

    pub fn attr(&self, id: NodeId, name: &str) -> Option<&str> {
        self.element(id)?.attr(name)
    }

    /// 子孫のテキストを連結する（DOM の textContent 相当）。
    pub fn text_content(&self, id: NodeId) -> String {
        self.descendants(id).filter_map(|n| self.text(n)).collect()
    }

    // ---- 変更 ----

    /// 属性を設定する（なければ追加）。要素以外には何もしない。
    pub fn set_attr(&mut self, id: NodeId, name: &str, value: &str) {
        let NodeKind::Element(i) = self.node(id).kind else {
            return;
        };
        let element = &mut self.elements[i as usize];
        match element.attrs.iter_mut().find(|a| &*a.name == name) {
            Some(attr) => attr.value = CompactString::from(value),
            None => element
                .attrs
                .push(Attr::new(markup5ever::LocalName::from(name), value)),
        }
    }

    /// 属性を取り除く。
    pub fn remove_attr(&mut self, id: NodeId, name: &str) {
        if let NodeKind::Element(i) = self.node(id).kind {
            self.elements[i as usize].attrs.retain(|a| &*a.name != name);
        }
    }

    /// `id` が `ancestor` の子孫（自身を含む）か。
    pub fn is_inclusive_descendant(&self, id: NodeId, ancestor: NodeId) -> bool {
        let mut node = Some(id);
        while let Some(n) = node {
            if n == ancestor {
                return true;
            }
            node = self.parent(n);
        }
        false
    }

    pub fn add_attrs_if_missing(&mut self, id: NodeId, attrs: Vec<Attr>) {
        let NodeKind::Element(i) = self.node(id).kind else {
            return;
        };
        let element = &mut self.elements[i as usize];
        for attr in attrs {
            if !element.attrs.iter().any(|a| a.name == attr.name) {
                element.attrs.push(attr);
            }
        }
    }

    /// `child` を現在の親から外し、`parent` の末尾に追加する。
    pub fn append(&mut self, parent: NodeId, child: NodeId) {
        self.detach(child);
        let last = self.last_child(parent);
        {
            let node = &mut self.nodes[child.index()];
            node.parent = Some(parent);
            node.prev_sibling = last;
        }
        match last {
            Some(last) => self.nodes[last.index()].next_sibling = Some(child),
            None => self.nodes[parent.index()].first_child = Some(child),
        }
        self.nodes[parent.index()].last_child = Some(child);
    }

    /// `child` を `sibling` の直前に挿入する。`sibling` は親を持っている必要がある。
    pub fn insert_before(&mut self, sibling: NodeId, child: NodeId) {
        self.detach(child);
        let parent = self
            .parent(sibling)
            .expect("insert_before: sibling に親がない");
        let prev = self.prev_sibling(sibling);
        {
            let node = &mut self.nodes[child.index()];
            node.parent = Some(parent);
            node.prev_sibling = prev;
            node.next_sibling = Some(sibling);
        }
        self.nodes[sibling.index()].prev_sibling = Some(child);
        match prev {
            Some(prev) => self.nodes[prev.index()].next_sibling = Some(child),
            None => self.nodes[parent.index()].first_child = Some(child),
        }
    }

    /// ノードを親から切り離す（アリーナ上には残る）。
    pub fn detach(&mut self, id: NodeId) {
        let Node {
            parent,
            prev_sibling,
            next_sibling,
            ..
        } = self.nodes[id.index()].clone();
        let Some(parent) = parent else {
            return;
        };
        match prev_sibling {
            Some(prev) => self.nodes[prev.index()].next_sibling = next_sibling,
            None => self.nodes[parent.index()].first_child = next_sibling,
        }
        match next_sibling {
            Some(next) => self.nodes[next.index()].prev_sibling = prev_sibling,
            None => self.nodes[parent.index()].last_child = prev_sibling,
        }
        let node = &mut self.nodes[id.index()];
        node.parent = None;
        node.prev_sibling = None;
        node.next_sibling = None;
    }

    /// 末尾にテキストを追加。直前の子がテキストなら連結する（パーサの挙動に合わせる）。
    pub fn append_text(&mut self, parent: NodeId, text: &str) {
        if let Some(last) = self.last_child(parent)
            && self.push_str_to_text(last, text)
        {
            return;
        }
        let node = self.create_text(text);
        self.append(parent, node);
    }

    /// `sibling` の直前にテキストを挿入。直前がテキストなら連結する。
    pub fn insert_text_before(&mut self, sibling: NodeId, text: &str) {
        if let Some(prev) = self.prev_sibling(sibling)
            && self.push_str_to_text(prev, text)
        {
            return;
        }
        let node = self.create_text(text);
        self.insert_before(sibling, node);
    }

    fn push_str_to_text(&mut self, id: NodeId, text: &str) -> bool {
        match self.nodes[id.index()].kind {
            NodeKind::Text(i) => {
                self.texts[i as usize].push_str(text);
                true
            }
            _ => false,
        }
    }

    /// `from` の子を全て `to` の末尾へ移す。
    pub fn reparent_children(&mut self, from: NodeId, to: NodeId) {
        while let Some(child) = self.first_child(from) {
            self.append(to, child);
        }
    }
}
