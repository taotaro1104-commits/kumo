//! MemoryGovernor の実行：背景のタブの凍結・破棄と、表示したときの復帰。

mod common;

use std::time::{Duration, Instant};

use common::{Site, browser, settle, title};
use kumo_browser::{Browser, TabState};
use kumo_mem::Pressure;

const MB: u64 = 1024 * 1024;

/// タブ 0（表示中）と背景のタブ 1 を開く。
fn two_tabs(name: &str) -> (Site, Browser) {
    let site = Site::new(name);
    let mut b = browser(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    b.click_content(5.0, 10.0, true); // Ctrl+クリックで背景に page B
    settle(&mut b);
    assert_eq!(b.tabs().len(), 2);
    (site, b)
}

fn later(secs: u64) -> Instant {
    Instant::now() + Duration::from_secs(secs)
}

#[test]
fn an_idle_background_tab_is_frozen_and_wakes_when_shown() {
    let (_site, mut b) = two_tabs("gov-freeze");
    assert!(!b.govern(50 * MB, Pressure::None, later(5)).chrome);
    assert_eq!(b.tabs()[1].state, TabState::Live, "まだ新しい");
    assert!(b.govern(50 * MB, Pressure::None, later(120)).chrome);
    assert_eq!(b.tabs()[1].state, TabState::Frozen);
    assert_eq!(b.tabs()[0].state, TabState::Live, "表示中は対象外");
    b.activate(1);
    assert_eq!(b.tabs()[1].state, TabState::Live);
    assert!(!b.tabs()[1].is_loading(), "凍結は読み込み直さない");
    b.wait_frame(Duration::from_secs(20)).expect("再描画される");
    assert_eq!(title(&b), "Page B");
}

#[test]
fn critical_pressure_discards_and_showing_reloads() {
    let (_site, mut b) = two_tabs("gov-discard");
    b.govern(50 * MB, Pressure::Critical, later(1));
    assert_eq!(b.tabs()[1].state, TabState::Discarded);
    assert_eq!(b.tabs()[1].title(), "Page B", "題名は覚えている");
    b.activate(1);
    assert!(b.tabs()[1].is_loading(), "読み込み直す");
    assert_eq!(b.tabs()[1].state, TabState::Live);
    settle(&mut b);
    assert_eq!(title(&b), "Page B");
}

#[test]
fn going_over_budget_freezes_background_tabs_at_once() {
    let (_site, mut b) = two_tabs("gov-budget");
    let over = b.memory_policy().budget + MB;
    b.govern(over, Pressure::None, later(1));
    assert_eq!(b.tabs()[1].state, TabState::Frozen);
}

#[test]
fn a_loading_tab_is_not_discarded() {
    let (site, mut b) = two_tabs("gov-loading");
    b.activate(1);
    b.navigate(site.url("a.html")); // 読み込みを依頼しただけ
    b.activate(0);
    assert!(b.tabs()[1].is_loading());
    b.govern(50 * MB, Pressure::Critical, later(1));
    assert_eq!(b.tabs()[1].state, TabState::Live);
}

#[test]
fn about_memory_opens_as_an_internal_page() {
    let (_site, mut b) = two_tabs("gov-about");
    b.govern(50 * MB, Pressure::None, later(120));
    b.open_tab(
        Some(url::Url::parse(kumo_browser::ABOUT_MEMORY_URL).unwrap()),
        true,
    );
    settle(&mut b);
    assert_eq!(title(&b), "メモリ");
}
