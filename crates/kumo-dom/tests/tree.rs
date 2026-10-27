use kumo_dom::{Attr, Document, LocalName, Node, NodeKind, QualName, local_name, ns};

fn html(name: &str) -> QualName {
    QualName::new(None, ns!(html), LocalName::from(name))
}

fn el(doc: &mut Document, name: &str) -> kumo_dom::NodeId {
    doc.create_element(html(name), Vec::new())
}

#[test]
fn new_document_has_root_only() {
    let doc = Document::new();
    assert_eq!(doc.len(), 1);
    assert_eq!(doc.node(doc.root()).kind(), NodeKind::Document);
    assert!(doc.first_child(doc.root()).is_none());
}

#[test]
fn append_builds_sibling_chain() {
    let mut doc = Document::new();
    let root = doc.root();
    let a = el(&mut doc, "a");
    let b = el(&mut doc, "b");
    let c = el(&mut doc, "c");
    doc.append(root, a);
    doc.append(root, b);
    doc.append(root, c);

    assert_eq!(doc.children(root).collect::<Vec<_>>(), vec![a, b, c]);
    assert_eq!(doc.parent(b), Some(root));
    assert_eq!(doc.prev_sibling(c), Some(b));
    assert_eq!(doc.last_child(root), Some(c));
}

#[test]
fn insert_before_and_detach() {
    let mut doc = Document::new();
    let root = doc.root();
    let a = el(&mut doc, "a");
    let c = el(&mut doc, "c");
    let b = el(&mut doc, "b");
    doc.append(root, a);
    doc.append(root, c);
    doc.insert_before(c, b);
    assert_eq!(doc.children(root).collect::<Vec<_>>(), vec![a, b, c]);

    doc.detach(a);
    assert_eq!(doc.children(root).collect::<Vec<_>>(), vec![b, c]);
    assert_eq!(doc.parent(a), None);
    assert_eq!(doc.prev_sibling(b), None);

    doc.detach(c);
    assert_eq!(doc.last_child(root), Some(b));
}

#[test]
fn append_moves_node_from_old_parent() {
    let mut doc = Document::new();
    let root = doc.root();
    let p = el(&mut doc, "p");
    let q = el(&mut doc, "q");
    let x = el(&mut doc, "x");
    doc.append(root, p);
    doc.append(root, q);
    doc.append(p, x);
    doc.append(q, x);
    assert_eq!(doc.first_child(p), None);
    assert_eq!(doc.parent(x), Some(q));
}

#[test]
fn append_text_merges_adjacent_text() {
    let mut doc = Document::new();
    let root = doc.root();
    let p = el(&mut doc, "p");
    doc.append(root, p);
    doc.append_text(p, "Hello, ");
    doc.append_text(p, "world");
    let kids: Vec<_> = doc.children(p).collect();
    assert_eq!(kids.len(), 1);
    assert_eq!(doc.text(kids[0]), Some("Hello, world"));
}

#[test]
fn insert_text_before_merges_with_previous_text() {
    let mut doc = Document::new();
    let root = doc.root();
    let b = el(&mut doc, "b");
    doc.append_text(root, "x");
    doc.append(root, b);
    doc.insert_text_before(b, "y");
    let first = doc.first_child(root).unwrap();
    assert_eq!(doc.text(first), Some("xy"));
    assert_eq!(doc.children(root).count(), 2);
}

#[test]
fn descendants_preorder_stays_inside_subtree() {
    let mut doc = Document::new();
    let root = doc.root();
    let a = el(&mut doc, "a");
    let b = el(&mut doc, "b");
    let c = el(&mut doc, "c");
    let d = el(&mut doc, "d");
    doc.append(root, a);
    doc.append(a, b);
    doc.append(b, c);
    doc.append(root, d);
    assert_eq!(doc.descendants(a).collect::<Vec<_>>(), vec![a, b, c]);
    assert_eq!(doc.descendants(root).count(), 5);
}

#[test]
fn attributes_and_classes() {
    let mut doc = Document::new();
    let attrs = vec![
        Attr::new(local_name!("id"), "main"),
        Attr::new(local_name!("class"), " foo  bar "),
        Attr::new(local_name!("href"), "/x"),
    ];
    let a = doc.create_element(html("a"), attrs);
    let data = doc.element(a).unwrap();
    assert_eq!(data.id(), Some("main"));
    assert!(data.has_class("bar"));
    assert!(!data.has_class("baz"));
    assert_eq!(doc.attr(a, "href"), Some("/x"));
    assert!(data.is_html("a"));

    doc.add_attrs_if_missing(
        a,
        vec![
            Attr::new(local_name!("id"), "x"),
            Attr::new(local_name!("title"), "t"),
        ],
    );
    assert_eq!(doc.attr(a, "id"), Some("main"));
    assert_eq!(doc.attr(a, "title"), Some("t"));
}

#[test]
fn text_content_concatenates_descendant_text() {
    let mut doc = Document::new();
    let root = doc.root();
    let p = el(&mut doc, "p");
    let b = el(&mut doc, "b");
    doc.append(root, p);
    doc.append_text(p, "a ");
    doc.append(p, b);
    doc.append_text(b, "bold");
    doc.append_text(p, " z");
    assert_eq!(doc.text_content(p), "a bold z");
}

#[test]
fn node_stays_small() {
    // 設計書の目標：ノード本体は約32バイト
    assert!(
        std::mem::size_of::<Node>() <= 32,
        "{}",
        std::mem::size_of::<Node>()
    );
}
