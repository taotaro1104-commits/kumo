//! MemoryGovernor：全体のメモリ予算と OS の圧迫を見て、背景のタブを段階的に降格する（設計書）。
//!
//! 段階は Live → Frozen（スクリプトと描画データを捨てる）→ Discarded（ページごと捨てる）。
//! 表示中のタブは対象にしない。ここは判断だけを行う純粋な関数で、実行は [`crate::Browser`] が行う。

use std::time::Duration;

use kumo_content::protocol::TabId;
use kumo_mem::Pressure;

/// タブの状態。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabState {
    #[default]
    Live,
    Frozen,
    Discarded,
}

impl TabState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Frozen => "frozen",
            Self::Discarded => "discarded",
        }
    }
}

/// 降格の方針。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
    /// 背景に回ってからこの時間で凍結する
    pub freeze_after: Duration,
    /// 凍結してからではなく、最後に見てからこの時間で破棄する
    pub discard_after: Duration,
    /// Browser と Content の合計の予算（バイト）。超えたら古い背景のタブから降格する
    pub budget: u64,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            freeze_after: Duration::from_secs(60),
            discard_after: Duration::from_secs(15 * 60),
            budget: 200 * 1024 * 1024,
        }
    }
}

/// 判断に使うタブの情報。
#[derive(Clone, Copy, Debug)]
pub struct TabInfo {
    pub id: TabId,
    pub state: TabState,
    /// 最後に表示してからの時間
    pub idle: Duration,
    pub active: bool,
    /// 読み込み中（降格しない）
    pub loading: bool,
    /// 失うと困る状態がある（入力中の欄、POST で開いたページ）。凍結まではよいが破棄しない
    pub protected: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Freeze(TabId),
    Discard(TabId),
}

/// いま取るべき降格を決める。`used` は Browser と Content の合計メモリ（わからなければ 0）。
///
/// - 時間：背景に回って `freeze_after` で凍結、`discard_after` で破棄。
/// - 予算超過または圧迫（中）：背景の Live を古い順にすべて凍結。凍結済みしかなければ、
///   古い順に1枚だけ破棄（測り直してから次を判断する）。
/// - 圧迫（危険）：背景のタブをすべて破棄する（保護されたものは凍結）。
pub fn plan(policy: &Policy, tabs: &[TabInfo], used: u64, pressure: Pressure) -> Vec<Action> {
    let mut background: Vec<&TabInfo> = tabs.iter().filter(|t| !t.active && !t.loading).collect();
    background.sort_by_key(|t| std::cmp::Reverse(t.idle)); // 古い（長く見ていない）順
    let over_budget = used > policy.budget;
    let squeezed = over_budget || pressure >= Pressure::Moderate;
    let mut actions = Vec::new();
    let mut discarded_for_budget = false;
    for tab in background {
        let can_discard = !tab.protected;
        let action = match tab.state {
            TabState::Discarded => None,
            _ if pressure == Pressure::Critical && can_discard => Some(Action::Discard(tab.id)),
            // 長く見ていないタブは、凍結を飛ばして破棄してよい（アプリが止まっていたときなど）
            TabState::Live if can_discard && tab.idle >= policy.discard_after => {
                Some(Action::Discard(tab.id))
            }
            TabState::Live if squeezed || tab.idle >= policy.freeze_after => {
                Some(Action::Freeze(tab.id))
            }
            TabState::Frozen if can_discard && tab.idle >= policy.discard_after => {
                Some(Action::Discard(tab.id))
            }
            TabState::Frozen if can_discard && squeezed && !discarded_for_budget => {
                discarded_for_budget = true;
                Some(Action::Discard(tab.id))
            }
            _ => None,
        };
        actions.extend(action);
    }
    // 凍結を先にして、同じ周期で凍結と破棄が混ざっても Content が順に処理できるようにする
    actions.sort_by_key(|a| matches!(a, Action::Discard(_)));
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: TabId, state: TabState, idle_s: u64) -> TabInfo {
        TabInfo {
            id,
            state,
            idle: Duration::from_secs(idle_s),
            active: false,
            loading: false,
            protected: false,
        }
    }

    fn run(tabs: &[TabInfo], used_mb: u64, pressure: Pressure) -> Vec<Action> {
        plan(&Policy::default(), tabs, used_mb * 1024 * 1024, pressure)
    }

    #[test]
    fn recent_background_tabs_are_left_alone() {
        let tabs = [tab(1, TabState::Live, 10)];
        assert!(run(&tabs, 50, Pressure::None).is_empty());
    }

    #[test]
    fn idle_tabs_freeze_then_discard_over_time() {
        assert_eq!(
            run(&[tab(1, TabState::Live, 61)], 50, Pressure::None),
            [Action::Freeze(1)]
        );
        assert_eq!(
            run(&[tab(1, TabState::Frozen, 16 * 60)], 50, Pressure::None),
            [Action::Discard(1)]
        );
        assert!(run(&[tab(1, TabState::Frozen, 120)], 50, Pressure::None).is_empty());
    }

    #[test]
    fn a_live_tab_unseen_for_very_long_is_discarded_directly() {
        assert_eq!(
            run(&[tab(1, TabState::Live, 16 * 60)], 50, Pressure::None),
            [Action::Discard(1)]
        );
    }

    #[test]
    fn the_active_and_loading_tabs_are_never_touched() {
        let mut active = tab(1, TabState::Live, 9999);
        active.active = true;
        let mut loading = tab(2, TabState::Live, 9999);
        loading.loading = true;
        assert!(run(&[active, loading], 900, Pressure::Critical).is_empty());
    }

    #[test]
    fn over_budget_freezes_every_live_background_tab_oldest_first() {
        let tabs = [
            tab(1, TabState::Live, 5),
            tab(2, TabState::Live, 30),
            tab(3, TabState::Live, 20),
        ];
        assert_eq!(
            run(&tabs, 300, Pressure::None),
            [Action::Freeze(2), Action::Freeze(3), Action::Freeze(1)]
        );
    }

    #[test]
    fn over_budget_with_only_frozen_tabs_discards_one_oldest() {
        let tabs = [tab(1, TabState::Frozen, 100), tab(2, TabState::Frozen, 400)];
        assert_eq!(run(&tabs, 300, Pressure::None), [Action::Discard(2)]);
    }

    #[test]
    fn moderate_pressure_acts_like_over_budget() {
        let tabs = [tab(1, TabState::Live, 5)];
        assert_eq!(run(&tabs, 10, Pressure::Moderate), [Action::Freeze(1)]);
    }

    #[test]
    fn critical_pressure_discards_all_background_tabs_but_spares_protected() {
        let mut typing = tab(3, TabState::Live, 5);
        typing.protected = true;
        let tabs = [
            tab(1, TabState::Live, 5),
            tab(2, TabState::Frozen, 5),
            typing,
        ];
        assert_eq!(
            run(&tabs, 10, Pressure::Critical),
            [Action::Freeze(3), Action::Discard(1), Action::Discard(2)]
        );
    }

    #[test]
    fn protected_tabs_are_frozen_but_not_discarded_by_time() {
        let mut t = tab(1, TabState::Frozen, 99999);
        t.protected = true;
        assert!(run(&[t], 50, Pressure::None).is_empty());
    }

    #[test]
    fn discarded_tabs_need_nothing() {
        assert!(
            run(
                &[tab(1, TabState::Discarded, 99999)],
                900,
                Pressure::Critical
            )
            .is_empty()
        );
    }
}
