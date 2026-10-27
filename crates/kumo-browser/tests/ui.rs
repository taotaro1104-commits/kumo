//! UI の見た目と反応：マウスの hover、新しいタブ、エラーページのボタン、読み込み中の表示。
mod common;

use common::{Site, browser, settle, title};
use kumo_browser::{CHROME_HEIGHT, HistoryDb};

#[test]
fn hovering_sets_the_pointer_and_shows_the_link_target() {
    use kumo_browser::Pointer;
    let site = Site::new("hover");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    assert_eq!(b.pointer(), Pointer::Default);
    b.hover_content(5.0, 10.0);
    settle(&mut b);
    assert_eq!(b.pointer(), Pointer::Link, "リンクの上は指の形");
    let bubble = b.status_bubble().expect("リンク先の吹き出し");
    assert!(bubble.width > 20 && bubble.width < 800 && bubble.height > 10);
    let text = bubble
        .page
        .document()
        .text_content(bubble.page.document().root());
    assert!(text.contains("b.html"), "{text}");
    b.hover_content(700.0, 400.0);
    settle(&mut b);
    assert_eq!(b.pointer(), Pointer::Default);
    assert!(b.status_bubble().is_none());

    // UI のリンクの上：hover クラスで作り直し、同じ位置では作り直さない
    let plus = {
        let list = &b.chrome_page().list;
        let link = list.links.iter().find(|l| &*l.href == "kumo:new").unwrap();
        (link.rect.x + 2.0, link.rect.y + 2.0)
    };
    assert!(b.hover_chrome(plus.0, plus.1).chrome);
    assert!(!b.hover_chrome(plus.0 + 1.0, plus.1).chrome);
    assert_eq!(b.pointer(), Pointer::Link);
    assert!(
        b.leave_window().chrome,
        "ウィンドウの外へ出たら hover を外す"
    );
    assert_eq!(b.pointer(), Pointer::Default);
}

#[test]
fn new_tab_page_lists_frequent_sites() {
    let site = Site::new("newtab");
    let history_path = site.dir.join("history.redb");
    let mut b = browser(Some(HistoryDb::open(&history_path).unwrap()));
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    b.open_tab(None, true);
    settle(&mut b);
    assert_eq!(title(&b), "新しいタブ");
    assert_eq!(b.url_text(), "", "新しいタブの URL は表示しない");
    // よく見るページのカードをクリックすると開く
    b.click_content(400.0, 300.0, false);
    let html = kumo_browser::new_tab_html(
        &HistoryDb::open(&site.dir.join("h2.redb"))
            .unwrap()
            .search("", 8),
    );
    assert!(html.contains("まだ履歴がありません"), "履歴がなければ案内");
    drop(b);
    let history = HistoryDb::open(&history_path).unwrap();
    let html = kumo_browser::new_tab_html(&history.search("", 8));
    assert!(html.contains("Page A") && html.contains(&site.url("a.html").to_string()));
}

#[test]
fn error_page_buttons_reload_and_go_back() {
    let site = Site::new("errbtn");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    b.navigate(site.url("later.html"));
    settle(&mut b);
    assert_eq!(title(&b), "読み込めません");
    std::fs::write(
        site.dir.join("later.html"),
        "<title>Later</title><p>now here</p>",
    )
    .unwrap();
    b.follow_link("kumo:reload", false);
    settle(&mut b);
    assert_eq!(title(&b), "Later", "エラーページの「再読み込み」");
    b.follow_link("kumo:back", false);
    settle(&mut b);
    assert_eq!(title(&b), "Page A", "エラーページの「戻る」");
    b.follow_link("kumo:close/0", false);
    assert_eq!(
        b.tabs().len(),
        1,
        "ページからは再読み込みと戻る以外の UI 操作はできない"
    );
}

#[test]
fn loading_and_hover_states_fit_in_the_chrome() {
    let site = Site::new("states");
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    b.open_tab(Some(site.url("b.html")), false);
    // 読み込み中のまま UI を作り、マウスを「戻る」以外のボタンに乗せる
    let reload = {
        let list = &b.chrome_page().list;
        let link = list
            .links
            .iter()
            .find(|l| &*l.href == "kumo:reload")
            .unwrap();
        (link.rect.x + 3.0, link.rect.y + 3.0)
    };
    b.hover_chrome(reload.0, reload.1);
    let chrome = b.chrome_page();
    let text = chrome.document().text_content(chrome.document().root());
    assert!(text.contains("読み込み中") && text.contains("●"));
    for link in &chrome.list.links {
        assert!(
            link.rect.bottom() <= CHROME_HEIGHT + 0.5,
            "{} が UI からはみ出す",
            link.href
        );
    }
    if let Some(path) = std::env::var_os("KUMO_CHROME_PNG") {
        let mut raster = kumo_paint::CpuRasterizer::default();
        let region = kumo_paint::RectF::new(0.0, 0.0, 800.0, CHROME_HEIGHT);
        let pm = kumo_paint::render_region(
            &chrome.list,
            &mut raster,
            &mut kumo_paint::NoImageProvider,
            region,
        )
        .unwrap();
        pm.save_png(path).unwrap();
    }
    settle(&mut b);
}
