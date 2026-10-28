//! JS からの DOM 書き換えに使う操作：要素の生成、テキストの更新、他ドキュメントからの取り込み、複製。

use compact_str::CompactString;
use markup5ever::{LocalName, QualName, ns};

use crate::node::{Attr, NodeId, NodeKind};
use crate::tree::Document;

impl Document {
    /// HTML 名前空間の要素を作る（`document.createElement`）。名前は小文字にする。
    pub fn create_html_element(&mut self, local: &str) -> NodeId {
        let name = QualName::new(None, ns!(html), LocalName::from(local.to_ascii_lowercase()));
        self.create_element(name, Vec::new())
    }

    /// テキストノードの内容を置き換える。テキストでなければ `false`。
    pub fn set_text_data(&mut self, id: NodeId, text: &str) -> bool {
        match self.nodes[id.index()].kind {
            NodeKind::Text(i) => {
                self.texts[i as usize] = CompactString::from(text);
                true
            }
            _ => false,
        }
    }

    /// 子をすべて切り離す。
    pub fn clear_children(&mut self, id: NodeId) {
        while let Some(child) = self.first_child(id) {
            self.detach(child);
        }
    }

    /// 他のドキュメントのノード（と子孫）をこのドキュメントへ複製して取り込み、新しいノードを返す。
    /// 取り込んだノードは親を持たない。コメントとドキュメント型は落とす（保持していないため）。
    pub fn import_subtree(&mut self, from: &Document, node: NodeId) -> Option<NodeId> {
        let copy = match from.node(node).kind() {
            NodeKind::Element(_) => {
                let data = from.element(node)?;
                let name = QualName::new(None, data.ns.clone(), data.local.clone());
                self.create_element(name, data.attrs.iter().cloned().collect::<Vec<Attr>>())
            }
            NodeKind::Text(_) => self.create_text(from.text(node)?),
            _ => return None,
        };
        for child in from.children(node) {
            if let Some(child_copy) = self.import_subtree(from, child) {
                self.append(copy, child_copy);
            }
        }
        Some(copy)
    }

    /// ノードの複製（`cloneNode`）。`deep` なら子孫も複製する。親は持たない。
    pub fn clone_node(&mut self, id: NodeId, deep: bool) -> Option<NodeId> {
        let copy = match self.node(id).kind() {
            NodeKind::Element(_) => {
                let data = self.element(id)?.clone();
                let name = QualName::new(None, data.ns, data.local);
                self.create_element(name, data.attrs.into_iter().collect::<Vec<Attr>>())
            }
            NodeKind::Text(_) => {
                let text = self.text(id)?.to_owned();
                self.create_text(&text)
            }
            _ => return None,
        };
        if deep {
            let children: Vec<NodeId> = self.children(id).collect();
            for child in children {
                if let Some(child_copy) = self.clone_node(child, true) {
                    self.append(copy, child_copy);
                }
            }
        }
        Some(copy)
    }
}
