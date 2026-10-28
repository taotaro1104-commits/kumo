use kumo_dom::{Document, NodeId};

fn find(doc: &Document, id: &str) -> NodeId {
    doc.descendants(doc.root())
        .find(|&n| doc.attr(n, "id") == Some(id))
        .unwrap_or_else(|| panic!("#{id}"))
}

/// kumo-html に依存せず、手で小さな木を作る。
fn sample(text: &str) -> Document {
    let mut doc = Document::new();
    let root = doc.root();
    let div = doc.create_html_element("DIV");
    doc.set_attr(div, "id", "a");
    doc.set_attr(div, "title", "q\"<&");
    doc.append(root, div);
    let p = doc.create_html_element("p");
    doc.set_attr(p, "id", "p");
    doc.append(div, p);
    doc.append_text(p, text);
    let img = doc.create_html_element("img");
    doc.set_attr(img, "src", "x.png");
    doc.append(div, img);
    let script = doc.create_html_element("script");
    doc.append_text(script, "if (a < b && c) {}");
    doc.append(div, script);
    doc
}

#[test]
fn elements_are_lowercased_and_text_is_replaceable() {
    let mut doc = sample("hello");
    let div = find(&doc, "a");
    assert_eq!(&*doc.element(div).unwrap().local, "div");
    let text = doc.first_child(find(&doc, "p")).unwrap();
    assert!(doc.set_text_data(text, "bye <b>"));
    assert_eq!(doc.text_content(find(&doc, "p")), "bye <b>");
    assert!(!doc.set_text_data(div, "x"), "要素はテキストではない");
}

#[test]
fn serializes_html_with_escaping_void_and_raw_text() {
    let doc = sample("1 < 2 & 3");
    let div = find(&doc, "a");
    assert_eq!(
        doc.inner_html(div),
        "<p id=\"p\">1 &lt; 2 &amp; 3</p><img src=\"x.png\"><script>if (a < b && c) {}</script>"
    );
    assert_eq!(
        doc.outer_html(find(&doc, "p")),
        "<p id=\"p\">1 &lt; 2 &amp; 3</p>"
    );
    assert!(
        doc.outer_html(div)
            .starts_with("<div id=\"a\" title=\"q&quot;<&amp;\">")
    );
}

#[test]
fn clone_import_and_clear() {
    let mut doc = sample("hello");
    let div = find(&doc, "a");
    let shallow = doc.clone_node(div, false).unwrap();
    assert_eq!(doc.parent(shallow), None);
    assert!(doc.first_child(shallow).is_none());
    let deep = doc.clone_node(div, true).unwrap();
    assert_eq!(doc.inner_html(deep), doc.inner_html(div));
    assert_eq!(doc.attr(deep, "id"), Some("a"), "属性も複製する");

    let mut other = Document::new();
    let imported = other.import_subtree(&doc, div).unwrap();
    assert_eq!(other.outer_html(imported), doc.outer_html(div));
    doc.clear_children(div);
    assert!(doc.first_child(div).is_none());
    assert!(doc.inner_html(deep).len() > 10, "複製は影響を受けない");
}

#[test]
fn inserting_a_node_before_itself_changes_nothing() {
    let mut doc = sample("x");
    let div = find(&doc, "a");
    let p = find(&doc, "p");
    let before = doc.inner_html(div);
    doc.insert_before(p, p);
    assert_eq!(doc.inner_html(div), before);
    assert_eq!(doc.parent(p), Some(div));
}
