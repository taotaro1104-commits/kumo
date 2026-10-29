//! `chrome.rs` のテスト。

use crate::chrome::*;
use kumo_content::Lang;

fn state<'a>(url_text: &'a str, hover: Option<&'a str>) -> ChromeState<'a> {
    ChromeState {
        tabs: vec![
            TabLabel {
                title: "<script>".into(),
                loading: true,
                dormant: false,
            },
            TabLabel {
                title: "x".repeat(40),
                loading: false,
                dormant: false,
            },
        ],
        active: 1,
        url_text,
        preedit: "",
        url_focused: false,
        can_go_back: false,
        can_go_forward: true,
        loading: false,
        bookmarked: true,
        zoom: 100,
        content_lost: false,
        hover,
        login_prompt: None,
        lang: Lang::Ja,
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
    assert_eq!(
        ChromeAction::from_href("kumo:bookmark"),
        Some(ChromeAction::ToggleBookmark)
    );
    assert_eq!(
        ChromeAction::from_href("kumo:bookmarks"),
        Some(ChromeAction::ShowBookmarks)
    );
    assert_eq!(
        ChromeAction::from_href("kumo:home"),
        Some(ChromeAction::Home)
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
        html.contains("star on") && html.contains("★"),
        "ブックマーク済みの星"
    );
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

#[test]
fn login_prompt_replaces_the_url_bar_with_three_choices() {
    let mut st = state("https://a.example/", Some("kumo:pwsave"));
    st.login_prompt = Some("<b> のパスワードを保存しますか？");
    let html = chrome_html(&st);
    assert!(html.contains("&lt;b&gt;") && !html.contains("<b>"));
    assert!(html.contains("class='askbtn main hover' href='kumo:pwsave'"));
    assert!(html.contains("kumo:pwskip") && html.contains("kumo:pwnever"));
    assert!(!html.contains("kumo:url"), "確認中は URL 欄を出さない");
    assert_eq!(
        ChromeAction::from_href("kumo:pwnever"),
        Some(ChromeAction::NeverSaveLogin)
    );
}

#[test]
fn preedit_text_is_shown_underlined_before_the_caret() {
    let mut editing = state("東京", None);
    editing.url_focused = true;
    editing.preedit = "てんき<";
    let html = chrome_html(&editing);
    assert!(
        html.contains("東京<span class=pre>てんき&lt;</span><span class=caret>"),
        "{html}"
    );
    // 変換中の文字だけでも案内は消える
    let mut only = state("", None);
    only.url_focused = true;
    only.preedit = "あ";
    assert!(!chrome_html(&only).contains("検索語または URL を入力"));
}
