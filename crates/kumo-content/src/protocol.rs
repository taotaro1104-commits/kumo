//! Browser ⇔ Content のメッセージ。フレーミングは Network と同じ（[`kumo_net::protocol`]）。
//!
//! Browser は要求を送るだけで待たない。Content は読み込みの完了・移動の依頼・描いたフレームを通知する。
//! 画素は共有メモリのフレームに描き、[`FromContent::Frame`] でその場所と描き直した行を知らせる。
//! Browser は写し終えたら [`ToContent::FrameDone`] を返す（それまで Content は次のフレームを描かない）。

use serde::{Deserialize, Serialize};

/// タブの識別子（Browser が決める）。
pub type TabId = u64;

/// 入力中の欄へのキー操作。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldKey {
    Text(String),
    Backspace,
    /// textarea なら改行、それ以外はフォーム送信
    Enter,
    /// フォーカスを外す
    Blur,
}

/// マウスの下にあるもの（カーソルの形と、リンク先の表示に使う）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoverKind {
    #[default]
    None,
    Link,
    /// 文字の入力欄
    Field,
    /// ボタン・チェックボックスなどの部品
    Control,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToContent {
    /// コンテンツ領域の大きさ（px）
    Viewport {
        width: u32,
        height: u32,
    },
    /// 設定。`javascript` が偽ならスクリプトを実行しない（読み込む前に送る）
    Settings {
        javascript: bool,
    },
    /// 表示言語（以後に作るエラーページから効く）
    Language(crate::i18n::Lang),
    /// 画像を読み込むか（以後に始める読み込みから効く）
    Images(bool),
    /// 読み込みの負荷レベル（以後に始める読み込みから効く）
    LoadLevel(crate::LoadLevel),
    /// `url` を読み込む。`nav` は Browser が付ける通し番号（古い読み込みの結果を捨てるため）。
    /// `post_body` があればフォームを POST で送信する。
    Load {
        tab: TabId,
        nav: u64,
        url: String,
        post_body: Option<Vec<u8>>,
    },
    /// Browser が作った HTML を表示する（新しいタブのページなど）。外部資源は読まない
    LoadHtml {
        tab: TabId,
        nav: u64,
        url: String,
        html: String,
    },
    Close {
        tab: TabId,
    },
    /// 表示するタブを切り替える（必ず全面を描き直す）
    Show {
        tab: TabId,
    },
    /// 背景のタブを凍結する（スクリプトと描画データを捨て、DOM だけ残す。表示すると再レイアウトする）
    Freeze {
        tab: TabId,
    },
    /// 背景のタブを破棄する（ページを丸ごと捨てる。表示するには読み込み直す）
    Discard {
        tab: TabId,
    },
    ScrollBy {
        tab: TabId,
        dy: f32,
    },
    /// 表示中の文書の `#id` へスクロールする（取得し直さない移動）
    ScrollToFragment {
        tab: TabId,
        url: String,
    },
    /// コンテンツ領域でのクリック（`y` は領域上端から）。`new_tab` ならリンクを新しいタブで開く
    Click {
        tab: TabId,
        x: f32,
        y: f32,
        new_tab: bool,
    },
    Key {
        tab: TabId,
        key: FieldKey,
    },
    /// マウスの位置（`y` は領域上端から）。下にあるものが変わったら [`FromContent::Hovered`] を返す
    Hover {
        tab: TabId,
        x: f32,
        y: f32,
    },
    /// 前のフレームを写し終えた
    FrameDone,
    /// ここまでの要求を処理したら [`FromContent::Synced`] を返す（描画も済ませる）
    Sync {
        id: u64,
    },
    Stats {
        id: u64,
    },
    /// 診断用：タブのページで JS の式を評価して [`FromContent::EvalResult`] を返す
    Eval {
        id: u64,
        tab: TabId,
        code: String,
    },
    /// 保存したログイン情報をページに入れる（ページがまだ `url` のときだけ）
    Autofill {
        tab: TabId,
        url: String,
        username: String,
        password: String,
    },
    Shutdown,
}

/// 計測用の数値（about:memory 相当）。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentStats {
    pub tabs: usize,
    pub dom_nodes: usize,
    pub display_items: usize,
    /// スクリプトを動かしているタブの数
    pub js_tabs: usize,
    /// スクリプトの未捕捉の例外の数
    pub js_errors: usize,
    /// 凍結しているタブの数
    pub frozen_tabs: usize,
    pub decoded_image_bytes: u64,
    pub compressed_image_bytes: u64,
    pub glyph_bytes: u64,
    pub frame_bytes: u64,
    /// Content プロセスのメモリ使用量（プロセス内で動かすときは 0）
    pub private_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FromContent {
    /// 読み込みが終わった（失敗時はエラーページを表示した）
    Loaded {
        tab: TabId,
        nav: u64,
        url: String,
        title: String,
    },
    /// ページ上の操作で移動したい（リンク・フォーム送信）
    Navigate {
        tab: TabId,
        url: String,
        post_body: Option<Vec<u8>>,
        new_tab: bool,
    },
    /// ログインのフォームを送信した（`url` は送信したページ）。保存を確かめるのは Browser
    LoginSubmitted {
        tab: TabId,
        url: String,
        username: String,
        password: String,
    },
    /// 入力欄へのフォーカスが変わった（キー入力の振り分け用）
    FieldFocus {
        tab: TabId,
        focused: bool,
    },
    Scrolled {
        tab: TabId,
        y: f32,
    },
    /// スクリプトが題名を変えた
    TitleChanged {
        tab: TabId,
        title: String,
    },
    /// マウスの下にあるものが変わった（`link` は解決済みのリンク先）
    Hovered {
        tab: TabId,
        link: Option<String>,
        kind: HoverKind,
    },
    /// 共有メモリ `path` に `width`×`height` の 0RGB 画素を描いた。`damage` は描き直した行 [start, end)
    Frame {
        tab: TabId,
        path: String,
        width: u32,
        height: u32,
        damage: (u32, u32),
    },
    Synced {
        id: u64,
    },
    Stats {
        id: u64,
        stats: ContentStats,
    },
    /// [`ToContent::Eval`] の結果（`Err` は例外・JS のないページ）
    EvalResult {
        id: u64,
        result: Result<String, String>,
    },
}
