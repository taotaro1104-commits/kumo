use std::borrow::Cow;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;

use html5ever::interface::{ElemName, ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::StrTendril;
use html5ever::{Attribute, LocalName, Namespace, QualName};
use kumo_dom::{Attr, Document, ElementData, NodeId};

/// html5ever の TreeSink 実装。`&self` で呼ばれるため内部可変性で Document を持つ。
#[derive(Default)]
pub(crate) struct ArenaSink {
    doc: RefCell<Document>,
    /// `<template>` 要素 → 木に属さない内容ノード（描画されない）
    templates: RefCell<HashMap<NodeId, NodeId>>,
}

#[derive(Debug)]
pub(crate) struct ArenaElemName<'a>(Ref<'a, ElementData>);

impl ElemName for ArenaElemName<'_> {
    fn ns(&self) -> &Namespace {
        &self.0.ns
    }

    fn local_name(&self) -> &LocalName {
        &self.0.local
    }
}

fn convert_attrs(attrs: Vec<Attribute>) -> Vec<Attr> {
    attrs
        .into_iter()
        .map(|a| Attr::new(a.name.local, &*a.value))
        .collect()
}

impl TreeSink for ArenaSink {
    type Handle = NodeId;
    type Output = Document;
    type ElemName<'a> = ArenaElemName<'a>;

    fn finish(self) -> Document {
        self.doc.into_inner()
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {}

    fn get_document(&self) -> NodeId {
        self.doc.borrow().root()
    }

    fn elem_name<'a>(&'a self, target: &'a NodeId) -> ArenaElemName<'a> {
        ArenaElemName(Ref::map(self.doc.borrow(), |doc| {
            doc.element(*target).expect("elem_name: 要素ではないノード")
        }))
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> NodeId {
        let mut doc = self.doc.borrow_mut();
        let node = doc.create_element(name, convert_attrs(attrs));
        if flags.template {
            // 内容は切り離されたノードの子として保持し、文書ツリーには出さない
            let contents = doc.create_comment();
            self.templates.borrow_mut().insert(node, contents);
        }
        node
    }

    fn create_comment(&self, _text: StrTendril) -> NodeId {
        self.doc.borrow_mut().create_comment()
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> NodeId {
        self.doc.borrow_mut().create_comment()
    }

    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        let mut doc = self.doc.borrow_mut();
        match child {
            NodeOrText::AppendNode(node) => doc.append(*parent, node),
            NodeOrText::AppendText(text) => doc.append_text(*parent, &text),
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &NodeId,
        prev_element: &NodeId,
        child: NodeOrText<NodeId>,
    ) {
        let has_parent = self.doc.borrow().parent(*element).is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public: StrTendril,
        _system: StrTendril,
    ) {
        let mut doc = self.doc.borrow_mut();
        let doctype = doc.create_doctype();
        let root = doc.root();
        doc.append(root, doctype);
    }

    fn get_template_contents(&self, target: &NodeId) -> NodeId {
        *self
            .templates
            .borrow()
            .get(target)
            .expect("get_template_contents: template 以外")
    }

    fn same_node(&self, x: &NodeId, y: &NodeId) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn append_before_sibling(&self, sibling: &NodeId, new_node: NodeOrText<NodeId>) {
        let mut doc = self.doc.borrow_mut();
        match new_node {
            NodeOrText::AppendNode(node) => doc.insert_before(*sibling, node),
            NodeOrText::AppendText(text) => doc.insert_text_before(*sibling, &text),
        }
    }

    fn add_attrs_if_missing(&self, target: &NodeId, attrs: Vec<Attribute>) {
        self.doc
            .borrow_mut()
            .add_attrs_if_missing(*target, convert_attrs(attrs));
    }

    fn remove_from_parent(&self, target: &NodeId) {
        self.doc.borrow_mut().detach(*target);
    }

    fn reparent_children(&self, node: &NodeId, new_parent: &NodeId) {
        self.doc.borrow_mut().reparent_children(*node, *new_parent);
    }
}
