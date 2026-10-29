//! ダウンロード：Network プロセスへの依頼、経過の取り込み、一覧ページの操作。
//! 保存は Network プロセスがファイルへ直接書く（Browser のメモリには溜めない）。

use std::path::PathBuf;
use std::sync::Arc;

use kumo_content::i18n::library::Lib;
use kumo_net::file_name;
use kumo_net::process::Fetcher;
use kumo_net::protocol::{DownloadEvent, DownloadState};
use url::Url;

use crate::browser::{Browser, Redraw};
use crate::downloads_page::{CANCEL_PREFIX, CLEAR_FINISHED, DOWNLOADS_URL};

/// 一覧に残す件数の上限（古い終わった項目から捨てる）。
const KEEP_ITEMS: usize = 100;

/// 一覧の1件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DownloadItem {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub received: u64,
    pub total: Option<u64>,
    pub state: DownloadState,
}

impl DownloadItem {
    pub fn is_running(&self) -> bool {
        self.state == DownloadState::Running
    }
}

/// ダウンロードの状態（Browser が持つ）。
pub(crate) struct Downloads {
    fetcher: Option<Arc<dyn Fetcher>>,
    pub items: Vec<DownloadItem>,
    pub dir: PathBuf,
}

impl Default for Downloads {
    fn default() -> Self {
        Self {
            fetcher: None,
            items: Vec::new(),
            dir: default_dir(),
        }
    }
}

/// 既定の保存先：ユーザーの「ダウンロード」フォルダ。
fn default_dir() -> PathBuf {
    ["USERPROFILE", "HOME"]
        .iter()
        .find_map(std::env::var_os)
        .map_or_else(std::env::temp_dir, |home| {
            PathBuf::from(home).join("Downloads")
        })
}

impl Browser {
    /// ダウンロードに使う取得元（Network プロセス）を設定する。
    pub fn set_downloader(&mut self, fetcher: Arc<dyn Fetcher>) {
        self.downloads.fetcher = Some(fetcher);
    }

    /// 保存先フォルダを変える。
    pub fn set_download_dir(&mut self, dir: PathBuf) {
        self.downloads.dir = dir;
    }

    /// 実行中のダウンロードがあるか（経過を定期的に取り込むかの判断用）。
    pub fn downloads_active(&self) -> bool {
        self.downloads.items.iter().any(DownloadItem::is_running)
    }

    /// `url` の保存を始める。`source` は移動元のページ（コンテンツブロックの判定用）。
    pub fn start_download(&mut self, url: &str, source: Option<&str>) {
        let name = Url::parse(url).map_or_else(|_| url.to_owned(), |u| file_name(None, &u));
        let id = self
            .downloads
            .fetcher
            .as_ref()
            .and_then(|f| f.start_download(url, &self.downloads.dir, source));
        let (id, state) = match id {
            Some(id) => (id, DownloadState::Running),
            // 取得元がない・保存に未対応・接続が切れている：失敗として一覧に出す
            None => (
                self.downloads.items.iter().map(|d| d.id).max().unwrap_or(0) + 1,
                DownloadState::Failed(Lib::DownloadUnsupported.t(self.lang).to_owned()),
            ),
        };
        self.downloads.items.push(DownloadItem {
            id,
            name,
            url: url.to_owned(),
            received: 0,
            total: None,
            state,
        });
        let items = &mut self.downloads.items;
        if items.len() > KEEP_ITEMS
            && let Some(old) = items.iter().position(|d| !d.is_running())
        {
            items.remove(old);
        }
    }

    /// Network から届いた経過を取り込む。一覧ページを表示中なら描き直す。
    pub fn poll_downloads(&mut self) -> Redraw {
        let Some(fetcher) = &self.downloads.fetcher else {
            return Redraw::default();
        };
        let events = fetcher.take_download_events();
        if events.is_empty() {
            return Redraw::default();
        }
        for event in events {
            self.apply_download_event(event);
        }
        if self.active_url_is(DOWNLOADS_URL) {
            self.reload();
            return Redraw::CHROME;
        }
        Redraw::default()
    }

    fn apply_download_event(&mut self, event: DownloadEvent) {
        let Some(item) = self.downloads.items.iter_mut().find(|d| d.id == event.id) else {
            return;
        };
        if !event.name.is_empty() {
            item.name = event.name;
        }
        item.received = event.received;
        item.total = event.total;
        item.state = event.state;
    }

    /// ダウンロードページを開く。
    pub fn show_downloads(&mut self) -> Redraw {
        self.show_internal(DOWNLOADS_URL)
    }

    /// 移動先がファイルだった：読み込み中の移動を取り消して保存を始め、一覧を見せる。
    pub(crate) fn download_requested(&mut self, tab: u64, nav: u64, url: &str) -> Redraw {
        let Some(index) = self.tab_index(tab) else {
            return Redraw::default();
        };
        let source = self.tabs[index].shown_url.as_ref().map(Url::to_string);
        if self.tabs[index].loading == Some(nav) {
            self.cancel_navigation(index, url);
        }
        self.start_download(url, source.as_deref());
        // 取り消しでタブが閉じた・空になったときも、一覧は必ず見せる
        if self.active_url_is(DOWNLOADS_URL) {
            self.reload();
            return Redraw::ALL;
        }
        self.show_downloads().merge(Redraw::CHROME)
    }

    /// 読み込み中の移動を取り消す：移動先として足した履歴の項目を戻し、URL 欄を表示中のページに戻す。
    /// 取り消したあとにタブが空になるなら、そのタブは閉じる（唯一のタブなら新しいタブに戻す）。
    fn cancel_navigation(&mut self, index: usize, url: &str) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        tab.loading = None;
        if let Ok(target) = Url::parse(url) {
            tab.session.discard_current(&target);
        }
        let empty = tab.session.current().is_none();
        let shown = tab
            .shown_url
            .as_ref()
            .filter(|u| u.scheme() != "about")
            .map(Url::to_string)
            .unwrap_or_default();
        if index == self.active {
            self.url_text = shown;
            self.url_focused = false;
        }
        if !empty {
            return;
        }
        if self.tabs.len() > 1 {
            self.close_tab(index);
        } else {
            let blank = Url::parse(crate::NEW_TAB_URL).expect("固定の URL");
            self.start_load(index, blank, true, None);
        }
    }

    /// 一覧ページのリンク（中止・終わった項目を消す）。そのページを表示しているときだけ受け付け、扱ったら `true`。
    pub(crate) fn download_link(&mut self, link: &Url) -> bool {
        let clear = link.as_str() == CLEAR_FINISHED;
        let cancel = link.as_str().starts_with(CANCEL_PREFIX);
        if !clear && !cancel {
            return false;
        }
        if !self.active_url_is(DOWNLOADS_URL) {
            return true; // 他のページからは操作させない
        }
        if clear {
            self.downloads.items.retain(DownloadItem::is_running);
        } else {
            let id = link
                .query_pairs()
                .find(|(k, _)| k == "id")
                .and_then(|(_, v)| v.parse::<u64>().ok());
            if let (Some(id), Some(fetcher)) = (id, &self.downloads.fetcher) {
                fetcher.cancel_download(id);
            }
        }
        self.reload();
        true
    }

    /// 一覧ページ用の HTML。
    pub fn downloads_html(&self) -> String {
        crate::downloads_page::downloads_html(
            &self.downloads.items,
            &self.downloads.dir.to_string_lossy(),
            self.lang,
        )
    }
}
