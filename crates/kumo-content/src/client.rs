//! Browser 側から Content を動かす口。子プロセス（既定）でも、同じプロセスのスレッド（テスト・`--in-process`）でも
//! 同じ API で使う。Content からの通知は受信箱に溜まり、届くたびに `notify` でイベントループを起こす。

use std::io::{self, BufReader};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use kumo_net::process::Fetcher;
use kumo_net::protocol::{read_message, write_message};

use crate::engine::ContentEngine;
use crate::host::{self, Event};
use crate::protocol::{FromContent, ToContent};

/// 通知が届いたときに呼ぶ関数（ウィンドウのイベントループを起こす）。
pub type Notify = Arc<dyn Fn() + Send + Sync>;
type NotifySlot = Arc<Mutex<Option<Notify>>>;

enum Link {
    Thread(Sender<Event>),
    Process(Mutex<ChildStdin>),
}

pub struct ContentClient {
    link: Link,
    inbox: Receiver<FromContent>,
    notify: NotifySlot,
    child: Option<Mutex<Child>>,
    worker: Option<JoinHandle<()>>,
    /// Content が終了した（プロセスが落ちた、またはスレッドが終わった）
    closed: Arc<AtomicBool>,
}

fn deliver(inbox: &Sender<FromContent>, notify: &NotifySlot, message: FromContent) -> bool {
    if inbox.send(message).is_err() {
        return false;
    }
    if let Some(notify) = notify.lock().ok().and_then(|n| n.clone()) {
        notify();
    }
    true
}

/// 終了を記録し、イベントループを起こして気づかせる。
fn mark_closed(closed: &AtomicBool, notify: &NotifySlot) {
    closed.store(true, Ordering::Release);
    if let Some(notify) = notify.lock().ok().and_then(|n| n.clone()) {
        notify();
    }
}

impl ContentClient {
    /// 同じプロセスのスレッドで Content を動かす。
    pub fn spawn_thread(fetcher: Arc<dyn Fetcher>) -> Self {
        let (events_tx, events_rx) = mpsc::channel();
        let (inbox_tx, inbox) = mpsc::channel();
        let notify: NotifySlot = Arc::default();
        let closed = Arc::new(AtomicBool::new(false));
        let worker = {
            let (events_tx, notify, closed) = (events_tx.clone(), notify.clone(), closed.clone());
            std::thread::spawn(move || {
                let engine = ContentEngine::new(fetcher, || 0);
                host::run(engine, &events_rx, &events_tx, |m| {
                    deliver(&inbox_tx, &notify, m)
                });
                drop(inbox_tx);
                mark_closed(&closed, &notify);
            })
        };
        Self {
            link: Link::Thread(events_tx),
            inbox,
            notify,
            child: None,
            worker: Some(worker),
            closed,
        }
    }

    /// `command`（Content プロセスとして動く実行ファイルと引数）を起動する。
    pub fn spawn_process(mut command: Command) -> io::Result<Self> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("stdin がない"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout がない"))?;
        let (inbox_tx, inbox) = mpsc::channel();
        let notify: NotifySlot = Arc::default();
        let closed = Arc::new(AtomicBool::new(false));
        let worker = {
            let (notify, closed) = (notify.clone(), closed.clone());
            std::thread::spawn(move || {
                let mut stdout = BufReader::new(stdout);
                while let Ok(Some(message)) = read_message::<FromContent>(&mut stdout) {
                    if !deliver(&inbox_tx, &notify, message) {
                        return;
                    }
                }
                drop(inbox_tx);
                mark_closed(&closed, &notify);
            })
        };
        Ok(Self {
            link: Link::Process(Mutex::new(stdin)),
            inbox,
            notify,
            child: Some(Mutex::new(child)),
            worker: Some(worker),
            closed,
        })
    }

    /// 通知が届いたときに呼ぶ関数を設定する。
    pub fn set_notify(&self, notify: Notify) {
        if let Ok(mut slot) = self.notify.lock() {
            *slot = Some(notify);
        }
    }

    /// 要求を送る。相手が終了していたら `false`。
    pub fn send(&self, message: ToContent) -> bool {
        let sent = match &self.link {
            Link::Thread(tx) => tx.send(Event::Message(message)).is_ok(),
            Link::Process(stdin) => stdin
                .lock()
                .is_ok_and(|mut pipe| write_message(&mut *pipe, &message).is_ok()),
        };
        if !sent {
            self.closed.store(true, Ordering::Release);
        }
        sent
    }

    /// Content が終了したか（届いている通知は先に取り出せる）。
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// 届いている通知を1つ取り出す（待たない）。
    pub fn try_recv(&self) -> Option<FromContent> {
        self.inbox.try_recv().ok()
    }

    /// 通知を待つ。`Err(true)` は切断、`Err(false)` は時間切れ。
    pub fn recv_timeout(&self, timeout: Duration) -> Result<FromContent, bool> {
        self.inbox
            .recv_timeout(timeout)
            .map_err(|e| e == RecvTimeoutError::Disconnected)
    }

    /// 子プロセスの ID（スレッドで動かしているなら `None`）。
    pub fn process_id(&self) -> Option<u32> {
        self.child.as_ref()?.lock().ok().map(|c| c.id())
    }
}

impl Drop for ContentClient {
    fn drop(&mut self) {
        self.send(ToContent::Shutdown);
        if let Some(child) = &self.child
            && let Ok(mut child) = child.lock()
        {
            // 正常終了を少し待ち、だめなら強制終了する
            for _ in 0..100 {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
