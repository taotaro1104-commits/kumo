use crate::{Document, NodeId};

/// 子ノードを順に返すイテレータ。
pub struct Children<'a> {
    pub(crate) doc: &'a Document,
    pub(crate) next: Option<NodeId>,
}

impl Iterator for Children<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let current = self.next?;
        self.next = self.doc.next_sibling(current);
        Some(current)
    }
}

/// `root` 自身を含む先行順（preorder）走査。再帰もスタックも使わない。
pub struct Descendants<'a> {
    pub(crate) doc: &'a Document,
    pub(crate) root: NodeId,
    pub(crate) next: Option<NodeId>,
}

impl Iterator for Descendants<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let current = self.next?;
        self.next = self.advance(current);
        Some(current)
    }
}

impl Descendants<'_> {
    fn advance(&self, current: NodeId) -> Option<NodeId> {
        if let Some(child) = self.doc.first_child(current) {
            return Some(child);
        }
        let mut node = current;
        loop {
            if node == self.root {
                return None;
            }
            if let Some(sibling) = self.doc.next_sibling(node) {
                return Some(sibling);
            }
            node = self.doc.parent(node)?;
        }
    }
}
