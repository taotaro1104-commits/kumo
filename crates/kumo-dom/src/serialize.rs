//! DOM を HTML 文字列にする（`innerHTML` / `outerHTML`）。

use crate::node::{NodeId, NodeKind};
use crate::tree::Document;

/// 閉じタグを持たない要素。
const VOID: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];
/// 中身をエスケープしない要素。
const RAW_TEXT: [&str; 5] = ["script", "style", "xmp", "iframe", "noembed"];

fn escape_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            c => out.push(c),
        }
    }
}

fn escape_attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            c => out.push(c),
        }
    }
}

impl Document {
    /// 子ノードの HTML。
    pub fn inner_html(&self, id: NodeId) -> String {
        let mut out = String::new();
        let raw = self
            .element(id)
            .is_some_and(|e| RAW_TEXT.contains(&&*e.local));
        for child in self.children(id) {
            self.write_node(child, raw, &mut out);
        }
        out
    }

    /// 自身を含む HTML。
    pub fn outer_html(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.write_node(id, false, &mut out);
        out
    }

    fn write_node(&self, id: NodeId, raw_parent: bool, out: &mut String) {
        match self.node(id).kind() {
            NodeKind::Text(_) => {
                let text = self.text(id).unwrap_or_default();
                if raw_parent {
                    out.push_str(text);
                } else {
                    escape_text(text, out);
                }
            }
            NodeKind::Element(_) => {
                let Some(element) = self.element(id) else {
                    return;
                };
                let name: &str = &element.local;
                out.push('<');
                out.push_str(name);
                for attr in &element.attrs {
                    out.push(' ');
                    out.push_str(&attr.name);
                    out.push_str("=\"");
                    escape_attr(&attr.value, out);
                    out.push('"');
                }
                out.push('>');
                if VOID.contains(&name) {
                    return;
                }
                let raw = RAW_TEXT.contains(&name);
                for child in self.children(id) {
                    self.write_node(child, raw, out);
                }
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
            NodeKind::Document => {
                for child in self.children(id) {
                    self.write_node(child, false, out);
                }
            }
            NodeKind::Doctype | NodeKind::Comment => {}
        }
    }
}
