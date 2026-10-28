//! タブの凍結（JS・描画データを捨てる）と破棄（ページごと捨てる）。

mod common;

use common::{Site, engine, pump};
use kumo_content::ContentEngine;
use kumo_content::protocol::ToContent;

const PAGE: &str = "<html><head><title>T</title></head><body><p id=out>0</p>\
    <script>setInterval(function(){ document.getElementById('out').textContent = 'tick'; }, 100);\
    </script></body></html>";

fn open(site: &Site) -> ContentEngine {
    std::fs::write(site.dir.join("p.html"), PAGE).unwrap();
    let mut e = engine();
    e.handle(ToContent::Viewport {
        width: 400,
        height: 300,
    });
    e.handle(ToContent::Show { tab: 1 });
    e.handle(ToContent::Load {
        tab: 1,
        nav: 1,
        url: site.url("p.html").to_string(),
        post_body: None,
    });
    pump(&mut e, true);
    e
}

#[test]
fn freezing_drops_script_and_display_list_but_keeps_the_document() {
    let site = Site::new("freeze-drop");
    let mut e = open(&site);
    e.handle(ToContent::Show { tab: 2 }); // 別のタブが表示中
    let before = e.stats();
    assert_eq!(before.js_tabs, 1);
    e.handle(ToContent::Freeze { tab: 1 });
    let after = e.stats();
    assert_eq!(after.js_tabs, 0, "スクリプトを止める");
    assert_eq!(after.display_items, 0, "描画命令を捨てる");
    assert_eq!(after.frozen_tabs, 1);
    assert_eq!(after.dom_nodes, before.dom_nodes, "DOM は残す");
    assert!(e.page(1).is_some_and(|p| p.is_frozen()));
}

#[test]
fn showing_a_frozen_tab_lays_it_out_again_without_fetching() {
    let site = Site::new("freeze-thaw");
    let mut e = open(&site);
    e.handle(ToContent::Show { tab: 2 });
    e.handle(ToContent::Freeze { tab: 1 });
    e.take_jobs(); // 回収の仕事は捨てる
    e.handle(ToContent::Show { tab: 1 });
    pump(&mut e, true);
    let stats = e.stats();
    assert_eq!(stats.frozen_tabs, 0);
    assert!(stats.display_items > 0, "表示できる");
    assert!(e.take_jobs().is_empty(), "取得し直さない");
}

#[test]
fn the_active_tab_is_never_frozen_or_discarded() {
    let site = Site::new("freeze-active");
    let mut e = open(&site);
    e.handle(ToContent::Freeze { tab: 1 });
    e.handle(ToContent::Discard { tab: 1 });
    assert!(e.page(1).is_some_and(|p| !p.is_frozen()));
}

#[test]
fn discarding_drops_the_page() {
    let site = Site::new("discard");
    let mut e = open(&site);
    e.handle(ToContent::Show { tab: 2 });
    let before = e.stats().dom_nodes;
    assert!(before > 0);
    e.handle(ToContent::Discard { tab: 1 });
    assert!(e.page(1).is_none());
    assert_eq!(e.stats().dom_nodes, 0);
}

#[test]
fn freezing_an_unknown_or_empty_tab_is_harmless() {
    let mut e = engine();
    e.handle(ToContent::Freeze { tab: 9 });
    e.handle(ToContent::Discard { tab: 9 });
    assert_eq!(e.stats().tabs, 0);
}
