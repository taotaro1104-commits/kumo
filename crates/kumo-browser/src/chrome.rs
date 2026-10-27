//! ブラウザ UI（タブ列・ナビゲーション・URL 欄）。設計書どおり第2の UI ツールキットを使わず、
//! 自エンジンで描く HTML/CSS として生成する。操作は `kumo:` スキームのリンクで表す。
//! `:hover` は使えないので、マウスが乗っているリンクには Browser が `hover` クラスを付けて作り直す。

use url::Url;

use kumo_content::escape;

use crate::url_display::format_url;

/// UI の高さ（px）。コンテンツはこの下に描く。
pub const CHROME_HEIGHT: f32 = 76.0;
/// タブ見出しの最大文字数。
const TAB_TITLE_CHARS: usize = 28;

/// UI のリンクが表す操作。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChromeAction {
    ActivateTab(usize),
    CloseTab(usize),
    NewTab,
    Back,
    Forward,
    Reload,
    FocusUrlBar,
}

impl ChromeAction {
    pub fn from_href(href: &str) -> Option<Self> {
        let rest = href.strip_prefix("kumo:")?;
        let index = |s: &str| s.parse::<usize>().ok();
        Some(match rest.split_once('/') {
            Some(("tab", i)) => Self::ActivateTab(index(i)?),
            Some(("close", i)) => Self::CloseTab(index(i)?),
            _ => match rest {
                "new" => Self::NewTab,
                "back" => Self::Back,
                "forward" => Self::Forward,
                "reload" => Self::Reload,
                "url" => Self::FocusUrlBar,
                _ => return None,
            },
        })
    }
}

/// タブ見出しに出す情報。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabLabel {
    pub title: String,
    pub loading: bool,
}

/// UI の描画に必要な状態。
pub struct ChromeState<'a> {
    pub tabs: Vec<TabLabel>,
    pub active: usize,
    pub url_text: &'a str,
    pub url_focused: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub loading: bool,
    /// Content プロセスが終了して作り直せなかった
    pub content_lost: bool,
    /// マウスが乗っている UI のリンク
    pub hover: Option<&'a str>,
}

const CSS: &str = "
html, body { margin: 0; background: #e3e6ea; color: #1f2328;
             font-family: 'Segoe UI', 'Yu Gothic UI', system-ui, sans-serif; font-size: 13px; }
.tabs { display: flex; box-sizing: border-box; height: 38px; padding: 6px 8px 0; gap: 4px; align-items: flex-end; }
.tab { display: flex; align-items: center; flex: 0 1 220px; min-width: 72px; height: 32px;
       padding: 0 4px 0 12px; border-radius: 9px 9px 0 0; overflow: hidden; }
.tab.hover { background: #d4d8de; }
.tab.active { background: #ffffff; }
.tab a.title { flex: 1; color: #4a5058; text-decoration: none; white-space: nowrap; overflow: hidden; }
.tab.active a.title { color: #1f2328; font-weight: 600; }
.dot { display: inline-block; width: 14px; color: #1a73e8; font-size: 10px; }
.tab a.close { flex: none; width: 20px; height: 20px; line-height: 20px; border-radius: 10px; text-align: center;
               color: #6b7280; text-decoration: none; font-size: 11px; }
.tab a.close.hover { background: #c5cad1; color: #1f2328; }
a.new { flex: none; width: 28px; height: 28px; line-height: 28px; margin-bottom: 2px; border-radius: 14px;
        text-align: center; color: #4a5058; text-decoration: none; font-size: 16px; }
a.new.hover { background: #d4d8de; }
.nav { display: flex; box-sizing: border-box; align-items: center; height: 38px; padding: 0 8px; gap: 4px;
       background: #ffffff; border-bottom: 1px solid #d5d9de; }
.nav.loading { border-bottom: 2px solid #1a73e8; }
.nav a.btn { flex: none; width: 30px; height: 30px; line-height: 30px; border-radius: 15px; text-align: center;
             color: #3c4043; text-decoration: none; font-size: 16px; }
.nav a.btn.hover { background: #eceef1; }
.nav a.btn.off { color: #c3c7cc; background: #ffffff; }
a.url { flex: 1; display: flex; box-sizing: border-box; align-items: center; height: 30px; padding: 0 14px 0 4px;
        background: #f1f3f4; border: 1px solid #f1f3f4; border-radius: 15px; text-decoration: none;
        white-space: nowrap; overflow: hidden; font-size: 14px; color: #1f2328; }
a.url.hover { background: #e8eaed; border-color: #e8eaed; }
a.url.focused { background: #ffffff; border: 2px solid #1a73e8; padding-left: 12px; }
.chip { flex: none; height: 22px; line-height: 22px; padding: 0 8px; margin-right: 4px; border-radius: 11px;
        font-size: 12px; color: #4a5058; }
.chip.insecure { background: #fce8e6; color: #b3261e; }
.chip.file { background: #e8eaed; }
.host { color: #1f2328; }
.rest { color: #6b7280; }
.placeholder { color: #80868b; }
.caret { color: #1a73e8; }
.badge { flex: none; height: 24px; line-height: 24px; padding: 0 10px; border-radius: 12px; font-size: 12px; }
.badge.loading { background: #e8f0fe; color: #1a56c4; }
.badge.lost { background: #fce8e6; color: #b3261e; }
";

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

/// `href` のリンクに付けるクラス（マウスが乗っていれば `hover` を足す）。
fn class_for(state: &ChromeState<'_>, base: &str, href: &str) -> String {
    if state.hover == Some(href) {
        format!("{base} hover")
    } else {
        base.to_owned()
    }
}

fn tabs_html(state: &ChromeState<'_>, html: &mut String) {
    html.push_str("<div class=tabs>");
    for (i, tab) in state.tabs.iter().enumerate() {
        let href = format!("kumo:tab/{i}");
        let base = if i == state.active {
            "tab active"
        } else {
            "tab"
        };
        let class = class_for(state, base, &href);
        let close_href = format!("kumo:close/{i}");
        let close = class_for(state, "close", &close_href);
        let dot = if tab.loading {
            "<span class=dot>●</span>"
        } else {
            ""
        };
        let title = escape(&truncate(&tab.title, TAB_TITLE_CHARS));
        html.push_str(&format!(
            "<div class='{class}'><a class=title href='{href}'>{dot}{title}</a>\
             <a class='{close}' href='{close_href}'>✕</a></div>"
        ));
    }
    let new = class_for(state, "new", "kumo:new");
    html.push_str(&format!("<a class='{new}' href='kumo:new'>＋</a></div>"));
}

/// URL 欄の中身。編集中は入力中の文字列、それ以外は安全性の印とドメインを強調した表記。
fn url_html(state: &ChromeState<'_>) -> String {
    if state.url_focused {
        if state.url_text.is_empty() {
            return "<span class=caret>|</span><span class=placeholder>検索語または URL を入力</span>".to_owned();
        }
        return format!("{}<span class=caret>|</span>", escape(state.url_text));
    }
    let parts = format_url(state.url_text);
    let chip = parts
        .security
        .chip()
        .map(|(class, label)| format!("<span class='chip {class}'>{label}</span>"))
        .unwrap_or_else(|| "<span class=chip></span>".to_owned());
    format!(
        "{chip}<span class=host>{}</span><span class=rest>{}</span>",
        escape(&parts.host),
        escape(&parts.rest)
    )
}

/// UI の HTML を生成する。
pub fn chrome_html(state: &ChromeState<'_>) -> String {
    let mut html = format!("<html><head><style>{CSS}</style></head><body>");
    tabs_html(state, &mut html);
    let nav_class = if state.loading { "nav loading" } else { "nav" };
    html.push_str(&format!("<div class='{nav_class}'>"));
    let button = |enabled: bool, href: &str, label: &str, title: &str| {
        let class = if enabled {
            class_for(state, "btn", href)
        } else {
            "btn off".to_owned()
        };
        format!("<a class='{class}' href='{href}' title='{title}'>{label}</a>")
    };
    html.push_str(&button(state.can_go_back, "kumo:back", "←", "戻る"));
    html.push_str(&button(state.can_go_forward, "kumo:forward", "→", "進む"));
    html.push_str(&button(true, "kumo:reload", "↻", "再読み込み"));
    let url_class = if state.url_focused {
        "url focused".to_owned()
    } else {
        class_for(state, "url", "kumo:url")
    };
    html.push_str(&format!(
        "<a class='{url_class}' href='kumo:url'>{}</a>",
        url_html(state)
    ));
    if state.content_lost {
        html.push_str("<span class='badge lost'>⚠ Content プロセスが終了しました</span>");
    } else if state.loading {
        html.push_str("<span class='badge loading'>読み込み中…</span>");
    }
    html.push_str("</div></body></html>");
    html
}

/// URL 欄の入力を URL にする。スキームがなければ https を補い、ローカルファイルも受け付ける。
pub fn parse_input(input: &str) -> Option<Url> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Ok(url) = Url::parse(input)
        && matches!(url.scheme(), "http" | "https" | "file" | "about")
    {
        return Some(url);
    }
    // 実在するファイル（相対パスを含む）はドメイン名より優先する
    let path = std::path::Path::new(input);
    if path.is_absolute() || path.exists() {
        // 無いファイルも開いて、エラーページで知らせる
        let full = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        return Url::from_file_path(full).ok();
    }
    if !input.contains(' ') && (input.contains('.') || input.starts_with("localhost")) {
        return Url::parse(&format!("https://{input}")).ok();
    }
    // 検索エンジンは持たないので、Wikipedia の検索に回す
    let mut url = Url::parse("https://en.wikipedia.org/w/index.php").ok()?;
    url.query_pairs_mut().append_pair("search", input);
    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state<'a>(url_text: &'a str, hover: Option<&'a str>) -> ChromeState<'a> {
        ChromeState {
            tabs: vec![
                TabLabel {
                    title: "<script>".into(),
                    loading: true,
                },
                TabLabel {
                    title: "x".repeat(40),
                    loading: false,
                },
            ],
            active: 1,
            url_text,
            url_focused: false,
            can_go_back: false,
            can_go_forward: true,
            loading: false,
            content_lost: false,
            hover,
        }
    }

    #[test]
    fn parses_actions() {
        assert_eq!(
            ChromeAction::from_href("kumo:tab/3"),
            Some(ChromeAction::ActivateTab(3))
        );
        assert_eq!(
            ChromeAction::from_href("kumo:close/0"),
            Some(ChromeAction::CloseTab(0))
        );
        assert_eq!(
            ChromeAction::from_href("kumo:new"),
            Some(ChromeAction::NewTab)
        );
        assert_eq!(
            ChromeAction::from_href("kumo:back"),
            Some(ChromeAction::Back)
        );
        assert_eq!(ChromeAction::from_href("kumo:tab/x"), None);
        assert_eq!(ChromeAction::from_href("https://x/"), None);
    }

    #[test]
    fn parses_url_bar_input() {
        assert_eq!(
            parse_input("https://a.b/c").unwrap().as_str(),
            "https://a.b/c"
        );
        assert_eq!(
            parse_input("example.com").unwrap().as_str(),
            "https://example.com/"
        );
        let search = parse_input("rust language").unwrap();
        assert!(search.as_str().contains("search=rust+language"));
        assert!(parse_input("   ").is_none());
        // 実在する相対パスはファイルとして開く（ドメイン名と解釈しない）
        let here = parse_input("Cargo.toml").unwrap();
        assert_eq!(here.scheme(), "file");
        let missing = if cfg!(windows) {
            "C:/no/such/file.html"
        } else {
            "/no/such/file.html"
        };
        assert_eq!(
            parse_input(missing).unwrap().scheme(),
            "file",
            "無いファイルもエラーページで知らせる"
        );
    }

    #[test]
    fn html_escapes_titles_and_marks_state() {
        let html = chrome_html(&state("https://a.example/?q=<x>", None));
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("tab active"));
        assert!(html.contains("…"));
        assert!(html.contains("class='btn off' href='kumo:back'"));
        assert!(
            html.contains("<span class=dot>●</span>&lt;script"),
            "読み込み中のタブに印"
        );
        assert!(
            html.contains("<span class=host>a.example</span>"),
            "ドメインを分けて強調"
        );
        assert!(html.contains("chip secure"));
        assert!(
            !html.contains(" hover'"),
            "マウスが乗っていなければ hover クラスはない"
        );
    }

    #[test]
    fn hover_editing_loading_and_lost_states() {
        let hovered = chrome_html(&state("http://plain.example/", Some("kumo:close/0")));
        assert!(hovered.contains("class='close hover' href='kumo:close/0'"));
        assert!(hovered.contains("保護されていない通信"));
        let mut editing = state("", Some("kumo:back"));
        editing.url_focused = true;
        editing.loading = true;
        let html = chrome_html(&editing);
        assert!(html.contains("url focused"));
        assert!(
            html.contains("検索語または URL を入力"),
            "空の URL 欄に案内"
        );
        assert!(html.contains("nav loading") && html.contains("読み込み中"));
        assert!(
            html.contains("class='btn off' href='kumo:back'"),
            "無効なボタンは hover しない"
        );
        editing.content_lost = true;
        assert!(chrome_html(&editing).contains("Content プロセスが終了しました"));
    }
}
