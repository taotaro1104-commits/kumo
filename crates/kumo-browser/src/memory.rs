//! メモリ管理の実行：定期的に使用量を測り、[`crate::governor`] の判断をタブに適用する。
//! 表示するときの復帰（凍結の解除・破棄したページの読み込み直し）もここ。

use std::time::{Duration, Instant};

use kumo_content::protocol::{FromContent, ToContent};
use kumo_mem::Pressure;

use crate::about_memory::{MemoryReport, TabRow};
use crate::browser::{Browser, Redraw};
use crate::governor::{Action, Policy, TabInfo, TabState, plan};

/// 使用量を測って判断する間隔。
const MAINTAIN_INTERVAL: Duration = Duration::from_secs(2);

/// Browser が持つメモリ管理の状態。
#[derive(Default)]
pub struct MemoryState {
    pub policy: Policy,
    /// 最後に知らせてもらった Content のメモリ（バイト）
    pub content_bytes: u64,
    last_check: Option<Instant>,
    /// 降格の累計（about:memory 用）
    pub frozen_total: u32,
    pub discarded_total: u32,
}

impl Browser {
    pub fn set_memory_policy(&mut self, policy: Policy) {
        self.memory.policy = policy;
    }

    pub fn memory_policy(&self) -> Policy {
        self.memory.policy
    }

    /// 事務：ウィンドウのイベントループから定期的に呼ぶ（間隔に満たなければ何もしない）。
    /// Content のメモリを問い合わせ、最後に知っている値と OS の圧迫から降格を決める。
    pub fn maintain_memory(&mut self, now: Instant) -> Redraw {
        let due = self
            .memory
            .last_check
            .is_none_or(|t| now.duration_since(t) >= MAINTAIN_INTERVAL);
        if !due {
            return Redraw::default();
        }
        self.memory.last_check = Some(now);
        let id = self.request_id();
        self.content.send(ToContent::Stats { id });
        let own = kumo_mem::process_memory().map_or(0, |m| m.primary as u64);
        self.govern(
            own + self.memory.content_bytes,
            kumo_mem::memory_pressure(),
            now,
        )
    }

    /// `used`（Browser と Content の合計）と `pressure` から降格を決めて適用する。
    pub fn govern(&mut self, used: u64, pressure: Pressure, now: Instant) -> Redraw {
        let infos: Vec<TabInfo> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| TabInfo {
                id: t.id,
                state: t.state,
                idle: now.saturating_duration_since(t.last_active),
                active: i == self.active,
                loading: t.is_loading(),
                protected: t.posted || t.field_focused,
            })
            .collect();
        let actions = plan(&self.memory.policy, &infos, used, pressure);
        let changed = !actions.is_empty();
        for action in actions {
            self.apply(action);
        }
        if changed {
            self.invalidate_chrome()
        } else {
            Redraw::default()
        }
    }

    fn apply(&mut self, action: Action) {
        let (id, state) = match action {
            Action::Freeze(id) => (id, TabState::Frozen),
            Action::Discard(id) => (id, TabState::Discarded),
        };
        let Some(index) = self.tab_index(id) else {
            return;
        };
        self.tabs[index].state = state;
        match action {
            Action::Freeze(id) => {
                self.memory.frozen_total += 1;
                self.content.send(ToContent::Freeze { tab: id });
            }
            Action::Discard(id) => {
                self.memory.discarded_total += 1;
                self.content.send(ToContent::Discard { tab: id });
            }
        }
    }

    /// タブを表示するとき：背景の時間を数え直し、凍結は解除、破棄は読み込み直す。
    pub(crate) fn wake(&mut self, index: usize, now: Instant) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        tab.last_active = now;
        let state = std::mem::replace(&mut tab.state, TabState::Live);
        if state != TabState::Discarded {
            return; // 凍結は Content が表示するときに再レイアウトする
        }
        let Some(url) = tab.url().cloned() else {
            return;
        };
        tab.restore_scroll = Some(tab.scroll_y);
        self.start_load(index, url, false, None);
    }

    /// 表示を離れるタブの時刻を記録する（ここから背景の時間を数える）。
    pub(crate) fn leave(&mut self, index: usize, now: Instant) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.last_active = now;
        }
    }

    /// Content からの統計を取り込む。
    pub(crate) fn memory_stats(&mut self, message: &FromContent) {
        if let FromContent::Stats { stats, .. } = message {
            self.memory.content_bytes = stats.private_bytes;
        }
    }

    /// 読み込み後にスクロール位置を戻す（破棄から復帰したとき）。
    pub(crate) fn restore_scroll(&mut self, index: usize) {
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        if let Some(y) = tab.restore_scroll.take().filter(|y| *y > 0.0) {
            let message = ToContent::ScrollBy { tab: tab.id, dy: y };
            self.content.send(message);
        }
    }

    /// `about:memory` に出す数値。
    pub(crate) fn memory_report(&self) -> MemoryReport {
        let now = Instant::now();
        MemoryReport {
            browser_bytes: kumo_mem::process_memory().map_or(0, |m| m.primary as u64),
            content_bytes: self.memory.content_bytes,
            policy: self.memory.policy,
            pressure: kumo_mem::memory_pressure(),
            tabs: self
                .tabs
                .iter()
                .enumerate()
                .map(|(i, t)| TabRow {
                    title: t.title_in(self.lang),
                    url: t.url().map(ToString::to_string).unwrap_or_default(),
                    state: t.state,
                    active: i == self.active,
                    idle_secs: now.saturating_duration_since(t.last_active).as_secs(),
                })
                .collect(),
            frozen_total: self.memory.frozen_total,
            discarded_total: self.memory.discarded_total,
        }
    }
}
