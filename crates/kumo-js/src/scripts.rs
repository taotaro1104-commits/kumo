//! `<script>` の見つけ方。Boa に触れないので、取得スレッド（`Send` でない `JsRuntime` を持てない場所）からも使える。

use kumo_dom::{Document, NodeId};
use url::Url;

/// 実行できる `<script>` の情報。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptInfo {
    /// `src`（文書の URL を基準にした絶対 URL）
    pub src: Option<String>,
    /// インラインのスクリプト本文
    pub text: String,
}

/// `type` が JavaScript か（空・省略も JavaScript）。モジュールやデータブロックは実行しない。
fn is_javascript(kind: &str) -> bool {
    let kind = kind.trim().to_ascii_lowercase();
    kind.is_empty()
        || kind.contains("javascript")
        || kind.contains("ecmascript")
        || kind == "text/jscript"
        || kind == "application/x-javascript"
}

/// `node` が実行できる `<script>` なら、その情報。空のものや実行できない種類は `None`。
pub fn script_info(doc: &Document, base: &Url, node: NodeId) -> Option<ScriptInfo> {
    let element = doc.element(node).filter(|e| e.is_html("script"))?;
    if !is_javascript(element.attr("type").unwrap_or_default()) {
        return None;
    }
    let src = element
        .attr("src")
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| base.join(s.trim()).ok())
        .map(String::from);
    let text = doc.text_content(node);
    (src.is_some() || !text.trim().is_empty()).then_some(ScriptInfo { src, text })
}

/// 文書中の実行できる `<script>` を文書順に返す。
pub fn scripts_in_order(doc: &Document, base: &Url) -> Vec<(NodeId, ScriptInfo)> {
    doc.descendants(doc.root())
        .filter_map(|n| script_info(doc, base, n).map(|info| (n, info)))
        .collect()
}

/// 文書に JS が必要か：実行できるスクリプトか、`onclick="…"` のようなイベント属性がある。
pub fn needs_js(doc: &Document, base: &Url) -> bool {
    doc.descendants(doc.root()).any(|n| {
        script_info(doc, base, n).is_some()
            || doc.element(n).is_some_and(|e| {
                e.attrs
                    .iter()
                    .any(|a| a.name.starts_with("on") && a.name.len() > 2)
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_runnable_scripts_in_order() {
        let doc = kumo_html::parse_html(
            "<script>var a=1</script><script src=/x.js></script><script type=module>m</script>\
             <script type='application/ld+json'>{}</script><script> </script><script type=text/javascript>b</script>",
        );
        let base = Url::parse("https://example.com/dir/p.html").unwrap();
        let found = scripts_in_order(&doc, &base);
        let summary: Vec<(Option<&str>, &str)> = found
            .iter()
            .map(|(_, i)| (i.src.as_deref(), i.text.as_str()))
            .collect();
        assert_eq!(
            summary,
            vec![
                (None, "var a=1"),
                (Some("https://example.com/x.js"), ""),
                (None, "b")
            ]
        );
    }

    #[test]
    fn detects_when_js_is_needed() {
        let base = Url::parse("https://example.com/").unwrap();
        let yes = |html: &str| needs_js(&kumo_html::parse_html(html), &base);
        assert!(yes("<p>x</p><script>1</script>"));
        assert!(yes("<button onclick='go()'>x</button>"));
        assert!(!yes(
            "<p>plain</p><script type=module>m</script><a href=/online>only</a>"
        ));
    }
}
