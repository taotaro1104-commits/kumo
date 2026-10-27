mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{Site, browser, settle, title};
use kumo_browser::{Browser, CHROME_HEIGHT, ChromeAction, HistoryDb};
use kumo_net::service::{NetworkConfig, NetworkService};

#[test]
fn loading_does_not_block_and_finishes_asynchronously() {
    let site = Site::new("async");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    // 依頼しただけで戻る：まだ読み込み中で、UI は読み込み中と表示できる
    assert!(b.active_tab().unwrap().is_loading());
    let chrome_text = {
        let chrome = b.chrome_page();
        chrome.document().text_content(chrome.document().root())
    };
    assert!(chrome_text.contains("読み込み中"), "{chrome_text}");
    // 読み込み中も UI の操作は受け付ける
    b.focus_url_bar();
    b.type_text("typing");
    settle(&mut b);
    assert_eq!(title(&b), "Page A");
    assert!(!b.active_tab().unwrap().is_loading());
    assert_eq!(b.url_text(), "typing", "入力中の URL 欄は上書きしない");
}

#[test]
fn navigation_back_forward_and_link_clicks() {
    let site = Site::new("nav");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    assert_eq!(title(&b), "Page A");
    // 外部 CSS（margin 0、20px）が効いて左上にリンクがある
    b.click_content(5.0, 10.0, false);
    settle(&mut b);
    assert_eq!(title(&b), "Page B");
    b.back();
    settle(&mut b);
    assert_eq!(title(&b), "Page A");
    b.forward();
    settle(&mut b);
    assert_eq!(title(&b), "Page B");
    assert!(b.active_tab().unwrap().session.can_go_back());
}

#[test]
fn background_tabs_and_closing() {
    let site = Site::new("tabs");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    b.click_content(5.0, 10.0, true);
    settle(&mut b);
    assert_eq!(b.tabs().len(), 2);
    assert_eq!(b.active_index(), 0, "Ctrl+クリックは背景で開く");
    assert_eq!(b.tabs()[1].title(), "Page B");
    b.resize(1000.0, 700.0);
    b.chrome_action(ChromeAction::ActivateTab(1));
    let frame = b
        .wait_frame(Duration::from_secs(20))
        .expect("フレームが届く");
    assert_eq!(
        (frame.width, frame.height),
        (1000, (700.0 - CHROME_HEIGHT) as u32)
    );

    b.chrome_action(ChromeAction::CloseTab(1));
    assert_eq!(b.tabs().len(), 1);
    assert_eq!(b.active_index(), 0);
    b.chrome_action(ChromeAction::CloseTab(0));
    settle(&mut b);
    assert_eq!(b.tabs().len(), 1, "最後のタブを閉じたら空白タブ");
    assert_eq!(title(&b), "新しいタブ");
    assert!(b.url_focused());
}

#[test]
fn url_bar_typing_errors_and_history() {
    let site = Site::new("url");
    let history_path = site.dir.join("history.redb");
    let mut b = browser(Some(HistoryDb::open(&history_path).unwrap()));
    b.open_tab(None, true);
    b.type_text(site.url("a.html").as_str());
    b.backspace();
    b.type_text("l\u{8}");
    b.submit_url();
    settle(&mut b);
    assert_eq!(title(&b), "Page A");

    b.focus_url_bar();
    b.type_text(site.url("missing.html").as_str());
    b.submit_url();
    settle(&mut b);
    assert_eq!(title(&b), "読み込めません", "エラーページを表示する");

    drop(b);
    let history = HistoryDb::open(&history_path).unwrap();
    assert_eq!(
        history.get(site.url("a.html").as_str()).unwrap().title,
        "Page A"
    );
}

#[test]
fn chrome_page_exposes_actions_as_links() {
    let site = Site::new("chrome");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    b.open_tab(Some(site.url("b.html")), false);
    settle(&mut b);
    let chrome = b.chrome_page();
    let hrefs: Vec<&str> = chrome.list.links.iter().map(|l| &*l.href).collect();
    for expected in [
        "kumo:tab/0",
        "kumo:tab/1",
        "kumo:close/1",
        "kumo:new",
        "kumo:back",
        "kumo:url",
    ] {
        assert!(hrefs.contains(&expected), "{expected} がない: {hrefs:?}");
    }
    let tab1 = chrome
        .list
        .links
        .iter()
        .find(|l| &*l.href == "kumo:tab/1")
        .unwrap();
    assert!(tab1.rect.bottom() <= CHROME_HEIGHT);
    let href = chrome
        .list
        .link_at(tab1.rect.x + 1.0, tab1.rect.y + 1.0)
        .unwrap()
        .clone();
    b.chrome_action(ChromeAction::from_href(&href).unwrap());
    assert_eq!(title(&b), "Page B");
}

#[test]
fn fragment_links_scroll_without_reloading() {
    let site = Site::new("anchor");
    std::fs::write(
        site.dir.join("long.html"),
        "<html><head><title>Long</title></head><body style='margin:0'>\
         <a href='#sec%E2%80%932'>jump</a><div style='height:2000px'></div>\
         <h2 id='sec–2' style='margin:0'>Section 2</h2><div style='height:2000px'></div></body></html>",
    )
    .unwrap();
    let mut b = browser(None);
    b.open_tab(Some(site.url("long.html")), true);
    settle(&mut b);
    assert_eq!(b.scroll_y(), 0.0);
    b.follow_link("#sec%E2%80%932", false);
    settle(&mut b);
    assert!(
        b.scroll_y() >= 2000.0 - 1.0,
        "飛び先へスクロール: {}",
        b.scroll_y()
    );
    assert_eq!(b.active_tab().unwrap().loads, 1, "再読み込みしない");
    b.back();
    settle(&mut b);
    assert_eq!(b.scroll_y(), 0.0, "戻るとアンカーなしの位置");
    assert_eq!(b.active_tab().unwrap().loads, 1, "戻るでも取得し直さない");
    // 最初から # 付きで開いても飛び先へ
    let mut direct = site.url("long.html");
    direct.set_fragment(Some("sec%E2%80%932"));
    b.reload();
    settle(&mut b);
    b.navigate(direct);
    settle(&mut b);
    assert!(b.scroll_y() >= 2000.0 - 1.0);
}

#[test]
fn forms_type_and_submit_through_content() {
    let site = Site::new("form");
    std::fs::write(
        site.dir.join("form.html"),
        "<html><head><title>Form</title></head><body style='margin:0'>\
         <form action='result.html'><input name=q placeholder='Search'></form></body></html>",
    )
    .unwrap();
    std::fs::write(
        site.dir.join("result.html"),
        "<title>Result</title><p>ok</p>",
    )
    .unwrap();
    let mut b = browser(None);
    b.open_tab(Some(site.url("form.html")), true);
    settle(&mut b);
    b.click_content(5.0, 5.0, false);
    settle(&mut b);
    assert!(b.field_focused());
    b.type_into_field("rust");
    b.backspace_field();
    b.type_into_field("t 1");
    b.submit_field();
    settle(&mut b);
    assert_eq!(title(&b), "Result");
    let url = b.active_tab().unwrap().url().unwrap().clone();
    assert_eq!(url.query(), Some("q=rust+1"));
    assert!(!b.field_focused(), "移動したらフォーカスは外れる");
    b.back();
    settle(&mut b);
    assert_eq!(title(&b), "Form");
}

#[test]
fn content_stats_and_frames() {
    let site = Site::new("stats");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    let stats = b.content_stats(Duration::from_secs(20)).expect("統計");
    assert_eq!(stats.tabs, 1);
    assert!(stats.dom_nodes > 0);
    let frame = b.wait_frame(Duration::from_secs(20)).unwrap();
    assert_eq!(frame.pixels.len(), 800 * (600.0 - CHROME_HEIGHT) as usize);
    assert!(
        b.content_client().process_id().is_none(),
        "スレッドで動いている"
    );
}

#[test]
fn frames_keep_coming_after_discarding_an_unpresented_frame() {
    let site = Site::new("discard");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    b.open_tab(Some(site.url("b.html")), false);
    settle(&mut b);
    // 写す前のフレームを1枚受け取った状態にする
    b.scroll_by(1.0);
    b.content_client()
        .send(kumo_content::protocol::ToContent::Show {
            tab: b.active_tab().unwrap().id,
        });
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !b.frame_pending() {
        assert!(std::time::Instant::now() < deadline, "フレームが届く");
        b.pump();
        std::thread::yield_now();
    }
    // 写さずにタブを切り替え、さらにリサイズしても、新しいフレームは届き続ける
    b.chrome_action(ChromeAction::ActivateTab(1));
    assert!(
        b.wait_frame(Duration::from_secs(5)).is_some(),
        "切替後のフレーム"
    );
    b.resize(900.0, 600.0);
    assert!(
        b.wait_frame(Duration::from_secs(5)).is_some(),
        "リサイズ後のフレーム"
    );
}

/// すぐに終了する「Content プロセス」（落ちた Content の代わり）。
fn dying_content() -> kumo_content::ContentClient {
    let mut command = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "exit"]);
        c
    } else {
        std::process::Command::new("true")
    };
    command.stdin(std::process::Stdio::null());
    kumo_content::ContentClient::spawn_process(command).unwrap()
}

#[test]
fn a_crashed_content_process_is_restarted_and_tabs_reload() {
    let site = Site::new("restart");
    let fetcher = Arc::new(NetworkService::new(&NetworkConfig::default()));
    let mut b = Browser::new(dying_content(), None, (800.0, 600.0));
    let restart_fetcher = fetcher.clone();
    b.set_content_restart(Box::new(move || {
        Ok(kumo_content::ContentClient::spawn_thread(
            restart_fetcher.clone(),
        ))
    }));
    b.open_tab(Some(site.url("a.html")), true);
    b.open_tab(Some(site.url("b.html")), false);
    settle(&mut b);
    assert_eq!(title(&b), "Page A", "作り直した Content で読み込み直す");
    assert_eq!(b.tabs()[1].title(), "Page B");
    assert!(b.wait_frame(Duration::from_secs(5)).is_some());
}

#[test]
fn without_restart_a_crash_stops_loading_and_says_so() {
    let site = Site::new("crash");
    let mut b = Browser::new(dying_content(), None, (800.0, 600.0));
    b.open_tab(Some(site.url("a.html")), true);
    assert!(!b.settle(Duration::from_secs(5)));
    assert!(
        !b.active_tab().unwrap().is_loading(),
        "読み込み中のまま残さない"
    );
    let chrome = b.chrome_page();
    let text = chrome.document().text_content(chrome.document().root());
    assert!(text.contains("Content プロセスが終了"), "{text}");
}
