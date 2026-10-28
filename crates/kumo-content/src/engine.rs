//! Content のロジック：全タブのページを持ち、Browser からの要求を処理して通知を返す。
//! 取得は [`Work`] として外へ出し（ホストが別スレッドで動かす）、結果を [`ContentEngine::done`] で受ける。
//! 入出力を持たないので、プロセスでもスレッドでも、テストでは直接でも同じように動かせる。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use kumo_dom::NodeId;
use kumo_net::process::Fetcher;
use kumo_net::protocol::FetchError;
use url::Url;

use crate::frame::FrameWriter;
use crate::page::Page;
use crate::protocol::{FromContent, HoverKind, TabId, ToContent};
use crate::services::{PageImages, Services};
use crate::work::{Done, FetchJob, Fetched, Work};

/// 何も開いていないタブに表示するページ。
pub const BLANK: &str =
    "<html><head><title>新しいタブ</title></head><body style='background:#fff'></body></html>";

#[derive(Default)]
pub(crate) struct ContentTab {
    pub page: Option<Page>,
    /// 最後に依頼された読み込み
    pub nav: u64,
    pub scroll_y: f32,
    /// ビューポートや DOM が変わった（表示前に再レイアウトが必要）
    pub needs_layout: bool,
    /// 文字入力中のフォーム部品
    pub focused: Option<NodeId>,
    /// 最後に知らせたマウスの下のもの
    pub hover: Option<(Option<String>, HoverKind)>,
    /// 入力欄にフォーカスしたときの値（外したとき `change` を送るかの判定）
    pub focus_value: Option<String>,
    /// スクリプトの未捕捉の例外の数
    pub js_errors: usize,
    /// 最後にタイマーを動かした時刻（背景のタブは間隔を空ける）
    pub last_tick: Duration,
}

pub struct ContentEngine {
    pub(crate) services: Services,
    pub(crate) tabs: HashMap<TabId, ContentTab>,
    pub(crate) active: Option<TabId>,
    pub(crate) viewport: (u32, u32),
    pub(crate) frame: FrameWriter,
    pub(crate) out: Vec<FromContent>,
    pub(crate) jobs: Vec<Work>,
    /// Content の時計（タイマーと `performance.now()` の基準）
    pub(crate) now: Duration,
    /// スクリプトを実行するか
    javascript: bool,
    /// 読み込みの負荷レベル
    pub(crate) level: crate::LoadLevel,
    /// 画像を読み込むか（設定）
    images: bool,
    /// エラーページの言語
    lang: crate::i18n::Lang,
    pub(crate) private_bytes: fn() -> u64,
}

impl ContentEngine {
    /// `private_bytes` はこのプロセスのメモリ使用量（統計用）を返す関数。
    pub fn new(fetcher: Arc<dyn Fetcher>, private_bytes: fn() -> u64) -> Self {
        Self {
            services: Services::new(fetcher),
            tabs: HashMap::new(),
            active: None,
            viewport: (800, 600),
            frame: FrameWriter::default(),
            out: Vec::new(),
            jobs: Vec::new(),
            now: Duration::ZERO,
            javascript: true,
            level: crate::LoadLevel::default(),
            images: true,
            lang: crate::i18n::Lang::default(),
            private_bytes,
        }
    }

    pub fn fetcher(&self) -> Arc<dyn Fetcher> {
        self.services.fetcher.clone()
    }

    /// テスト・計測用：タブのページ。
    pub fn page(&self, tab: TabId) -> Option<&Page> {
        self.tabs.get(&tab)?.page.as_ref()
    }

    /// 溜まった通知を取り出す。
    pub fn take_output(&mut self) -> Vec<FromContent> {
        std::mem::take(&mut self.out)
    }

    /// 溜まった取得の依頼を取り出す。
    pub fn take_jobs(&mut self) -> Vec<Work> {
        std::mem::take(&mut self.jobs)
    }

    /// 取り出した依頼を戻す（テストで取得を遅らせるため）。
    pub fn requeue(&mut self, job: Work) {
        self.jobs.push(job);
    }

    pub(crate) fn viewport_f32(&self) -> (f32, f32) {
        as_f32(self.viewport)
    }

    fn apply_images(&self) {
        self.services
            .set_images_enabled(self.images && self.level.images());
    }

    /// 要求を1つ処理する。`Shutdown` はホストが扱う。
    pub fn handle(&mut self, message: ToContent) {
        match message {
            ToContent::Viewport { width, height } => self.set_viewport(width, height),
            ToContent::Settings { javascript } => self.javascript = javascript,
            ToContent::LoadLevel(level) => {
                self.level = level;
                self.apply_images();
            }
            ToContent::Language(lang) => self.lang = lang,
            ToContent::Images(on) => {
                self.images = on;
                self.apply_images();
            }
            ToContent::Load {
                tab,
                nav,
                url,
                post_body,
            } => self.load(tab, nav, &url, post_body),
            ToContent::LoadHtml {
                tab,
                nav,
                url,
                html,
            } => {
                self.blur(tab);
                self.tabs.entry(tab).or_default().nav = nav;
                let base = Url::parse(&url)
                    .unwrap_or_else(|_| Url::parse("about:blank").expect("固定の URL"));
                let page = Page::from_html(&html, base, &mut self.services, as_f32(self.viewport));
                self.show_page(tab, nav, page);
            }
            ToContent::Close { tab } => {
                self.tabs.remove(&tab);
                if self.active == Some(tab) {
                    self.active = None;
                }
            }
            ToContent::Show { tab } => {
                self.active = Some(tab);
                self.tabs.entry(tab).or_default();
                self.frame.invalidate();
            }
            ToContent::Freeze { tab } => self.freeze(tab),
            ToContent::Discard { tab } => self.discard(tab),
            ToContent::ScrollBy { tab, dy } => self.scroll_by(tab, dy),
            ToContent::ScrollToFragment { tab, url } => self.scroll_to_fragment(tab, &url),
            ToContent::Click { tab, x, y, new_tab } => self.click(tab, x, y, new_tab),
            ToContent::Key { tab, key } => self.key(tab, key),
            ToContent::Hover { tab, x, y } => self.hover(tab, x, y),
            ToContent::FrameDone => self.frame.done(),
            ToContent::Sync { id } => {
                self.render();
                self.out.push(FromContent::Synced { id });
            }
            ToContent::Stats { id } => {
                let stats = self.stats();
                self.out.push(FromContent::Stats { id, stats });
            }
            ToContent::Eval { id, tab, code } => {
                let result = self.eval(tab, &code);
                self.out.push(FromContent::EvalResult { id, result });
            }
            ToContent::Autofill {
                tab,
                url,
                username,
                password,
            } => self.autofill(tab, &url, &username, &password),
            ToContent::Shutdown => {}
        }
    }

    /// 背景のタブを凍結する。表示中のタブは対象にしない。
    fn freeze(&mut self, id: TabId) {
        if self.active == Some(id) {
            return;
        }
        if let Some(page) = self.tabs.get_mut(&id).and_then(|t| t.page.as_mut()) {
            page.freeze();
            self.jobs.push(Work::Trim);
        }
    }

    /// 背景のタブを破棄する（ページを捨てる。Browser が表示するときに読み込み直す）。
    fn discard(&mut self, id: TabId) {
        if self.active == Some(id) {
            return;
        }
        self.blur(id);
        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.page = None;
            tab.hover = None;
        }
        self.jobs.retain(|job| job.tab() != id);
        self.jobs.push(Work::Trim);
        kumo_mem::release_free_memory();
    }

    fn set_viewport(&mut self, width: u32, height: u32) {
        let viewport = (width.max(1), height.max(1));
        if viewport == self.viewport {
            return;
        }
        self.viewport = viewport;
        for tab in self.tabs.values_mut() {
            tab.needs_layout = true;
        }
        self.frame.invalidate();
    }

    fn load(&mut self, tab: TabId, nav: u64, url: &str, post_body: Option<Vec<u8>>) {
        self.blur(tab);
        self.tabs.entry(tab).or_default().nav = nav;
        let parsed = Url::parse(url);
        match parsed {
            Ok(url) if url.as_str() == "about:blank" => {
                let page = Page::from_html(BLANK, url, &mut self.services, as_f32(self.viewport));
                self.show_page(tab, nav, page);
            }
            Ok(url) => self.jobs.push(Work::Document(FetchJob {
                tab,
                nav,
                url,
                post_body,
                level: self.level,
                images: self.images,
            })),
            Err(e) => {
                let page = self.error_page(url, &FetchError::BadUrl(e.to_string()));
                self.show_page(tab, nav, page);
            }
        }
    }

    /// 取得が終わった。古い読み込み（その後に別の移動が依頼された）やもう閉じたタブの結果は捨てる。
    pub fn fetched(&mut self, fetched: Fetched) {
        let current = self
            .tabs
            .get(&fetched.tab)
            .is_some_and(|t| t.nav == fetched.nav);
        if !current {
            return;
        }
        let page = match fetched.result {
            Ok(page) => Page::build(
                page,
                &mut self.services,
                as_f32(self.viewport),
                (self.javascript && self.level.javascript()).then_some(self.now),
                self.level.script_limits(),
            ),
            Err(e) => self.error_page(fetched.url.as_str(), &e),
        };
        self.show_page(fetched.tab, fetched.nav, page);
        self.pump_js(fetched.tab);
    }

    /// 取得スレッドの仕事が終わった。
    pub fn done(&mut self, done: Done) {
        match done {
            Done::Document(fetched) => self.fetched(*fetched),
            Done::Sub(sub) => self.sub_fetched(sub),
        }
    }

    fn error_page(&mut self, url: &str, error: &FetchError) -> Page {
        let html = crate::error_page::error_html(self.lang, url, error);
        let base =
            Url::parse(url).unwrap_or_else(|_| Url::parse("about:blank").expect("固定の URL"));
        Page::from_html(&html, base, &mut self.services, as_f32(self.viewport))
    }

    fn show_page(&mut self, id: TabId, nav: u64, page: Page) {
        let height = self.viewport_f32().1;
        let url = page.url().to_string();
        let title = page.title().to_owned();
        let target = crate::anchor_y(&page, page.url());
        let tab = self.tabs.entry(id).or_default();
        tab.page = Some(page);
        tab.needs_layout = false;
        tab.focused = None;
        tab.hover = None;
        tab.js_errors = 0;
        tab.scroll_y = 0.0;
        if let Some(y) = target {
            tab.scroll_y = y;
            tab.clamp_scroll(height);
        }
        let y = tab.scroll_y;
        self.out.push(FromContent::Loaded {
            tab: id,
            nav,
            url,
            title,
        });
        self.out.push(FromContent::Scrolled { tab: id, y });
        if self.active == Some(id) {
            self.frame.invalidate();
        }
        kumo_mem::release_free_memory();
    }

    /// 表示中のタブを（必要なら再レイアウトしてから）共有フレームに描く。
    pub fn render(&mut self) {
        let Some(id) = self.active else { return };
        self.ensure_layout(id);
        let size = self.viewport;
        let Some(tab) = self.tabs.get(&id) else {
            return;
        };
        let page = tab.page.as_ref();
        let base = page
            .map(Page::url)
            .cloned()
            .unwrap_or_else(|| Url::parse("about:blank").expect("固定の URL"));
        let mut images = PageImages {
            base: &base,
            store: &mut self.services.images,
        };
        if let Some(message) = self.frame.render(id, page, &mut images, tab.scroll_y, size) {
            self.out.push(message);
        }
    }

    fn ensure_layout(&mut self, id: TabId) {
        let viewport = self.viewport_f32();
        let Some(tab) = self.tabs.get_mut(&id) else {
            return;
        };
        let stale = tab
            .page
            .as_ref()
            .is_some_and(|p| p.is_frozen() || p.viewport() != viewport);
        if !(tab.needs_layout || stale) {
            return;
        }
        if let Some(page) = tab.page.as_mut() {
            page.relayout(viewport, &mut self.services);
        }
        tab.needs_layout = false;
        let before = tab.scroll_y;
        tab.clamp_scroll(viewport.1);
        if tab.scroll_y != before {
            self.out.push(FromContent::Scrolled {
                tab: id,
                y: tab.scroll_y,
            });
        }
        self.frame.invalidate();
    }
}

impl ContentTab {
    /// スクロール位置をページの範囲に収める。
    pub fn clamp_scroll(&mut self, viewport_height: f32) {
        let max = self
            .page
            .as_ref()
            .map_or(0.0, |p| (p.document_height() - viewport_height).max(0.0));
        self.scroll_y = self.scroll_y.clamp(0.0, max).round();
    }
}

fn as_f32(size: (u32, u32)) -> (f32, f32) {
    (size.0 as f32, size.1 as f32)
}
