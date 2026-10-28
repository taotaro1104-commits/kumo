//! セレクタによる要素の検索（`querySelector` / `matches` / `closest` 用）。

use kumo_dom::{Document, NodeId};
use selectors::matching::{
    MatchingContext, MatchingForInvalidation, MatchingMode, NeedsSelectorFlags, QuirksMode,
    SelectorCaches, matches_selector,
};

use crate::element::ElementRef;
use crate::selector::SelectorList;

fn matches_with(
    doc: &Document,
    node: NodeId,
    list: &SelectorList,
    caches: &mut SelectorCaches,
) -> bool {
    if doc.element(node).is_none() {
        return false;
    }
    let element = ElementRef::new(doc, node);
    let mut ctx = MatchingContext::new(
        MatchingMode::Normal,
        None,
        caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    list.slice()
        .iter()
        .any(|selector| matches_selector(selector, 0, None, &element, &mut ctx))
}

/// `node` がセレクタに一致するか（`element.matches`）。
pub fn matches(doc: &Document, node: NodeId, list: &SelectorList) -> bool {
    matches_with(doc, node, list, &mut SelectorCaches::default())
}

/// `root` の子孫（自身は含まない）のうち一致する要素を文書順に返す。`first_only` なら最初の1つだけ。
pub fn query_all(
    doc: &Document,
    root: NodeId,
    list: &SelectorList,
    first_only: bool,
) -> Vec<NodeId> {
    let mut caches = SelectorCaches::default();
    let mut found = Vec::new();
    for node in doc.descendants(root) {
        if node != root && matches_with(doc, node, list, &mut caches) {
            found.push(node);
            if first_only {
                break;
            }
        }
    }
    found
}

/// `node` 自身から祖先へ向かって最初に一致する要素（`element.closest`）。
pub fn closest(doc: &Document, node: NodeId, list: &SelectorList) -> Option<NodeId> {
    let mut caches = SelectorCaches::default();
    let mut current = Some(node);
    while let Some(n) = current {
        if matches_with(doc, n, list, &mut caches) {
            return Some(n);
        }
        current = doc.parent(n);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selector::parse_selector_list;

    fn doc() -> Document {
        kumo_html::parse_html(
            "<div id=a class='box big'><p id=b>x</p><p id=c class=note><span id=d>y</span></p></div><p id=e></p>",
        )
    }

    fn find(doc: &Document, id: &str) -> NodeId {
        doc.descendants(doc.root())
            .find(|&n| doc.attr(n, "id") == Some(id))
            .unwrap()
    }

    fn ids(doc: &Document, nodes: &[NodeId]) -> Vec<String> {
        nodes
            .iter()
            .map(|&n| doc.attr(n, "id").unwrap_or("").to_owned())
            .collect()
    }

    #[test]
    fn queries_in_document_order() {
        let doc = doc();
        let list = parse_selector_list("p").unwrap();
        assert_eq!(
            ids(&doc, &query_all(&doc, doc.root(), &list, false)),
            ["b", "c", "e"]
        );
        assert_eq!(ids(&doc, &query_all(&doc, doc.root(), &list, true)), ["b"]);
        // 範囲は子孫だけ（自身は含まない）
        let a = find(&doc, "a");
        let div = parse_selector_list("div").unwrap();
        assert!(query_all(&doc, a, &div, false).is_empty());
        let compound = parse_selector_list("#a > .note span, p:first-child").unwrap();
        assert_eq!(
            ids(&doc, &query_all(&doc, doc.root(), &compound, false)),
            ["b", "d"]
        );
    }

    #[test]
    fn matches_and_closest() {
        let doc = doc();
        let d = find(&doc, "d");
        assert!(matches(&doc, d, &parse_selector_list("p > span").unwrap()));
        assert!(!matches(
            &doc,
            d,
            &parse_selector_list("div > span").unwrap()
        ));
        assert!(
            !matches(&doc, doc.root(), &parse_selector_list("*").unwrap()),
            "文書は要素ではない"
        );
        let big = parse_selector_list(".big").unwrap();
        assert_eq!(closest(&doc, d, &big), Some(find(&doc, "a")));
        assert_eq!(closest(&doc, find(&doc, "e"), &big), None);
        assert_eq!(
            closest(&doc, d, &parse_selector_list("span").unwrap()),
            Some(d),
            "自身も含む"
        );
    }
}
