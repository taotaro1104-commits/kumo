use kumo_dom::NodeKind;
use kumo_html::{StyleSource, parse_html, stylesheets};

fn find(doc: &kumo_dom::Document, tag: &str) -> Vec<kumo_dom::NodeId> {
    doc.descendants(doc.root())
        .filter(|&n| doc.element(n).is_some_and(|e| e.is_html(tag)))
        .collect()
}

#[test]
fn builds_implied_html_head_body() {
    let doc = parse_html("<p>Hello <b>world</b>");
    assert_eq!(find(&doc, "html").len(), 1);
    assert_eq!(find(&doc, "head").len(), 1);
    let body = find(&doc, "body")[0];
    let p = doc.first_child(body).unwrap();
    assert!(doc.element(p).unwrap().is_html("p"));
    assert_eq!(doc.text_content(p), "Hello world");
}

#[test]
fn doctype_and_comments_are_nodes() {
    let doc = parse_html("<!DOCTYPE html><!-- hi --><html><body></body></html>");
    let kinds: Vec<_> = doc
        .children(doc.root())
        .map(|n| doc.node(n).kind())
        .collect();
    assert_eq!(kinds[0], NodeKind::Doctype);
    assert_eq!(kinds[1], NodeKind::Comment);
}

#[test]
fn misnested_tags_are_fixed_up() {
    // アダプションエージェンシーで <i> が分割される
    let doc = parse_html("<p><b>1<i>2</b>3</i></p>");
    let p = find(&doc, "p")[0];
    assert_eq!(doc.text_content(p), "123");
    assert_eq!(find(&doc, "i").len(), 2);
}

#[test]
fn table_foster_parenting_moves_text_before_table() {
    let doc = parse_html("<table>x<tr><td>y</td></tr></table>");
    let body = find(&doc, "body")[0];
    let first = doc.first_child(body).unwrap();
    assert_eq!(doc.text(first), Some("x"));
}

#[test]
fn attributes_are_preserved() {
    let doc = parse_html(r#"<a href="/wiki" class="x y">l</a>"#);
    let a = find(&doc, "a")[0];
    assert_eq!(doc.attr(a, "href"), Some("/wiki"));
    assert!(doc.element(a).unwrap().has_class("y"));
}

#[test]
fn template_contents_are_not_in_tree() {
    let doc = parse_html("<template><p>hidden</p></template><p>shown</p>");
    let body = find(&doc, "body")[0];
    assert_eq!(doc.text_content(body), "shown");
}

#[test]
fn collects_stylesheets_in_document_order() {
    let doc = parse_html(
        r#"<head><link rel="stylesheet" href="a.css"><style>p{color:red}</style>
           <link rel="icon" href="x.ico"><link rel="Stylesheet preload" href="b.css"></head>"#,
    );
    assert_eq!(
        stylesheets(&doc),
        vec![
            StyleSource::External("a.css".into(), None),
            StyleSource::Inline("p{color:red}".into()),
            StyleSource::External("b.css".into(), None),
        ]
    );
}

#[test]
fn bytes_are_decoded_lossily() {
    let doc = kumo_html::parse_html_bytes(b"<p>caf\xc3\xa9 \xff</p>");
    let p = find(&doc, "p")[0];
    assert_eq!(doc.text_content(p), "café \u{FFFD}");
}

#[test]
fn media_attributes_wrap_stylesheets_in_media_rules() {
    let doc = kumo_html::parse_html(
        "<link rel=stylesheet href=p.css media=print><style media='screen and (min-width: 600px)'>p{color:red}</style>\
         <link rel=stylesheet href=a.css media=all><style>b{}</style>",
    );
    assert_eq!(
        kumo_html::stylesheets(&doc),
        vec![
            StyleSource::External("p.css".into(), Some("print".into())),
            StyleSource::Inline("@media screen and (min-width: 600px) { p{color:red} }".into()),
            StyleSource::External("a.css".into(), None),
            StyleSource::Inline("b{}".into()),
        ]
    );
}
