//! 取得の待ち行列。取得スレッドは1本なので、順番を工夫して表示中のタブを待たせない。
//!
//! - 表示中のタブの読み込みは先頭に入れる（背景タブの読み込みの後ろで待たせない）。
//! - 同じタブの新しい読み込みが来たら、まだ始まっていない古いものは捨てる（結果は使われないため）。
//! - 閉じたタブの読み込みは捨て、表示に切り替わったタブの読み込みは先頭へ移す。

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

use crate::engine::FetchJob;
use crate::protocol::TabId;

#[derive(Default)]
struct State {
    queue: VecDeque<FetchJob>,
    closed: bool,
}

#[derive(Default)]
pub struct JobQueue {
    state: Mutex<State>,
    ready: Condvar,
}

impl JobQueue {
    /// 依頼を入れる。`urgent`（表示中のタブ）なら先頭へ。
    pub fn push(&self, job: FetchJob, urgent: bool) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.queue.retain(|queued| queued.tab != job.tab);
        if urgent {
            state.queue.push_front(job);
        } else {
            state.queue.push_back(job);
        }
        self.ready.notify_one();
    }

    /// 表示に切り替わったタブの依頼を先頭へ移す。
    pub fn promote(&self, tab: TabId) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if let Some(at) = state.queue.iter().position(|job| job.tab == tab)
            && let Some(job) = state.queue.remove(at)
        {
            state.queue.push_front(job);
        }
    }

    /// 閉じたタブの依頼を捨てる。
    pub fn cancel(&self, tab: TabId) {
        if let Ok(mut state) = self.state.lock() {
            state.queue.retain(|job| job.tab != tab);
        }
    }

    /// 次の依頼を待って取り出す。閉じられたら `None`。
    pub fn pop(&self) -> Option<FetchJob> {
        let mut state = self.state.lock().ok()?;
        loop {
            if let Some(job) = state.queue.pop_front() {
                return Some(job);
            }
            if state.closed {
                return None;
            }
            state = self.ready.wait(state).ok()?;
        }
    }

    /// 取得スレッドを終わらせる（残りの依頼は捨てる）。
    pub fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            state.queue.clear();
        }
        self.ready.notify_all();
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.state.lock().map_or(0, |s| s.queue.len())
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use url::Url;

    fn job(tab: TabId, nav: u64) -> FetchJob {
        FetchJob {
            tab,
            nav,
            url: Url::parse("https://example.com/").unwrap(),
            post_body: None,
        }
    }

    fn order(queue: &JobQueue) -> Vec<(TabId, u64)> {
        let mut out = Vec::new();
        while !queue.is_empty() {
            let j = queue.pop().unwrap();
            out.push((j.tab, j.nav));
        }
        out
    }

    #[test]
    fn active_tab_jumps_ahead_and_old_loads_are_dropped() {
        let queue = JobQueue::default();
        queue.push(job(1, 1), false);
        queue.push(job(2, 2), false);
        queue.push(job(3, 3), false);
        queue.push(job(2, 4), false); // タブ 2 の新しい読み込みで古い方は捨てる
        queue.push(job(9, 5), true); // 表示中のタブは先頭
        assert_eq!(order(&queue), vec![(9, 5), (1, 1), (3, 3), (2, 4)]);
    }

    #[test]
    fn shown_tabs_are_promoted_and_closed_tabs_cancelled() {
        let queue = JobQueue::default();
        for tab in 1..=4 {
            queue.push(job(tab, tab), false);
        }
        queue.promote(3);
        queue.cancel(2);
        queue.promote(7); // 入っていないタブは何もしない
        assert_eq!(order(&queue), vec![(3, 3), (1, 1), (4, 4)]);
    }

    #[test]
    fn pop_waits_until_a_job_arrives_or_the_queue_closes() {
        let queue = Arc::new(JobQueue::default());
        let worker = {
            let queue = queue.clone();
            std::thread::spawn(move || {
                let first = queue.pop().map(|j| j.nav);
                let second = queue.pop().map(|j| j.nav);
                (first, second)
            })
        };
        std::thread::sleep(std::time::Duration::from_millis(50));
        queue.push(job(1, 42), false);
        std::thread::sleep(std::time::Duration::from_millis(50));
        queue.close();
        assert_eq!(worker.join().unwrap(), (Some(42), None));
    }
}
