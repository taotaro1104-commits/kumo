//! 閲覧履歴ページとダウンローダー（取得元は Network の代わりの偽物で、保存の依頼と経過だけを模す）。
mod common;

use std::path::Path;
use std::sync::{Arc, Mutex};

use common::{Site, settle, title};
use kumo_browser::{Browser, ChromeAction, DOWNLOADS_URL, HISTORY_URL, HistoryDb};
use kumo_net::process::{FetchResult, Fetcher};
use kumo_net::protocol::{DownloadEvent, DownloadState, FetchError, NetworkStats, ResourceKind};
use kumo_net::service::{NetworkConfig, NetworkService};

/// `.zip` への移動は「保存するファイル」として返し、保存の依頼を記録する。それ以外は本物に任せる。
struct FakeNetwork {
    inner: NetworkService,
    started: Mutex<Vec<(String, String, Option<String>)>>,
    cancelled: Mutex<Vec<u64>>,
    events: Mutex<Vec<DownloadEvent>>,
}

impl FakeNetwork {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: NetworkService::new(&NetworkConfig::default()),
            started: Mutex::default(),
            cancelled: Mutex::default(),
            events: Mutex::default(),
        })
    }

    fn push(&self, id: u64, name: &str, received: u64, total: Option<u64>, state: DownloadState) {
        self.events.lock().unwrap().push(DownloadEvent {
            id,
            name: name.to_owned(),
            received,
            total,
            state,
        });
    }
}

impl Fetcher for FakeNetwork {
    fn fetch(&self, url: &str, kind: ResourceKind, source: Option<&str>) -> FetchResult {
        if kind == ResourceKind::Document && url.ends_with(".zip") {
            return Err(FetchError::Download);
        }
        Fetcher::fetch(&self.inner, url, kind, source)
    }

    fn post(&self, url: &str, ct: &str, body: Vec<u8>, source: Option<&str>) -> FetchResult {
        Fetcher::post(&self.inner, url, ct, body, source)
    }

    fn stats(&self) -> Option<NetworkStats> {
        None
    }

    fn start_download(&self, url: &str, dir: &Path, source: Option<&str>) -> Option<u64> {
        let mut started = self.started.lock().unwrap();
        started.push((
            url.to_owned(),
            dir.to_string_lossy().into_owned(),
            source.map(str::to_owned),
        ));
        Some(started.len() as u64)
    }

    fn cancel_download(&self, id: u64) {
        self.cancelled.lock().unwrap().push(id);
    }

    fn take_download_events(&self) -> Vec<DownloadEvent> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }
}

fn rig(history: Option<HistoryDb>) -> (Browser, Arc<FakeNetwork>) {
    let network = FakeNetwork::new();
    let mut b = Browser::in_process(network.clone(), history, (800.0, 600.0));
    b.set_downloader(network.clone());
    b.set_download_dir("C:/dl".into());
    (b, network)
}

fn enc(url: &str) -> String {
    url::form_urlencoded::byte_serialize(url.as_bytes()).collect()
}

// ---- 閲覧履歴 ----

#[test]
fn history_page_lists_removes_and_clears_only_from_itself() {
    let site = Site::new("history-page");
    let path = site.dir.join("history.redb");
    let (a, b_url) = (site.url("a.html"), site.url("b.html"));
    let (mut b, _net) = rig(Some(HistoryDb::open(&path).unwrap()));
    b.open_tab(Some(a.clone()), true);
    settle(&mut b);
    b.open_tab(Some(b_url.clone()), true);
    settle(&mut b);

    // ほかのページからのリンクでは消せない
    b.follow_link(&format!("kumo:unhistory?u={}", enc(a.as_str())), false);
    b.follow_link("kumo:clearhistory", false);
    settle(&mut b);

    b.show_history();
    settle(&mut b);
    assert_eq!(title(&b), "閲覧履歴");
    assert_eq!(b.active_tab().unwrap().url().unwrap().as_str(), HISTORY_URL);
    b.follow_link(&format!("kumo:unhistory?u={}", enc(a.as_str())), false);
    settle(&mut b);
    drop(b);
    let db = HistoryDb::open(&path).unwrap();
    assert!(db.get(a.as_str()).is_none(), "1件消えた");
    assert!(db.get(b_url.as_str()).is_some(), "ほかは残る");
    drop(db);

    let (mut b, _net) = rig(Some(HistoryDb::open(&path).unwrap()));
    b.show_history();
    settle(&mut b);
    b.follow_link("kumo:clearhistory", false);
    settle(&mut b);
    drop(b);
    let db = HistoryDb::open(&path).unwrap();
    assert!(db.recent(10).is_empty(), "全部消えた");
}

#[test]
fn clearing_history_keeps_bookmarks() {
    let site = Site::new("history-keep");
    let path = site.dir.join("history.redb");
    let db = HistoryDb::open(&path).unwrap();
    db.record("https://x.example/", "X").unwrap();
    db.add_bookmark("https://keep.example/", "keep").unwrap();
    db.clear_visits().unwrap();
    assert!(db.recent(10).is_empty());
    assert_eq!(db.bookmarks().len(), 1);
}

#[test]
fn recent_history_is_newest_first() {
    let site = Site::new("history-order");
    let db = HistoryDb::open(&site.dir.join("h.redb")).unwrap();
    db.record("https://one.example/", "1").unwrap();
    db.record("https://two.example/", "2").unwrap();
    assert_eq!(db.recent(1).len(), 1);
    db.remove_visit("https://one.example/").unwrap();
    assert_eq!(db.recent(10)[0].url, "https://two.example/");
}

// ---- ダウンローダー ----

#[test]
fn a_link_to_a_file_starts_a_download_and_keeps_the_page() {
    let site = Site::new("download-link");
    let (mut b, net) = rig(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);

    b.follow_link("http://files.test/pack.zip", false);
    settle(&mut b);

    let started = net.started.lock().unwrap().clone();
    assert_eq!(started.len(), 1, "保存の依頼が1つ");
    assert_eq!(started[0].0, "http://files.test/pack.zip");
    assert_eq!(started[0].1, "C:/dl");
    assert_eq!(started[0].2.as_deref(), Some(site.url("a.html").as_str()));
    assert!(b.downloads_active());

    assert_eq!(b.tabs().len(), 2, "一覧は新しいタブで見せる");
    assert_eq!(title(&b), "ダウンロード");
    let first = &b.tabs()[0];
    assert_eq!(
        first.url().unwrap(),
        &site.url("a.html"),
        "元のタブは移動していない"
    );
    assert!(!first.is_loading());
    assert_eq!(first.title(), "Page A");
}

#[test]
fn progress_completion_and_cancel_are_reflected() {
    let site = Site::new("download-progress");
    let (mut b, net) = rig(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    b.follow_link("http://files.test/pack.zip", false);
    settle(&mut b);

    net.push(1, "pack.zip", 512, Some(1024), DownloadState::Running);
    b.poll_downloads();
    let html = b.downloads_html();
    assert!(
        html.contains("pack.zip") && html.contains("(50%)"),
        "{html}"
    );
    assert!(html.contains("kumo:dlcancel?id=1"));

    // 一覧ページ以外からの中止・整理は受け付けない
    b.activate(0);
    b.follow_link("kumo:dlcancel?id=1", false);
    b.follow_link("kumo:dlclear", false);
    assert!(net.cancelled.lock().unwrap().is_empty());

    b.activate(1);
    b.follow_link("kumo:dlcancel?id=1", false);
    settle(&mut b);
    assert_eq!(*net.cancelled.lock().unwrap(), vec![1]);

    net.push(1, "pack.zip", 600, Some(1024), DownloadState::Cancelled);
    b.poll_downloads();
    assert!(!b.downloads_active());
    assert!(b.downloads_html().contains("中止しました"));

    b.follow_link("kumo:dlclear", false);
    settle(&mut b);
    assert!(
        b.downloads_html()
            .contains("ダウンロードしたファイルはありません"),
        "終わった項目が消えた"
    );
}

#[test]
fn finished_download_shows_where_it_was_saved() {
    let site = Site::new("download-done");
    let (mut b, net) = rig(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    b.follow_link("http://files.test/pack.zip", false);
    settle(&mut b);
    net.push(
        1,
        "pack (1).zip",
        1024,
        Some(1024),
        DownloadState::Done("C:/dl/pack (1).zip".into()),
    );
    b.poll_downloads();
    let html = b.downloads_html();
    assert!(html.contains("pack (1).zip") && html.contains("C:/dl/pack (1).zip"));
    assert!(html.contains("完了"));
}

#[test]
fn opening_a_file_in_the_only_tab_leaves_just_the_downloads_page() {
    let (mut b, net) = rig(None);
    b.open_tab(
        Some(url::Url::parse("http://files.test/pack.zip").unwrap()),
        true,
    );
    settle(&mut b);
    assert_eq!(net.started.lock().unwrap().len(), 1);
    assert_eq!(b.tabs().len(), 1, "空のタブは残さない");
    assert_eq!(
        b.active_tab().unwrap().url().unwrap().as_str(),
        DOWNLOADS_URL
    );
    assert!(!b.active_tab().unwrap().is_loading());
}

#[test]
fn a_download_in_a_background_tab_closes_that_tab() {
    let site = Site::new("download-bg");
    let (mut b, net) = rig(None);
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    b.open_tab(
        Some(url::Url::parse("http://files.test/pack.zip").unwrap()),
        false,
    );
    settle(&mut b);
    assert_eq!(net.started.lock().unwrap().len(), 1);
    let urls: Vec<_> = b
        .tabs()
        .iter()
        .filter_map(|t| t.url().map(|u| u.to_string()))
        .collect();
    assert!(!urls.iter().any(|u| u.ends_with("pack.zip")), "{urls:?}");
}

#[test]
fn without_a_downloader_the_item_is_listed_as_failed() {
    let site = Site::new("download-none");
    let network = FakeNetwork::new();
    let mut b = Browser::in_process(network, None, (800.0, 600.0));
    b.open_tab(Some(site.url("a.html")), true);
    settle(&mut b);
    b.follow_link("http://files.test/pack.zip", false);
    settle(&mut b);
    assert!(b.downloads_html().contains("失敗"));
    assert!(!b.downloads_active());
}

#[test]
fn shortcuts_open_the_pages() {
    let (mut b, _net) = rig(None);
    b.open_tab(None, true);
    settle(&mut b);
    b.chrome_action(ChromeAction::ShowDownloads);
    settle(&mut b);
    assert_eq!(title(&b), "ダウンロード");
    b.chrome_action(ChromeAction::ShowHistory);
    settle(&mut b);
    assert_eq!(title(&b), "閲覧履歴");
}
