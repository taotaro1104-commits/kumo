//! ブラウザ全体：タブの集合、移動、UI の状態、履歴。
//! ページの処理は Content（別プロセスまたはスレッド）が行い、ここは要求を送るだけで待たない。
//! Content からの通知は [`Browser::pump`] で取り込む。

use std::sync::Arc;

use kumo_content::protocol::{TabId, ToContent};
use kumo_content::{ContentClient, FrameReader, Notify, Page, Services, same_document};
use kumo_net::process::Fetcher;
use url::Url;

use crate::chrome::CHROME_HEIGHT;
use crate::governor::TabState;
use crate::history::HistoryDb;
use crate::newtab::NEW_TAB_URL;
use crate::offline::Offline;
use crate::tab::Tab;

pub struct Browser {
    pub(crate) content: ContentClient,
    /// UI のページを描くための資源（UI は外部資源を読まない）
    pub(crate) chrome_services: Services,
    pub(crate) tabs: Vec<Tab>,
    pub(crate) active: usize,
    next_id: TabId,
    next_nav: u64,
    pub(crate) history: Option<HistoryDb>,
    /// ウィンドウ全体の大きさ（UI を含む）
    pub(crate) window: (f32, f32),
    pub(crate) url_text: String,
    /// 変換中の文字（URL 欄に入力中のときだけ持つ）
    pub(crate) preedit: String,
    pub(crate) url_focused: bool,
    /// UI のページ（状態が変わったら作り直す）
    pub(crate) chrome: Option<Page>,
    pub(crate) frame: FrameReader,
    /// 届いたがまだ写していないフレーム（描き直した行）
    pub(crate) frame_damage: Option<(u32, u32)>,
    pub(crate) next_request: u64,
    /// Content が落ちたときに作り直す関数（なければ作り直さない）
    pub(crate) restart: Option<ContentFactory>,
    pub(crate) restarts: u32,
    /// Content が落ちて作り直せなかった
    pub(crate) content_lost: bool,
    /// 通知が届いたときに呼ぶ関数（作り直した Content にも設定する）
    pub(crate) notify: Option<Notify>,
    /// マウスが乗っている UI のリンク
    pub(crate) chrome_hover: Option<String>,
    /// リンク先の吹き出し
    pub(crate) bubble: Option<crate::pointer::BubbleCache>,
    /// メモリ管理（凍結・破棄の方針と状態）
    pub(crate) memory: crate::memory::MemoryState,
    /// 読み込みの負荷レベル（Content を作り直したら送り直す）
    pub(crate) load_level: kumo_content::LoadLevel,
    /// スクリプトを実行するか（`--no-js`）
    pub(crate) javascript: bool,
    /// 画像を読み込むか
    pub(crate) images: bool,
    /// パスワードを保存・自動入力するか
    pub(crate) passwords: bool,
    /// 表示言語（UI・内部ページ・エラーページ）
    pub(crate) lang: kumo_content::Lang,
    /// パスワードの暗号鍵（必要になってから OS の資格情報ストアで得る）
    pub(crate) vault: Option<crate::vault::Vault>,
    /// 保存を確認中のログイン情報
    pub(crate) pending_login: Option<crate::login_actions::PendingLogin>,
    /// 一覧ページでパスワードを表示している組（origin, ユーザー名）
    pub(crate) revealed: Option<(String, String)>,
    /// ダウンロードの一覧と保存先
    pub(crate) downloads: crate::downloads::Downloads,
}

/// Content を作る関数（落ちたときの作り直し用）。
pub type ContentFactory = Box<dyn FnMut() -> std::io::Result<ContentClient>>;

/// 移動・操作の結果、画面のどこを描き直すべきか。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Redraw {
    pub chrome: bool,
    /// 新しいコンテンツのフレームが届いた
    pub content: bool,
}

impl Redraw {
    pub const ALL: Self = Self {
        chrome: true,
        content: true,
    };
    pub const CHROME: Self = Self {
        chrome: true,
        content: false,
    };
    pub const CONTENT: Self = Self {
        chrome: false,
        content: true,
    };

    pub fn merge(self, other: Self) -> Self {
        Self {
            chrome: self.chrome || other.chrome,
            content: self.content || other.content,
        }
    }
}

impl Browser {
    pub fn new(content: ContentClient, history: Option<HistoryDb>, window: (f32, f32)) -> Self {
        let load_level = crate::home::saved_load_level(history.as_ref());
        let images = history.as_ref().is_none_or(HistoryDb::images_enabled);
        let lang = history
            .as_ref()
            .and_then(HistoryDb::language)
            .unwrap_or_default();
        let passwords = history.as_ref().is_some_and(HistoryDb::passwords_enabled);
        let browser = Self {
            content,
            chrome_services: Services::new(Arc::new(Offline)),
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            next_nav: 1,
            history,
            window,
            url_text: String::new(),
            preedit: String::new(),
            url_focused: false,
            chrome: None,
            frame: FrameReader::default(),
            frame_damage: None,
            next_request: 1,
            restart: None,
            restarts: 0,
            content_lost: false,
            notify: None,
            chrome_hover: None,
            bubble: None,
            memory: crate::memory::MemoryState::default(),
            load_level,
            javascript: true,
            images,
            passwords,
            lang,
            vault: None,
            pending_login: None,
            revealed: None,
            downloads: crate::downloads::Downloads::default(),
        };
        browser.send_viewport();
        browser.send_load_level();
        browser.send_images();
        browser.send_language();
        browser
    }

    /// Content を同じプロセスのスレッドで動かす（テスト・`--in-process`）。
    pub fn in_process(
        fetcher: Arc<dyn Fetcher>,
        history: Option<HistoryDb>,
        window: (f32, f32),
    ) -> Self {
        Self::new(ContentClient::spawn_thread(fetcher), history, window)
    }

    /// コンテンツ領域の大きさ。
    pub fn content_viewport(&self) -> (f32, f32) {
        (self.window.0, (self.window.1 - CHROME_HEIGHT).max(1.0))
    }

    pub(crate) fn send_viewport(&self) {
        let (w, h) = self.content_viewport();
        self.content.send(ToContent::Viewport {
            width: w as u32,
            height: h as u32,
        });
    }

    pub fn content_client(&self) -> &ContentClient {
        &self.content
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    pub(crate) fn tab_index(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// 新しいタブを開く。`url` が `None` なら空白ページ。
    pub fn open_tab(&mut self, url: Option<Url>, activate: bool) -> Redraw {
        self.tabs.push(Tab::new(self.next_id));
        self.next_id += 1;
        let index = self.tabs.len() - 1;
        let blank = url.is_none();
        let target = url.unwrap_or_else(|| Url::parse(NEW_TAB_URL).expect("固定の URL"));
        self.start_load(index, target, !blank, None);
        if activate {
            self.activate(index);
            if blank {
                self.url_text.clear();
                self.url_focused = true;
            }
        }
        self.invalidate_chrome()
    }

    pub fn close_tab(&mut self, index: usize) -> Redraw {
        if index >= self.tabs.len() {
            return Redraw::default();
        }
        let tab = self.tabs.remove(index);
        self.content.send(ToContent::Close { tab: tab.id });
        if self.tabs.is_empty() {
            return self.open_tab(None, true);
        }
        if self.active > index || self.active >= self.tabs.len() {
            self.active = self.active.saturating_sub(1);
        }
        self.activate(self.active)
    }

    pub fn activate(&mut self, index: usize) -> Redraw {
        let Some(id) = self.tabs.get(index).map(|t| t.id) else {
            return Redraw::default();
        };
        self.content.send(ToContent::Show { tab: id });
        let now = std::time::Instant::now();
        self.leave(self.active, now);
        self.active = index;
        self.url_focused = false;
        self.url_text = self.tabs[index]
            .url()
            .map(Url::to_string)
            .unwrap_or_default();
        // 破棄したタブはここで読み込み直す（URL 欄は読み込む URL に合わせる）
        self.wake(index, now);
        // 前のタブのフレームは使わない（新しいタブのフレームが届くまで待つ）
        self.discard_frame();
        self.invalidate_chrome()
    }

    /// 現在のタブで移動する。
    pub fn navigate(&mut self, url: Url) -> Redraw {
        if self.tabs.is_empty() {
            return self.open_tab(Some(url), true);
        }
        self.load(self.active, url, true, false, None);
        self.invalidate_chrome()
    }

    pub fn back(&mut self) -> Redraw {
        let Some(url) = self
            .tabs
            .get_mut(self.active)
            .and_then(|t| t.session.back().cloned())
        else {
            return Redraw::default();
        };
        self.load(self.active, url, false, false, None);
        self.invalidate_chrome()
    }

    pub fn forward(&mut self) -> Redraw {
        let Some(url) = self
            .tabs
            .get_mut(self.active)
            .and_then(|t| t.session.forward().cloned())
        else {
            return Redraw::default();
        };
        self.load(self.active, url, false, false, None);
        self.invalidate_chrome()
    }

    pub fn reload(&mut self) -> Redraw {
        let Some(url) = self.active_tab().and_then(|t| t.url().cloned()) else {
            return Redraw::default();
        };
        self.load(self.active, url, false, true, None);
        self.invalidate_chrome()
    }

    /// 読み込みを依頼する。`push` なら履歴に積む。
    /// 表示中と同じ文書の `#id` への移動は、`force`（再読み込み）でなければ取得せずスクロールだけする。
    pub(crate) fn load(
        &mut self,
        index: usize,
        url: Url,
        push: bool,
        force: bool,
        post_body: Option<Vec<u8>>,
    ) {
        if self.page_action(index, &url) {
            return;
        }
        if !force && post_body.is_none() && self.scroll_within_document(index, &url, push) {
            return;
        }
        self.start_load(index, url, push, post_body);
    }

    pub(crate) fn start_load(
        &mut self,
        index: usize,
        url: Url,
        push: bool,
        post_body: Option<Vec<u8>>,
    ) {
        let nav = self.next_nav;
        self.next_nav += 1;
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        if push {
            tab.session.push(url.clone());
        }
        tab.loading = Some(nav);
        tab.state = TabState::Live;
        tab.posted = post_body.is_some();
        tab.hover_link = None;
        tab.hover_kind = kumo_content::protocol::HoverKind::None;
        tab.field_focused = false;
        let id = tab.id;
        let message = self.load_message(id, nav, &url, post_body);
        self.content.send(message);
        if index == self.active {
            self.url_text = url.to_string();
            self.url_focused = false;
        }
    }

    /// `url` が表示中の文書の `#id` なら、取得せずにその位置へスクロールさせる。
    fn scroll_within_document(&mut self, index: usize, url: &Url, push: bool) -> bool {
        let Some(tab) = self.tabs.get_mut(index) else {
            return false;
        };
        let Some(shown) = &tab.shown_url else {
            return false;
        };
        // 新しい移動で # がないものは通常の読み込み。戻る・進むで同じ文書なら取得し直さない
        if tab.is_loading() || !same_document(shown, url) || (push && url.fragment().is_none()) {
            return false;
        }
        if push {
            tab.session.push(url.clone());
        }
        tab.shown_url = Some(url.clone());
        self.content.send(ToContent::ScrollToFragment {
            tab: tab.id,
            url: url.to_string(),
        });
        if index == self.active {
            self.url_text = url.to_string();
            self.url_focused = false;
        }
        true
    }

    /// ウィンドウの大きさが変わった。再レイアウトは Content が表示中のタブから行う。
    pub fn resize(&mut self, width: f32, height: f32) -> Redraw {
        if self.window == (width, height) {
            return Redraw::default();
        }
        self.window = (width, height);
        self.send_viewport();
        self.discard_frame();
        self.chrome = None;
        Redraw::CHROME
    }

    /// 届いたがまだ写していないフレームを捨てる。Content は写し終えた通知を待って次を描くので、
    /// 捨てるときも必ず通知する（しないと以後のフレームが届かなくなる）。
    pub(crate) fn discard_frame(&mut self) {
        if self.frame_damage.take().is_some() {
            self.content.send(ToContent::FrameDone);
        }
    }

    pub(crate) fn invalidate_chrome(&mut self) -> Redraw {
        self.chrome = None;
        Redraw::CHROME
    }
}
