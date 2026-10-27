use kumo_dom::{Document, NodeId};

/// ドキュメントが参照するスタイルシート。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleSource {
    /// `<style>` の中身
    Inline(String),
    /// `<link rel=stylesheet href=…>` の href（未解決のまま）
    External(String),
}

/// `<style>` と `<link rel=stylesheet>` を文書順に集める。
pub fn stylesheets(doc: &Document) -> Vec<StyleSource> {
    doc.descendants(doc.root())
        .filter_map(|node| style_source(doc, node))
        .collect()
}

fn style_source(doc: &Document, node: NodeId) -> Option<StyleSource> {
    let element = doc.element(node)?;
    if element.is_html("style") {
        return Some(StyleSource::Inline(doc.text_content(node)));
    }
    if element.is_html("link") && is_stylesheet_rel(element.attr("rel")?) {
        return element
            .attr("href")
            .map(|href| StyleSource::External(href.to_owned()));
    }
    None
}

fn is_stylesheet_rel(rel: &str) -> bool {
    rel.split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case("stylesheet"))
}
