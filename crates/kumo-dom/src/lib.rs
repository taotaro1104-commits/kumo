//! Kumo のアリーナDOM。
//!
//! ノードはドキュメントごとの `Vec` に格納し、`u32` ベースの [`NodeId`] で参照する。
//! `Rc<RefCell<…>>` の木を使わないため、参照カウントとポインタ分のメモリが不要で、
//! ドキュメントを drop すれば全ノードが一括で解放される。
#![forbid(unsafe_code)]

mod edit;
mod iter;
mod node;
mod serialize;
mod tree;

pub use iter::{Children, Descendants};
pub use markup5ever::{LocalName, Namespace, QualName, local_name, ns};
pub use node::{Attr, ElementData, Node, NodeId, NodeKind};
pub use tree::Document;
