//! Content と JS ランタイムの結びつき：タイマーの進行、イベントの送出、`fetch`・動的スクリプトの受け渡し、
//! スクリプトが起こした変化（DOM の更新・移動・題名）の取り込み。

use std::time::Duration;

use kumo_dom::NodeId;
use kumo_js::{InitValue, JsRuntime};

use crate::engine::ContentEngine;
use crate::protocol::{FromContent, TabId};
use crate::work::{SubFetched, SubJob, SubKind, Work};

/// 背景のタブのタイマーを動かす間隔（ブラウザの背景タブの制限に合わせる）。
const BACKGROUND_TICK: Duration = Duration::from_secs(1);
/// 1回の取り込みで、動的スクリプトの実行が新しいスクリプトを呼ぶ連鎖を何段まで追うか。
const MAX_ROUNDS: usize = 8;
/// 1ページが記録するスクリプトの例外の数の上限。
const MAX_ERRORS_KEPT: usize = 200;

/// `KUMO_JS_LOG` が設定されていれば、コンソール出力と例外を標準エラーへ出す。
fn log_enabled() -> bool {
    std::env::var_os("KUMO_JS_LOG").is_some()
}

impl ContentEngine {
    /// 時計を進める（タイマーと `performance.now()` の基準）。ホストがイベントの処理前に呼ぶ。
    pub fn set_time(&mut self, now: Duration) {
        self.now = now;
    }

    /// タブの JS ランタイムで `f` を実行し、起こった変化を取り込む。ランタイムがなければ `None`。
    pub(crate) fn with_js<R>(
        &mut self,
        id: TabId,
        f: impl FnOnce(&mut JsRuntime) -> R,
    ) -> Option<R> {
        let now = self.now;
        let result = {
            let js = self.tabs.get_mut(&id)?.page.as_mut()?.js_mut()?;
            js.set_time(now);
            f(js)
        };
        self.pump_js(id);
        Some(result)
    }

    /// ユーザー操作をイベントとして JS に渡す。既定の動作を止められたら `false`（JS がなければ `true`）。
    pub(crate) fn js_event(
        &mut self,
        id: TabId,
        node: NodeId,
        class: &str,
        kind: &str,
        init: &[(&str, InitValue)],
    ) -> bool {
        self.with_js(id, |js| js.dispatch_event(node, class, kind, init))
            .unwrap_or(true)
    }

    /// スクリプトが起こした変化を取り込む：DOM の更新・移動・題名・動的スクリプト・`fetch`。
    /// 診断用：タブのページで式を評価する。評価後に DOM が変わっていれば描画に反映する。
    pub(crate) fn eval(&mut self, id: TabId, code: &str) -> Result<String, String> {
        let result = self
            .tabs
            .get_mut(&id)
            .and_then(|t| t.page.as_mut())
            .ok_or_else(|| "ページがありません".to_owned())?
            .js_mut()
            .ok_or_else(|| "このページはスクリプトを実行していません".to_owned())?
            .eval_to_string(code);
        self.pump_js(id);
        result
    }

    pub(crate) fn pump_js(&mut self, id: TabId) {
        for _ in 0..MAX_ROUNDS {
            let Some(changes) = self.take_js_changes(id) else {
                return;
            };
            let more = self.apply_js_changes(id, changes);
            if !more {
                return;
            }
        }
    }

    fn take_js_changes(&mut self, id: TabId) -> Option<JsChanges> {
        let tab = self.tabs.get_mut(&id)?;
        let page = tab.page.as_mut()?;
        let js = page.js_mut()?;
        Some(JsChanges {
            dirty: js.take_dirty(),
            navigation: js.take_navigation(),
            scripts: js.take_new_scripts(),
            fetches: js.take_fetches(),
            errors: js.take_errors(),
            console: js.take_console(),
        })
    }

    /// 取り込む。実行したスクリプトがさらに変化を起こしたかもしれないとき `true`。
    fn apply_js_changes(&mut self, id: TabId, changes: JsChanges) -> bool {
        let nav = self.tabs.get(&id).map_or(0, |t| t.nav);
        if log_enabled() {
            for line in &changes.console {
                eprintln!("[js {id}] {line}");
            }
            for line in &changes.errors {
                eprintln!("[js {id}] {line}");
            }
        }
        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.js_errors = (tab.js_errors + changes.errors.len()).min(MAX_ERRORS_KEPT);
        }
        let mut ran_script = false;
        let source = self
            .tabs
            .get(&id)
            .and_then(|t| t.page.as_ref())
            .map_or_else(String::new, |p| p.url().to_string());
        for node in changes.scripts {
            let info = self.with_js_quiet(id, |js| js.script_info(node)).flatten();
            let Some(info) = info else { continue };
            match info.src {
                None => {
                    self.with_js_quiet(id, |js| js.run_script(&info.text, "inline", Some(node)));
                    ran_script = true;
                }
                Some(url) => self.jobs.push(Work::Sub(SubJob {
                    tab: id,
                    nav,
                    kind: SubKind::Script(node),
                    url,
                    method: "GET".to_owned(),
                    content_type: None,
                    body: None,
                    source: source.clone(),
                    level: self.level,
                })),
            }
        }
        for request in changes.fetches {
            self.jobs.push(Work::Sub(SubJob {
                tab: id,
                nav,
                kind: SubKind::Api(request.id),
                url: request.url,
                method: request.method,
                content_type: request.content_type,
                body: request.body,
                source: source.clone(),
                level: self.level,
            }));
        }
        if changes.dirty
            && let Some(tab) = self.tabs.get_mut(&id)
        {
            tab.needs_layout = true;
            let title = tab
                .page
                .as_mut()
                .and_then(|p| p.refresh_title().then(|| p.title().to_owned()));
            if let Some(title) = title {
                self.out.push(FromContent::TitleChanged { tab: id, title });
            }
        }
        if let Some(url) = changes.navigation {
            self.out.push(FromContent::Navigate {
                tab: id,
                url,
                post_body: None,
                new_tab: false,
            });
        }
        ran_script
    }

    /// タブのページから `url` へ移動してよいか（遠隔のページからローカルのファイルへは行けない）。
    pub(crate) fn may_navigate(&self, id: TabId, url: &url::Url) -> bool {
        self.tabs
            .get(&id)
            .and_then(|t| t.page.as_ref())
            .is_none_or(|p| kumo_js::may_reach(p.url(), url))
    }

    /// `with_js` と同じだが、変化の取り込みは呼び出し側がまとめて行う（再帰を避ける）。
    fn with_js_quiet<R>(&mut self, id: TabId, f: impl FnOnce(&mut JsRuntime) -> R) -> Option<R> {
        let now = self.now;
        let js = self.tabs.get_mut(&id)?.page.as_mut()?.js_mut()?;
        js.set_time(now);
        Some(f(js))
    }

    /// 取得スレッドから戻った結果（スクリプト・`fetch`）を JS に渡す。古いページへの結果は捨てる。
    pub fn sub_fetched(&mut self, done: SubFetched) {
        if self.tabs.get(&done.tab).is_none_or(|t| t.nav != done.nav) {
            return;
        }
        let id = done.tab;
        self.with_js(id, |js| match (done.kind, done.result) {
            (SubKind::Api(request), result) => js.fetch_done(request, result),
            (SubKind::Script(node), Ok(response)) if response.status == 200 => {
                let body = String::from_utf8_lossy(&response.body).into_owned();
                let _ = js.run_script(&body, &response.url, Some(node));
            }
            (SubKind::Script(_), Ok(response)) => {
                eprintln!(
                    "[js] {}: 状態 {}（実行しません）",
                    response.url, response.status
                );
            }
            (SubKind::Script(_), Err(message)) => {
                eprintln!("[js] スクリプトを取得できません: {message}")
            }
        });
    }

    /// 期限の来たタイマーを実行する。表示中のタブは毎回、背景のタブは 1 秒に 1 回まで。
    pub fn tick(&mut self) {
        let now = self.now;
        let active = self.active;
        let due: Vec<TabId> = self
            .tabs
            .iter()
            .filter(|(id, tab)| {
                tab.page.as_ref().is_some_and(|p| p.has_js())
                    && (Some(**id) == active
                        || now.saturating_sub(tab.last_tick) >= BACKGROUND_TICK)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in due {
            if let Some(tab) = self.tabs.get_mut(&id) {
                tab.last_tick = now;
            }
            self.with_js(id, |js| js.tick(now));
        }
    }

    /// 次にタイマーを動かすまでの時間。動かすものがなければ `None`。
    pub fn next_wake(&mut self) -> Option<Duration> {
        let now = self.now;
        let active = self.active;
        self.tabs
            .iter_mut()
            .filter_map(|(id, tab)| {
                let last = tab.last_tick;
                let js = tab.page.as_mut()?.js_mut()?;
                let wait = js.next_timer_in(now)?;
                Some(if Some(*id) == active {
                    wait
                } else {
                    wait.max(BACKGROUND_TICK.saturating_sub(now.saturating_sub(last)))
                })
            })
            .min()
    }
}

struct JsChanges {
    dirty: bool,
    navigation: Option<String>,
    scripts: Vec<NodeId>,
    fetches: Vec<kumo_js::FetchRequest>,
    errors: Vec<String>,
    console: Vec<String>,
}
