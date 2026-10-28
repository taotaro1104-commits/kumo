//! `about:bookmarks`：ブックマークの一覧ページ。削除は `kumo:unbookmark?u=URL` のリンクで行う。

use kumo_content::escape;
use url::form_urlencoded;

use crate::bookmarks::Bookmark;

pub const BOOKMARKS_URL: &str = "about:bookmarks";
/// 削除リンクの接頭辞（ページ内の `kumo:` リンク）。
pub(crate) const UNBOOKMARK_PREFIX: &str = "kumo:unbookmark?u=";

const CSS: &str = "
html, body { margin: 0; background: #f6f7f9; color: #1f2328; font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; }
.wrap { max-width: 860px; margin: 0 auto; padding: 36px 24px; }
h1 { margin: 0 0 20px; font-size: 26px; color: #1a73e8; }
.box { background: #ffffff; border: 1px solid #e1e4e8; border-radius: 12px; padding: 4px 16px; }
.row { display: flex; align-items: center; padding: 10px 0; border-bottom: 1px solid #eef0f2; }
.row:last-child { border-bottom: 0; }
.main { flex: 1; overflow: hidden; }
a.t { display: block; color: #1f2328; font-size: 14px; font-weight: 600; text-decoration: none; white-space: nowrap; overflow: hidden; }
.u { color: #6b7280; font-size: 12px; white-space: nowrap; overflow: hidden; }
a.rm { flex: none; padding: 4px 10px; border-radius: 12px; color: #b3261e; font-size: 12px; text-decoration: none; }
.empty { padding: 28px; text-align: center; color: #6b7280; font-size: 14px; background: #ffffff;
         border: 1px dashed #d5d9de; border-radius: 12px; }
";

pub fn bookmarks_html(list: &[Bookmark]) -> String {
    let mut html = format!(
        "<html><head><title>ブックマーク</title><style>{CSS}</style></head><body><div class=wrap><h1>ブックマーク</h1>"
    );
    if list.is_empty() {
        html.push_str(
            "<div class=empty>まだありません。ページを開いて Ctrl+D（または ☆）で追加できます。</div>",
        );
    } else {
        html.push_str("<div class=box>");
        for b in list {
            let title = if b.title.trim().is_empty() {
                &b.url
            } else {
                &b.title
            };
            let enc: String = form_urlencoded::byte_serialize(b.url.as_bytes()).collect();
            html.push_str(&format!(
                "<div class=row><div class=main><a class=t href='{url}'>{title}</a><div class=u>{url}</div></div>\
                 <a class=rm href='{UNBOOKMARK_PREFIX}{enc}'>削除</a></div>",
                url = escape(&b.url),
                title = escape(title),
            ));
        }
        html.push_str("</div>");
    }
    html.push_str("</div></body></html>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_escaped_entries_with_remove_links() {
        let html = bookmarks_html(&[
            Bookmark {
                url: "https://a.example/?q=1&r=<2>".into(),
                title: "A <b>".into(),
                added: 1,
            },
            Bookmark {
                url: "https://b.example/".into(),
                title: String::new(),
                added: 0,
            },
        ]);
        assert!(html.contains("A &lt;b&gt;"));
        assert!(!html.contains("<b>"));
        assert!(html.contains("kumo:unbookmark?u=https%3A%2F%2Fb.example%2F"));
        assert_eq!(html.matches("class=row").count(), 2);
    }

    #[test]
    fn empty_state_guides() {
        assert!(bookmarks_html(&[]).contains("Ctrl+D"));
    }
}
