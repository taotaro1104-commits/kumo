use std::num::NonZeroU32;

use compact_str::CompactString;
use markup5ever::{LocalName, Namespace};
use smallvec::SmallVec;

/// アリーナ内のノードを指す32ビットID（`Option<NodeId>` も4バイトに収まる）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(NonZeroU32);

impl NodeId {
    /// アリーナ上のインデックスから ID を作る（[`NodeId::index`] の逆）。
    pub fn from_index(index: usize) -> Self {
        let raw = u32::try_from(index + 1).expect("ノード数が u32 の範囲を超えた");
        Self(NonZeroU32::new(raw).expect("index + 1 は 0 にならない"))
    }

    /// アリーナ上のインデックス（0始まり）。
    pub fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

/// ノード種別。可変長データは別の Vec に置き、ここでは添字だけを持つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Document,
    Doctype,
    /// `Document::elements` への添字
    Element(u32),
    /// `Document::texts` への添字
    Text(u32),
    /// Phase 0 ではJSが無いのでコメント本文は保持しない
    Comment,
}

/// ツリー上の1ノード。木構造のリンクはすべて4バイトの `Option<NodeId>`。
#[derive(Clone, Debug)]
pub struct Node {
    pub(crate) parent: Option<NodeId>,
    pub(crate) first_child: Option<NodeId>,
    pub(crate) last_child: Option<NodeId>,
    pub(crate) prev_sibling: Option<NodeId>,
    pub(crate) next_sibling: Option<NodeId>,
    pub(crate) kind: NodeKind,
}

impl Node {
    pub(crate) fn new(kind: NodeKind) -> Self {
        Self {
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            kind,
        }
    }

    pub fn kind(&self) -> NodeKind {
        self.kind
    }
}

/// 属性。名前はインターン済みAtom、値は短ければインライン保持。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attr {
    pub name: LocalName,
    pub value: CompactString,
}

impl Attr {
    pub fn new(name: LocalName, value: impl Into<CompactString>) -> Self {
        Self {
            name,
            value: value.into(),
        }
    }
}

/// 要素固有のデータ。属性は2個までインライン、あふれた分だけヒープ。
#[derive(Clone, Debug)]
pub struct ElementData {
    pub ns: Namespace,
    pub local: LocalName,
    pub attrs: SmallVec<[Attr; 2]>,
}

impl ElementData {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| &*a.name == name)
            .map(|a| a.value.as_str())
    }

    pub fn id(&self) -> Option<&str> {
        self.attr("id")
    }

    /// `class` 属性を空白で分割したクラス名列。
    pub fn classes(&self) -> impl Iterator<Item = &str> {
        self.attr("class").unwrap_or("").split_ascii_whitespace()
    }

    pub fn has_class(&self, name: &str) -> bool {
        self.classes().any(|c| c == name)
    }

    pub fn is_html(&self, local: &str) -> bool {
        self.ns == markup5ever::ns!(html) && &*self.local == local
    }
}
