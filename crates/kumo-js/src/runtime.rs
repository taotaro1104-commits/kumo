//! JS ランタイムの公開 API。Content はこれを通して、スクリプトの実行・イベントの送出・タイマーの進行・
//! `fetch` の受け渡しを行う。

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use boa_engine::{Context, JsString, JsValue, Source};
use kumo_dom::{Document, NodeId};
use url::Url;

use crate::bootstrap::bootstrap;
use crate::dispatch;
use crate::events::describe_error;
use crate::host::{FetchOutcome, FetchRequest, Host};
use crate::natives::complete_fetch;
use crate::scripts::ScriptInfo;
use crate::timers;

/// ランタイムと Content が共有する DOM。
pub type SharedDocument = Rc<RefCell<Document>>;

/// イベントの初期値（`clientX` や `key` など）。Boa の型を呼び出し側に見せないための簡易な値。
#[derive(Clone, Debug, PartialEq)]
pub enum InitValue {
    Num(f64),
    Str(String),
    Bool(bool),
}

impl InitValue {
    fn to_js(&self) -> JsValue {
        match self {
            Self::Num(n) => JsValue::from(*n),
            Self::Str(s) => JsValue::from(JsString::from(s.as_str())),
            Self::Bool(b) => JsValue::from(*b),
        }
    }
}

pub struct JsRuntime {
    ctx: Context,
    host: Host,
    /// ネイティブ側の panic を捕まえた回数
    panics: u32,
    /// 実行したスクリプトの合計の大きさと、実行にかけた時間（予算の消費）
    script_bytes: usize,
    script_time: Duration,
}

/// 1つのループの反復回数の上限（超えると例外）。`while (true) {}` のようなループを止める。
const LOOP_ITERATION_LIMIT: u64 = 20_000_000;

/// 1つのスクリプトの大きさの上限。巨大なバンドルは純 Rust のエンジンでは実用的な時間・メモリで動かない。
pub const SCRIPT_MAX_BYTES: usize = 512 * 1024;
/// 1ページが実行するスクリプトの合計の大きさの予算（設計書の「JS ヒープ＋バイトコード 10MB」に収める）。
pub const SCRIPT_BYTES_BUDGET: usize = 1536 * 1024;
/// 1ページがスクリプトの実行に使える時間の予算。使い切ったら以降のスクリプトは実行しない。
pub const SCRIPT_TIME_BUDGET: Duration = Duration::from_secs(2);

/// この回数 panic したページの JS は止める（状態が壊れているかもしれないため）。
const MAX_PANICS: u32 = 3;

/// panic のペイロードを文字列にする。
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "不明な panic".to_owned())
}

impl JsRuntime {
    /// `doc` を DOM とするランタイムを作る。`url` は `location` の値。
    pub fn new(doc: SharedDocument, url: Url, viewport: (f32, f32)) -> Result<Self, String> {
        let host = Host::new(doc, url);
        host.state_mut().viewport = viewport;
        let mut ctx = Context::default();
        // 終わらないループでタブ（Content 全体）が固まらないよう、1つのループの反復に上限を設ける
        ctx.runtime_limits_mut()
            .set_loop_iteration_limit(LOOP_ITERATION_LIMIT);
        ctx.insert_data(host.clone());
        bootstrap(&mut ctx, &host)
            .map_err(|e| format!("JS の初期化に失敗: {}", describe_error(&e, &mut ctx)))?;
        Ok(Self {
            ctx,
            host,
            panics: 0,
            script_bytes: 0,
            script_time: Duration::ZERO,
        })
    }

    /// エンジンやバインディングの panic がプロセス全体（全タブ）を落とさないよう、実行を保護する。
    /// panic したら内部エラーとして記録し、`fallback` を返す。繰り返すページは以後実行しない。
    fn guarded<R>(&mut self, fallback: R, f: impl FnOnce(&mut Self) -> R) -> R {
        if self.panics >= MAX_PANICS {
            return fallback;
        }
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(self))) {
            Ok(value) => value,
            Err(payload) => {
                self.panics += 1;
                let message = panic_message(&*payload);
                self.host
                    .report_error(format!("内部エラー（panic）: {message}"));
                fallback
            }
        }
    }

    /// スクリプトを実行する。例外は記録し、`Err` で返す（ページの残りの処理は続けられる）。
    /// 大きすぎるものや、予算（合計の大きさ・実行時間）を使い切った後のものは実行しない。
    pub fn run_script(
        &mut self,
        source: &str,
        name: &str,
        node: Option<NodeId>,
    ) -> Result<(), String> {
        if let Some(reason) = self.over_budget(source.len()) {
            let message = format!("{name}: {reason}ので実行しません");
            let mut state = self.host.state_mut();
            if state.console.len() < crate::host::MAX_LOG {
                state.console.push(format!("[budget] {message}"));
            }
            return Err(message);
        }
        self.script_bytes += source.len();
        let started = std::time::Instant::now();
        let outcome = self.run_script_unchecked(source, name, node);
        self.script_time += started.elapsed();
        outcome
    }

    /// 予算を超えるなら、その理由。
    fn over_budget(&self, bytes: usize) -> Option<&'static str> {
        if bytes > SCRIPT_MAX_BYTES {
            Some("1つのスクリプトとして大きすぎる")
        } else if self.script_bytes + bytes > SCRIPT_BYTES_BUDGET {
            Some("このページのスクリプトの大きさの予算を超える")
        } else if self.script_time >= SCRIPT_TIME_BUDGET {
            Some("このページのスクリプトの実行時間の予算を使い切った")
        } else {
            None
        }
    }

    /// これまでに実行したスクリプトの合計の大きさと実行時間（診断用）。
    pub fn script_usage(&self) -> (usize, Duration) {
        (self.script_bytes, self.script_time)
    }

    /// 不要になったオブジェクトを回収する（読み込みの後などに呼び、ページのメモリを小さく保つ）。
    pub fn collect_garbage(&mut self) {
        boa_gc::force_collect();
    }

    fn run_script_unchecked(
        &mut self,
        source: &str,
        name: &str,
        node: Option<NodeId>,
    ) -> Result<(), String> {
        self.guarded(
            Err(format!("{name}: 内部エラーで実行できません")),
            |js| {
                js.host.state_mut().current_script = node;
                let result = js.ctx.eval(Source::from_bytes(source));
                js.host.state_mut().current_script = None;
                let outcome = match result {
                    Ok(_) => Ok(()),
                    Err(error) => {
                        let message = format!("{name}: {}", describe_error(&error, &mut js.ctx));
                        js.host.report_error(message.clone());
                        Err(message)
                    }
                };
                js.run_jobs();
                outcome
            },
        )
    }

    /// 式を評価して文字列にする（テスト・診断用）。
    pub fn eval_to_string(&mut self, source: &str) -> Result<String, String> {
        self.guarded(
            Err("内部エラーで実行できません".to_owned()),
            |js| {
                let value = js
                    .ctx
                    .eval(Source::from_bytes(source))
                    .map_err(|e| describe_error(&e, &mut js.ctx))?;
                js.run_jobs();
                value
                    .to_string(&mut js.ctx)
                    .map(|s: JsString| s.to_std_string_escaped())
                    .map_err(|e| describe_error(&e, &mut js.ctx))
            },
        )
    }

    fn run_jobs(&mut self) {
        if let Err(error) = self.ctx.run_jobs() {
            let message = describe_error(&error, &mut self.ctx);
            self.host
                .report_error(format!("Uncaught (in promise) {message}"));
        }
    }

    /// `<script>` の実行に必要な情報。実行できない種類（モジュール・データブロック）や空なら `None`。
    pub fn script_info(&self, node: NodeId) -> Option<ScriptInfo> {
        let base = self.host.state().url.clone();
        let doc = self.host.doc();
        let doc = doc.borrow();
        crate::scripts::script_info(&doc, &base, node)
    }

    // ---- ライフサイクル ----

    /// `document.readyState` を進める。
    pub fn set_ready_state(&mut self, state: &str) {
        self.host.state_mut().ready_state = state.to_owned();
        self.guarded((), |js| {
            let root = js.host.doc().borrow().root();
            let _ = dispatch::dispatch_to_node(&mut js.ctx, root, "Event", "readystatechange", &[]);
        });
    }

    /// `DOMContentLoaded`（文書で発火し、window までバブリングする）。
    pub fn fire_dom_content_loaded(&mut self) {
        self.guarded((), |js| {
            let root = js.host.doc().borrow().root();
            let _ = dispatch::dispatch_to_node(&mut js.ctx, root, "Event", "DOMContentLoaded", &[]);
            js.run_jobs();
        });
    }

    /// `load`（window）。
    pub fn fire_load(&mut self) {
        self.guarded((), |js| {
            let _ = dispatch::dispatch_to_window(&mut js.ctx, "load");
            js.run_jobs();
        });
    }

    /// ユーザー操作のイベントを `node` に送る。既定の動作を止められたら `false`。
    pub fn dispatch_event(
        &mut self,
        node: NodeId,
        class: &str,
        kind: &str,
        init: &[(&str, InitValue)],
    ) -> bool {
        self.guarded(true, |js| {
            let values: Vec<(&str, JsValue)> =
                init.iter().map(|(name, v)| (*name, v.to_js())).collect();
            let result = dispatch::dispatch_to_node(&mut js.ctx, node, class, kind, &values);
            js.run_jobs();
            match result {
                Ok(proceed) => proceed,
                Err(error) => {
                    let message = describe_error(&error, &mut js.ctx);
                    js.host.report_error(format!("Uncaught {message}"));
                    true
                }
            }
        })
    }

    /// `click` の既定の動作（リンクの移動、チェックの切り替え）。
    pub fn default_click_action(&mut self, node: NodeId) {
        self.guarded((), |js| dispatch::default_click_action(&mut js.ctx, node));
    }

    // ---- 時間 ----

    /// 時刻を `now`（ランタイムを作ってからの経過）に進め、期限の来たタイマーを実行する。実行した数を返す。
    pub fn tick(&mut self, now: Duration) -> usize {
        self.guarded(0, |js| {
            let ran = timers::run_due(&mut js.ctx, now);
            js.run_jobs();
            ran
        })
    }

    /// 仮想時刻を進める（タイマーは実行しない）。新しく登録するタイマーの基準になる。
    pub fn set_time(&mut self, now: Duration) {
        self.host.state_mut().now = now;
    }

    /// 次のタイマーが来るまでの時間。タイマーがなければ `None`。
    pub fn next_timer_in(&self, now: Duration) -> Option<Duration> {
        timers::next_due(&self.host, now)
    }

    // ---- fetch ----

    /// スクリプトが出した `fetch` の要求を取り出す。
    pub fn take_fetches(&mut self) -> Vec<FetchRequest> {
        std::mem::take(&mut self.host.state_mut().fetches)
    }

    /// `fetch` の結果を渡す。
    pub fn fetch_done(&mut self, id: u64, outcome: Result<FetchOutcome, String>) {
        self.guarded((), |js| complete_fetch(&mut js.ctx, id, outcome));
    }

    /// panic で止めた（これ以上スクリプトを実行しない）か。
    pub fn is_broken(&self) -> bool {
        self.panics >= MAX_PANICS
    }

    // ---- 変更の取り出し ----

    /// DOM が変わったか（取り出すと戻る）。
    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.host.state_mut().dirty)
    }

    /// スクリプトが求めた移動先。
    pub fn take_navigation(&mut self) -> Option<String> {
        self.host.state_mut().navigation.take()
    }

    /// 動的に挿入された `<script>`。
    pub fn take_new_scripts(&mut self) -> Vec<NodeId> {
        std::mem::take(&mut self.host.state_mut().new_scripts)
    }

    pub fn take_console(&mut self) -> Vec<String> {
        std::mem::take(&mut self.host.state_mut().console)
    }

    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.host.state_mut().errors)
    }

    /// 表示領域の大きさを知らせる（`innerWidth` / `matchMedia`）。
    pub fn set_viewport(&mut self, viewport: (f32, f32)) {
        self.host.state_mut().viewport = viewport;
    }

    /// `location` が今指す URL（`history.pushState` で変わりうる）。
    pub fn url(&self) -> Url {
        self.host.state().url.clone()
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
