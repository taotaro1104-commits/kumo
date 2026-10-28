//! ページの取得段階：文書・スタイルシート・画像を取得し、HTML をパースする。
//! フォントDBや画像キャッシュ（[`crate::Services`]）を使わないので、Content のメインスレッドとは
//! 別のスレッドで動かせる。取得を待つ間もメインスレッドは他のタブの描画や入力を処理できる。

use kumo_css::Stylesheet;
use kumo_dom::{Document, NodeId};
use kumo_html::{StyleSource, stylesheets};
use kumo_mem::{Subsystem, scope};
use kumo_net::process::Fetcher;
use kumo_net::protocol::{FetchError, ResourceKind};
use url::Url;

use crate::level::LoadLevel;
use crate::page::escape;

/// フォーム送信の本文の形式。
pub const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

/// 取得とパースを終えたページ（スタイル計算・レイアウトの前）。
pub struct FetchedPage {
    /// 最終 URL（リダイレクト後。要求の `#` を引き継ぐ）
    pub url: Url,
    pub doc: Document,
    pub stylesheets: Vec<Stylesheet>,
    /// 先に取得した画像（絶対 URL と圧縮データ。`None` は取得失敗）
    pub images: Vec<(String, Option<Vec<u8>>)>,
    /// 先に取得した外部スクリプトの本文（`None` は取得失敗・ブロック・大きすぎる）
    pub scripts: Vec<(NodeId, Option<String>)>,
}

/// 文書を取得してパースする。`post_body` があればフォームを POST で送る。
pub fn fetch_document(
    fetcher: &dyn Fetcher,
    url: &Url,
    post_body: Option<Vec<u8>>,
    level: LoadLevel,
) -> Result<FetchedPage, FetchError> {
    let response = match post_body {
        Some(body) => fetcher.post(url.as_str(), FORM_CONTENT_TYPE, body, None)?,
        None => fetcher.fetch(url.as_str(), ResourceKind::Document, None)?,
    };
    let mut final_url = Url::parse(&response.final_url).unwrap_or_else(|_| url.clone());
    if final_url.fragment().is_none() {
        final_url.set_fragment(url.fragment());
    }
    let html = document_html(&response.body, response.content_type.as_deref(), &final_url);
    drop(response);
    Ok(prepare(&html, final_url, fetcher, level))
}

/// HTML をパースし、外部スタイルシートと画像を取得する。
pub fn prepare(html: &str, url: Url, fetcher: &dyn Fetcher, level: LoadLevel) -> FetchedPage {
    let doc = {
        let _tag = scope(Subsystem::Dom);
        // パーサの一時データは専用ヒープに置き、完成した DOM だけを通常ヒープへ詰めて複製する
        kumo_mem::with_scratch_heap(|| {
            let parsed = kumo_html::parse_html(html);
            kumo_mem::with_long_lived(|| parsed.clone())
        })
    };
    let stylesheets = {
        let _tag = scope(Subsystem::Style);
        load_stylesheets(&doc, &url, fetcher, level)
    };
    let images = if level.images() {
        prefetch_images(&doc, &url, fetcher)
    } else {
        Vec::new()
    };
    let scripts = if level.javascript() {
        prefetch_scripts(&doc, &url, fetcher, level)
    } else {
        Vec::new()
    };
    FetchedPage {
        url,
        doc,
        stylesheets,
        images,
        scripts,
    }
}

/// レスポンス本文を HTML にする。画像やテキストは表示用の HTML で包む。
fn document_html(body: &[u8], content_type: Option<&str>, url: &Url) -> String {
    let mime = content_type
        .and_then(|c| c.split(';').next())
        .map(|m| m.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if mime.starts_with("image/") {
        return format!(
            "<body style='margin:0;background:#222'><img src='{}'></body>",
            escape(url.as_str())
        );
    }
    let text = String::from_utf8_lossy(body);
    if mime == "text/plain"
        || mime == "text/css"
        || mime.contains("javascript")
        || mime.ends_with("json")
    {
        return format!("<pre style='white-space:pre-wrap'>{}</pre>", escape(&text));
    }
    text.into_owned()
}

/// 外部スタイルシートをまとめて（Network プロセス側で並行に）取得する。
fn load_stylesheets(
    doc: &Document,
    base: &Url,
    fetcher: &dyn Fetcher,
    level: LoadLevel,
) -> Vec<Stylesheet> {
    let mut sources = stylesheets(doc);
    if !level.stylesheets() {
        sources.retain(|s| matches!(s, StyleSource::Inline(_)));
    }
    let external: Vec<(String, ResourceKind)> = sources
        .iter()
        .filter_map(|s| match s {
            StyleSource::External(href, _) => base.join(href.trim()).ok(),
            StyleSource::Inline(_) => None,
        })
        .map(|url| (url.to_string(), ResourceKind::Stylesheet))
        .collect();
    let mut fetched = fetcher
        .fetch_all(&external, Some(base.as_str()))
        .into_iter();
    sources
        .into_iter()
        .filter_map(|source| match source {
            StyleSource::Inline(css) => Some(Stylesheet::parse(&css)),
            StyleSource::External(href, media) => {
                if base.join(href.trim()).is_err() {
                    return None;
                }
                match fetched.next()? {
                    Ok(response) if response.status == 200 => {
                        let css = String::from_utf8_lossy(&response.body);
                        // `media` 属性は @media で包んで、スタイル計算のときに表示領域で評価する
                        Some(match media {
                            Some(media) => {
                                Stylesheet::parse(&format!("@media {media} {{ {css} }}"))
                            }
                            None => Stylesheet::parse(&css),
                        })
                    }
                    Ok(_) | Err(FetchError::Blocked) => None,
                    Err(e) => {
                        eprintln!("[css] {href}: {e}");
                        None
                    }
                }
            }
        })
        .collect()
}

/// 文書中の画像（`<img src>`・`<input type=image src>`）の絶対 URL。重複は除く。
pub fn image_urls(doc: &Document, base: &Url) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for node in doc.descendants(doc.root()) {
        let Some(element) = doc.element(node) else {
            continue;
        };
        let is_image = element.is_html("img")
            || (element.is_html("input")
                && element
                    .attr("type")
                    .is_some_and(|t| t.eq_ignore_ascii_case("image")));
        let Some(src) = element.attr("src").filter(|_| is_image) else {
            continue;
        };
        if let Ok(url) = base.join(src.trim()) {
            let url = url.to_string();
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
    }
    urls
}

/// 外部スクリプトをまとめて（Network プロセス側で並行に）取得する。
fn prefetch_scripts(
    doc: &Document,
    base: &Url,
    fetcher: &dyn Fetcher,
    level: LoadLevel,
) -> Vec<(NodeId, Option<String>)> {
    let max_bytes = level.script_limits().max_bytes;
    let external: Vec<(NodeId, String)> = kumo_js::scripts_in_order(doc, base)
        .into_iter()
        .filter_map(|(node, info)| info.src.map(|src| (node, src)))
        .take(level.max_scripts())
        .collect();
    if external.is_empty() {
        return Vec::new();
    }
    let requests: Vec<(String, ResourceKind)> = external
        .iter()
        .map(|(_, url)| (url.clone(), ResourceKind::Script))
        .collect();
    let results = fetcher.fetch_all(&requests, Some(base.as_str()));
    external
        .into_iter()
        .zip(results)
        .map(|((node, url), result)| {
            let body = match result {
                Ok(response) if response.status == 200 && response.body.len() <= max_bytes => {
                    Some(String::from_utf8_lossy(&response.body).into_owned())
                }
                Ok(response) => {
                    eprintln!(
                        "[js] {url}: 実行しません（状態 {}、{} バイト）",
                        response.status,
                        response.body.len()
                    );
                    None
                }
                Err(FetchError::Blocked) => None,
                Err(e) => {
                    eprintln!("[js] {url}: {e}");
                    None
                }
            };
            (node, body)
        })
        .collect()
}

fn prefetch_images(
    doc: &Document,
    base: &Url,
    fetcher: &dyn Fetcher,
) -> Vec<(String, Option<Vec<u8>>)> {
    let urls = image_urls(doc, base);
    let requests: Vec<(String, ResourceKind)> = urls
        .iter()
        .map(|u| (u.clone(), ResourceKind::Image))
        .collect();
    let results = fetcher.fetch_all(&requests, Some(base.as_str()));
    urls.into_iter()
        .zip(results)
        .map(|(url, result)| {
            let data = match result {
                Ok(response) if response.status == 200 => Some(response.body),
                Ok(_) | Err(FetchError::Blocked) => None,
                Err(e) => {
                    eprintln!("[image] {url}: {e}");
                    None
                }
            };
            (url, data)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_non_html_bodies() {
        let url = Url::parse("https://a.example/x.png").unwrap();
        assert!(
            document_html(b"", Some("image/png"), &url)
                .contains("<img src='https://a.example/x.png'>")
        );
        let text = document_html(b"<b>", Some("text/plain; charset=utf-8"), &url);
        assert!(text.contains("&lt;b&gt;"));
        assert_eq!(document_html(b"<p>x", None, &url), "<p>x");
    }

    #[test]
    fn collects_unique_image_urls() {
        let doc = kumo_html::parse_html(
            "<img src=a.png><img src='a.png'><input type=IMAGE src=b.png><input src=c.png><img>",
        );
        let base = Url::parse("https://a.example/dir/").unwrap();
        assert_eq!(
            image_urls(&doc, &base),
            vec!["https://a.example/dir/a.png", "https://a.example/dir/b.png"]
        );
    }
}
