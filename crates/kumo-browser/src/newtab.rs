//! 新しいタブのページ：何をすればよいかの案内と、よく見るページのカード。

use kumo_content::escape;
use kumo_content::protocol::{TabId, ToContent};
use url::Url;

use crate::about_memory::ABOUT_MEMORY_URL;
use crate::bookmarks_page::BOOKMARKS_URL;
use crate::browser::Browser;
use crate::history::HistoryEntry;
use crate::settings_page::SETTINGS_URL;
use crate::url_display::format_url;

/// 新しいタブのページの URL（Browser が HTML を作って Content に渡す）。
pub const NEW_TAB_URL: &str = "about:newtab";
/// カードに並べる数。
pub const FREQUENT_SITES: usize = 8;

const CSS: &str = "
html, body { margin: 0; background: #f6f7f9; color: #1f2328; font-family: 'Segoe UI', 'Yu Gothic UI', sans-serif; }
.wrap { max-width: 760px; margin: 0 auto; padding: 64px 24px 40px; }
h1 { margin: 0; text-align: center; font-size: 44px; font-weight: 600; color: #1a73e8; }
.lead { display: flex; justify-content: center; align-items: center; gap: 6px; margin: 10px 0 36px;
        color: #4a5058; font-size: 15px; }
.key { display: inline-block; line-height: 18px; padding: 0 7px; background: #ffffff; border: 1px solid #d5d9de; border-radius: 6px; color: #3c4043;
       font-size: 13px; }
h2 { margin: 0 0 12px; font-size: 14px; font-weight: 600; color: #4a5058; }
.grid { display: flex; flex-wrap: wrap; gap: 12px; }
.card { display: block; box-sizing: border-box; width: 175px; height: 92px; padding: 14px; background: #ffffff;
        border: 1px solid #e1e4e8; border-radius: 12px; text-decoration: none; overflow: hidden; }
.ic { width: 28px; height: 28px; line-height: 28px; border-radius: 14px; background: #e8f0fe; color: #1a56c4;
      text-align: center; font-size: 14px; font-weight: 700; }
.t { margin-top: 8px; color: #1f2328; font-size: 13px; font-weight: 600; white-space: nowrap; overflow: hidden; }
.u { margin-top: 2px; color: #6b7280; font-size: 12px; white-space: nowrap; overflow: hidden; }
.empty { padding: 28px; text-align: center; color: #6b7280; font-size: 14px; background: #ffffff;
         border: 1px dashed #d5d9de; border-radius: 12px; }
.keys { margin-top: 40px; display: flex; flex-wrap: wrap; gap: 8px 20px; color: #6b7280; font-size: 13px; }
.set { color: #1a56c4; text-decoration: none; }
.k { display: flex; align-items: center; gap: 6px; }
";

/// 使い方の早見（キーと説明）。
const SHORTCUTS: [(&str, &str); 8] = [
    ("Alt+Home", "ホームへ"),
    ("Ctrl+D", "ブックマーク"),
    ("Ctrl+B", "ブックマーク一覧"),
    ("Ctrl+L", "URL 欄へ"),
    ("Ctrl+T", "新しいタブ"),
    ("Ctrl+W", "タブを閉じる"),
    ("Alt+←", "戻る"),
    ("Ctrl+クリック", "リンクを背景のタブで開く"),
];

/// カードの題名の幅（半角1・全角2で数える）。CSS の text-overflow は使えないので先に切る。
const CARD_TITLE_WIDTH: usize = 19;

/// 表示幅が `max` を超えたら切って「…」を付ける。
fn shorten(text: &str, max: usize) -> String {
    let mut width = 0;
    let mut out = String::new();
    for c in text.chars() {
        width += if c.is_ascii() { 1 } else { 2 };
        if width > max {
            out.push('…');
            return out;
        }
        out.push(c);
    }
    out
}

fn card(entry: &HistoryEntry) -> String {
    let parts = format_url(&entry.url);
    let label = if parts.host.is_empty() {
        parts.rest.clone()
    } else {
        parts.host.clone()
    };
    let initial = label
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "•".to_owned());
    let title = if entry.visit.title.is_empty() {
        label.clone()
    } else {
        entry.visit.title.clone()
    };
    let title = shorten(&title, CARD_TITLE_WIDTH);
    format!(
        "<a class=card href='{}'><div class=ic>{}</div><div class=t>{}</div><div class=u>{}</div></a>",
        escape(&entry.url),
        escape(&initial),
        escape(&title),
        escape(&label)
    )
}

/// 新しいタブのページの HTML。`frequent` はよく見るページ（多い順）。
pub fn new_tab_html(frequent: &[HistoryEntry]) -> String {
    let mut html = format!(
        "<html><head><title>新しいタブ</title><style>{CSS}</style></head><body><div class=wrap>\
         <h1>Kumo</h1><div class=lead><span>上の欄に URL か調べたい言葉を入力してください</span><span class=key>Ctrl+L</span></div>\
         <h2>よく見るページ</h2>"
    );
    let cards: Vec<String> = frequent
        .iter()
        .filter(|e| e.url.starts_with("http") || e.url.starts_with("file"))
        .take(FREQUENT_SITES)
        .map(card)
        .collect();
    if cards.is_empty() {
        html.push_str(
            "<div class=empty>まだ履歴がありません。開いたページがここに並びます。</div>",
        );
    } else {
        html.push_str("<div class=grid>");
        html.push_str(&cards.concat());
        html.push_str("</div>");
    }
    html.push_str("<div class=keys>");
    for (key, label) in SHORTCUTS {
        html.push_str(&format!(
            "<span class=k><span class=key>{key}</span><span>{label}</span></span>"
        ));
    }
    html.push_str(
        "</div><div class=keys><a class=set href='about:settings'>設定</a>         <a class=set href='about:bookmarks'>ブックマーク</a>         <a class=set href='about:memory'>メモリ</a></div></div></body></html>",
    );
    html
}

impl Browser {
    /// 読み込みの依頼。新しいタブのページは履歴から HTML を作って渡す（取得しない）。
    pub(crate) fn load_message(
        &self,
        tab: TabId,
        nav: u64,
        url: &Url,
        post_body: Option<Vec<u8>>,
    ) -> ToContent {
        if url.as_str() == NEW_TAB_URL {
            let frequent = self
                .history
                .as_ref()
                .map(|h| h.search("", FREQUENT_SITES * 2))
                .unwrap_or_default();
            return ToContent::LoadHtml {
                tab,
                nav,
                url: url.to_string(),
                html: new_tab_html(&frequent),
            };
        }
        if url.as_str() == SETTINGS_URL {
            let home = self
                .history
                .as_ref()
                .and_then(|h| h.home_url())
                .unwrap_or_default();
            return ToContent::LoadHtml {
                tab,
                nav,
                url: url.to_string(),
                html: crate::settings_page::settings_html(&home, self.load_level, self.images),
            };
        }
        if url.as_str() == BOOKMARKS_URL {
            let list = self
                .history
                .as_ref()
                .map(|h| h.bookmarks())
                .unwrap_or_default();
            return ToContent::LoadHtml {
                tab,
                nav,
                url: url.to_string(),
                html: crate::bookmarks_page::bookmarks_html(&list),
            };
        }
        if url.as_str() == ABOUT_MEMORY_URL {
            return ToContent::LoadHtml {
                tab,
                nav,
                url: url.to_string(),
                html: crate::about_memory::memory_html(&self.memory_report()),
            };
        }
        ToContent::Load {
            tab,
            nav,
            url: url.to_string(),
            post_body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Visit;

    fn entry(url: &str, title: &str) -> HistoryEntry {
        HistoryEntry {
            url: url.to_owned(),
            visit: Visit {
                title: title.to_owned(),
                last_visit: 0,
                count: 1,
            },
        }
    }

    #[test]
    fn cards_show_title_domain_and_initial() {
        let html = new_tab_html(&[
            entry("https://www.rust-lang.org/learn", "Learn <Rust>"),
            entry("https://example.com/", ""),
            entry("about:blank", "skip"),
        ]);
        assert!(html.contains("<div class=ic>R</div>"), "ドメインの頭文字");
        assert!(html.contains("Learn &lt;Rust&gt;"));
        assert!(html.contains("<div class=u>rust-lang.org</div>"));
        assert!(
            html.contains("<div class=t>example.com</div>"),
            "題名がなければドメイン"
        );
        assert!(!html.contains("skip"), "about: は並べない");
        assert!(html.contains("Ctrl+T"));
    }

    #[test]
    fn long_titles_are_shortened_by_display_width() {
        assert_eq!(shorten("short", 19), "short");
        assert_eq!(
            shorten("Rust (programming language) - Wikipedia", 19),
            "Rust (programming l…"
        );
        assert_eq!(shorten("プログラミング言語の一覧です", 10), "プログラミ…");
    }

    #[test]
    fn limits_the_number_of_cards() {
        let many: Vec<HistoryEntry> = (0..20)
            .map(|i| entry(&format!("https://s{i}.example/"), "x"))
            .collect();
        assert_eq!(
            new_tab_html(&many).matches("class=card").count(),
            FREQUENT_SITES
        );
    }
}
