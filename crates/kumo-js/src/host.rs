//! JS 実行環境が共有する状態。Boa の `Context` のホストデータとして持ち、ネイティブ関数から引く。
//!
//! 借用は短くとる（JS を呼ぶ間は借用を持たない）：リスナーから DOM を書き換える再入があるため。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use boa_engine::{Context, JsObject, JsValue};
use kumo_dom::{Document, NodeId};
use url::Url;

pub(crate) type SharedDoc = Rc<RefCell<Document>>;

/// `window` をイベント対象として表すキー（ノードの添字と重ならない）。
pub(crate) const WINDOW_KEY: usize = usize::MAX;

/// 登録されたイベントリスナー。
#[derive(Clone)]
pub(crate) struct Listener {
    pub event: String,
    pub callback: JsObject,
    pub capture: bool,
    pub once: bool,
}

pub(crate) struct Timer {
    pub id: u32,
    pub due: Duration,
    /// `setInterval` の間隔
    pub interval: Option<Duration>,
    pub callback: JsObject,
    pub args: Vec<JsValue>,
}

/// `fetch` の要求（Content が Network プロセスへ流す）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchRequest {
    pub id: u64,
    pub url: String,
    pub method: String,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
}

/// `fetch` の結果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchOutcome {
    pub status: u16,
    pub url: String,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

pub(crate) struct HostState {
    pub doc: SharedDoc,
    pub url: Url,
    pub wrappers: HashMap<usize, JsObject>,
    pub protos: HashMap<&'static str, JsObject>,
    pub listeners: HashMap<usize, Vec<Listener>>,
    pub timers: Vec<Timer>,
    pub next_timer: u32,
    /// 仮想の現在時刻（ランタイムを作ってからの経過）。実行の前に Content が進める
    pub now: Duration,
    pub fetches: Vec<FetchRequest>,
    pub pending_fetches: HashMap<u64, (JsObject, JsObject)>,
    pub next_fetch: u64,
    /// DOM が変わった（再スタイル・再レイアウトが必要）
    pub dirty: bool,
    /// スクリプトが移動を求めた（`location.href = …`）
    pub navigation: Option<String>,
    pub console: Vec<String>,
    pub errors: Vec<String>,
    /// 動的に挿入された `<script>`（取得・実行待ち）
    pub new_scripts: Vec<NodeId>,
    pub cookie: String,
    /// `document.readyState`（loading → interactive → complete）
    pub ready_state: String,
    /// 実行中の `<script>`（`document.currentScript`）
    pub current_script: Option<NodeId>,
    /// 表示領域の大きさ（`innerWidth` / `matchMedia`）
    pub viewport: (f32, f32),
    /// 最後のレイアウトでの要素の境界ボックス（要素の添字 → 文書座標）と、ページのスクロール位置
    pub geometry: HashMap<usize, crate::geometry::Rect>,
    pub scroll_y: f32,
    /// `onclick="…"` のような属性ハンドラをコンパイルした関数（要素の添字・イベント名 → 属性の本文と関数）
    pub inline_handlers: HashMap<(usize, String), (String, JsObject)>,
}

/// 共有状態への参照（複製は参照カウントを増やすだけ）。
#[derive(Clone)]
pub(crate) struct Host(pub Rc<RefCell<HostState>>);

impl Host {
    pub fn new(doc: SharedDoc, url: Url) -> Self {
        Self(Rc::new(RefCell::new(HostState {
            doc,
            url,
            wrappers: HashMap::new(),
            protos: HashMap::new(),
            listeners: HashMap::new(),
            timers: Vec::new(),
            next_timer: 1,
            now: Duration::ZERO,
            fetches: Vec::new(),
            pending_fetches: HashMap::new(),
            next_fetch: 1,
            dirty: false,
            navigation: None,
            console: Vec::new(),
            errors: Vec::new(),
            new_scripts: Vec::new(),
            cookie: String::new(),
            ready_state: "loading".to_owned(),
            current_script: None,
            viewport: (1280.0, 800.0),
            geometry: HashMap::new(),
            scroll_y: 0.0,
            inline_handlers: HashMap::new(),
        })))
    }

    /// Boa のコンテキストから共有状態を取り出す。
    pub fn of(ctx: &Context) -> Self {
        ctx.get_data::<Host>()
            .expect("Host は JsRuntime::new で登録済み")
            .clone()
    }

    pub fn state(&self) -> std::cell::Ref<'_, HostState> {
        self.0.borrow()
    }

    pub fn state_mut(&self) -> std::cell::RefMut<'_, HostState> {
        self.0.borrow_mut()
    }

    pub fn doc(&self) -> SharedDoc {
        self.0.borrow().doc.clone()
    }

    pub fn mark_dirty(&self) {
        self.0.borrow_mut().dirty = true;
    }

    pub fn proto(&self, name: &str) -> JsObject {
        self.0
            .borrow()
            .protos
            .get(name)
            .unwrap_or_else(|| panic!("プロトタイプ {name} が未登録"))
            .clone()
    }

    /// エラーを記録する（長くなりすぎないよう上限を設ける）。
    pub fn report_error(&self, message: String) {
        let mut state = self.0.borrow_mut();
        if state.errors.len() < MAX_LOG {
            state.errors.push(message);
        }
    }
}

/// ページ `from` から `to` へ移動・取得してよいか。ローカルファイル（`file:`）へ行けるのは、
/// 自分自身もローカルファイルのページだけ（遠隔のページにローカルのファイルを読ませない）。
pub fn may_reach(from: &Url, to: &Url) -> bool {
    to.scheme() != "file" || from.scheme() == "file"
}

/// 記録するコンソール行・エラーの上限。
pub(crate) const MAX_LOG: usize = 500;
