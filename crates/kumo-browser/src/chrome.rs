//! ブラウザ UI（タブ列・ナビゲーション・URL 欄）。設計書どおり第2の UI ツールキットを使わず、
//! 自エンジンで描く HTML/CSS として生成する。操作は `kumo:` スキームのリンクで表す。
//! `:hover` は使えないので、マウスが乗っているリンクには Browser が `hover` クラスを付けて作り直す。

use url::Url;

use kumo_content::i18n::ui::Ui;
use kumo_content::{Lang, escape};

use crate::chrome_css::CSS;
use crate::url_display::format_url;

/// UI の高さ（px）。コンテンツはこの下に描く。
pub const CHROME_HEIGHT: f32 = 76.0;
/// タブ見出しの最大文字数。
const TAB_TITLE_CHARS: usize = 28;
/// パスワードの確認の文の最大文字数。
const ASK_CHARS: usize = 60;

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
    ToggleBookmark,
    ShowBookmarks,
    ShowHistory,
    ShowDownloads,
    Home,
    /// パスワードの保存の確認：保存する・今回は保存しない・このサイトでは保存しない
    SaveLogin,
    DismissLogin,
    NeverSaveLogin,
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
                "bookmark" => Self::ToggleBookmark,
                "bookmarks" => Self::ShowBookmarks,
                "history" => Self::ShowHistory,
                "downloads" => Self::ShowDownloads,
                "home" => Self::Home,
                "pwsave" => Self::SaveLogin,
                "pwskip" => Self::DismissLogin,
                "pwnever" => Self::NeverSaveLogin,
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
    /// 凍結または破棄されている（見出しを薄くして印を付ける）
    pub dormant: bool,
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
    /// 表示中のページがブックマーク済み
    pub bookmarked: bool,
    /// Content プロセスが終了して作り直せなかった
    pub content_lost: bool,
    /// マウスが乗っている UI のリンク
    pub hover: Option<&'a str>,
    /// パスワードを保存するかの確認（あれば URL 欄の代わりに出す）
    pub login_prompt: Option<&'a str>,
    /// 表示言語
    pub lang: Lang,
}

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
        let base = match (i == state.active, tab.dormant) {
            (true, _) => "tab active",
            (false, true) => "tab dormant",
            (false, false) => "tab",
        };
        let class = class_for(state, base, &href);
        let close_href = format!("kumo:close/{i}");
        let close = class_for(state, "close", &close_href);
        let dot = if tab.loading {
            "<span class=dot>●</span>"
        } else if tab.dormant {
            "<span class=dot>○</span>"
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
            return format!(
                "<span class=caret>|</span><span class=placeholder>{}</span>",
                Ui::UrlPlaceholder.t(state.lang)
            );
        }
        return format!("{}<span class=caret>|</span>", escape(state.url_text));
    }
    let parts = format_url(state.url_text);
    let chip = parts
        .security
        .chip(state.lang)
        .map(|(class, label)| format!("<span class='chip {class}'>{label}</span>"))
        .unwrap_or_else(|| "<span class=chip></span>".to_owned());
    format!(
        "{chip}<span class=host>{}</span><span class=rest>{}</span>",
        escape(&parts.host),
        escape(&parts.rest)
    )
}

/// パスワードの保存の確認（URL 欄の位置に出す）。
fn login_prompt_html(state: &ChromeState<'_>, prompt: &str) -> String {
    let button = |class: &str, href: &str, label: &str| {
        format!(
            "<a class='{}' href='{href}'>{label}</a>",
            class_for(state, class, href)
        )
    };
    format!(
        "<span class=ask>🔑 {}</span>{}{}{}",
        escape(&truncate(prompt, ASK_CHARS)),
        button("askbtn main", "kumo:pwsave", Ui::SaveLogin.t(state.lang)),
        button("askbtn", "kumo:pwskip", Ui::SkipLogin.t(state.lang)),
        button("askbtn", "kumo:pwnever", Ui::NeverSaveLogin.t(state.lang)),
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
    html.push_str(&button(
        state.can_go_back,
        "kumo:back",
        "←",
        Ui::Back.t(state.lang),
    ));
    html.push_str(&button(
        state.can_go_forward,
        "kumo:forward",
        "→",
        Ui::Forward.t(state.lang),
    ));
    html.push_str(&button(true, "kumo:reload", "↻", Ui::Reload.t(state.lang)));
    html.push_str(&button(true, "kumo:home", "⌂", Ui::Home.t(state.lang)));
    if let Some(prompt) = state.login_prompt {
        html.push_str(&login_prompt_html(state, prompt));
        html.push_str("</div></body></html>");
        return html;
    }
    let url_class = if state.url_focused {
        "url focused".to_owned()
    } else {
        class_for(state, "url", "kumo:url")
    };
    html.push_str(&format!(
        "<a class='{url_class}' href='kumo:url'>{}</a>",
        url_html(state)
    ));
    let star_class = class_for(
        state,
        if state.bookmarked { "star on" } else { "star" },
        "kumo:bookmark",
    );
    let (star, star_title) = if state.bookmarked {
        ("★", Ui::BookmarkRemove.t(state.lang))
    } else {
        ("☆", Ui::BookmarkAdd.t(state.lang))
    };
    html.push_str(&format!(
        "<a class='{star_class}' href='kumo:bookmark' title='{star_title}'>{star}</a>"
    ));
    html.push_str(&button(
        true,
        "kumo:bookmarks",
        "☰",
        Ui::BookmarkList.t(state.lang),
    ));
    if state.content_lost {
        html.push_str(&format!(
            "<span class='badge lost'>{}</span>",
            Ui::ContentLost.t(state.lang)
        ));
    } else if state.loading {
        html.push_str(&format!(
            "<span class='badge loading'>{}</span>",
            Ui::Loading.t(state.lang)
        ));
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
