use kumo_dom::{Document, NodeId};

/// ドキュメントが参照するスタイルシート。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleSource {
    /// `<style>` の中身。`media` 属性があるときは `@media` で包んである
    Inline(String),
    /// `<link rel=stylesheet href=…>` の href（未解決のまま）と `media` 属性
    External(String, Option<String>),
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
        let css = doc.text_content(node);
        return Some(StyleSource::Inline(match media_of(element.attr("media")) {
            Some(media) => format!("@media {media} {{ {css} }}"),
            None => css,
        }));
    }
    if element.is_html("link") && is_stylesheet_rel(element.attr("rel")?) {
        return element
            .attr("href")
            .map(|href| StyleSource::External(href.to_owned(), media_of(element.attr("media"))));
    }
    None
}

/// `media` 属性（空・`all` は条件なし）。
fn media_of(media: Option<&str>) -> Option<String> {
    let media = media?.trim();
    (!media.is_empty() && !media.eq_ignore_ascii_case("all")).then(|| media.to_owned())
}

fn is_stylesheet_rel(rel: &str) -> bool {
    rel.split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case("stylesheet"))
}
