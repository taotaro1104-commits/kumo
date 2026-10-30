//! ウィンドウの外から届く文字入力（Android のソフトキーボード）。
//! Java の `InputConnection` は UI スレッドで呼ばれるので、待ち行列に積んでイベントループを起こし、
//! ループの側で取り出して処理する。

use std::collections::VecDeque;
use std::sync::Mutex;

/// ソフトキーボードからの入力。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeInput {
    /// 変換中の文字（空なら変換の終わり）
    Preedit(String),
    /// 確定した文字
    Commit(String),
    Backspace,
    Enter,
}

static QUEUE: Mutex<VecDeque<ImeInput>> = Mutex::new(VecDeque::new());
type Waker = Box<dyn Fn() + Send + Sync>;

static WAKER: Mutex<Option<Waker>> = Mutex::new(None);

/// 入力を積み、イベントループを起こす。どのスレッドから呼んでもよい。
pub fn push(input: ImeInput) {
    if let Ok(mut queue) = QUEUE.lock() {
        queue.push_back(input);
    }
    if let Ok(waker) = WAKER.lock()
        && let Some(wake) = waker.as_ref()
    {
        wake();
    }
}

/// 積まれた入力を古い順に全部取り出す。
pub(crate) fn drain() -> Vec<ImeInput> {
    QUEUE
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default()
}

/// 入力が積まれたときにイベントループを起こす関数（アクティビティが作り直されたら差し替える）。
pub(crate) fn set_waker(wake: Waker) {
    if let Ok(mut waker) = WAKER.lock() {
        *waker = Some(wake);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // 待ち行列は全体で1つなので、1つのテストにまとめて順に確かめる
    #[test]
    fn queue_keeps_order_and_wakes_the_loop() {
        static WOKEN: AtomicUsize = AtomicUsize::new(0);
        drain();
        set_waker(Box::new(|| {
            WOKEN.fetch_add(1, Ordering::SeqCst);
        }));
        push(ImeInput::Preedit("かん".into()));
        push(ImeInput::Commit("漢".into()));
        push(ImeInput::Backspace);
        assert_eq!(WOKEN.load(Ordering::SeqCst), 3);
        assert_eq!(
            drain(),
            [
                ImeInput::Preedit("かん".into()),
                ImeInput::Commit("漢".into()),
                ImeInput::Backspace
            ]
        );
        assert!(drain().is_empty(), "取り出した分は残らない");
    }
}
