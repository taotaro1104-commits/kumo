//! Content のイベントループ。要求と取得結果を1つのキューで受け、溜まった分をまとめて処理してから1回描く。
//! 取得は別スレッドで動かすので、読み込み中も他のタブの描画や入力は止まらない。

use std::io::{self, BufWriter};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use kumo_net::process::Fetcher;
use kumo_net::protocol::{read_message, write_message};

use crate::engine::{ContentEngine, Fetched};
use crate::jobs::JobQueue;
use crate::protocol::{FromContent, ToContent};

/// 同時に取得・パースするページ数（文書内の CSS・画像は Network 側で並行に取得する）。
/// 2 以上にすると 10 タブで約 12MB 増える（スレッドごとのヒープに DOM が分かれるため）ので、
/// 省メモリを優先して 1 にする。読み込みは順番になるが、UI と表示中のタブは止まらない。
const FETCH_WORKERS: usize = 1;

pub enum Event {
    Message(ToContent),
    Fetched(Box<Fetched>),
    /// Browser との接続が切れた
    Closed,
}

/// `events` が閉じるか Shutdown を受けるまで動く。`send` が `false` を返したら（相手が閉じた）終わる。
pub fn run(
    mut engine: ContentEngine,
    events: &Receiver<Event>,
    events_tx: &Sender<Event>,
    mut send: impl FnMut(FromContent) -> bool,
) {
    let jobs = FetchWorkers::spawn(engine.fetcher(), events_tx);
    // 表示中のタブ（その読み込みを先に回す）
    let mut active = None;
    while let Ok(first) = events.recv() {
        let mut next = Some(first);
        // 溜まっている分を全部処理してから描く（連続した入力を1回の再レイアウトにまとめる）
        while let Some(event) = next {
            match event {
                Event::Message(ToContent::Shutdown) | Event::Closed => return,
                Event::Message(message) => {
                    match &message {
                        ToContent::Show { tab } => {
                            active = Some(*tab);
                            jobs.queue.promote(*tab);
                        }
                        ToContent::Close { tab } => jobs.queue.cancel(*tab),
                        _ => {}
                    }
                    engine.handle(message);
                }
                Event::Fetched(fetched) => engine.fetched(*fetched),
            }
            for job in engine.take_jobs() {
                let urgent = active == Some(job.tab);
                jobs.queue.push(job, urgent);
            }
            next = events.try_recv().ok();
        }
        engine.render();
        for message in engine.take_output() {
            if !send(message) {
                return;
            }
        }
    }
}

/// 取得とパースを受け持つ常駐スレッド。1件ごとにスレッドを作ると、終わったスレッドのヒープの
/// 空きページが OS へ返らずに残るため常駐させる。drop で待ち行列を閉じてスレッドを終わらせる。
struct FetchWorkers {
    queue: Arc<JobQueue>,
}

impl FetchWorkers {
    fn spawn(fetcher: Arc<dyn Fetcher>, events: &Sender<Event>) -> Self {
        let queue = Arc::new(JobQueue::default());
        for _ in 0..FETCH_WORKERS {
            let (queue, fetcher, events) = (queue.clone(), fetcher.clone(), events.clone());
            std::thread::spawn(move || {
                while let Some(job) = queue.pop() {
                    let fetched = job.run(&*fetcher);
                    if events.send(Event::Fetched(Box::new(fetched))).is_err() {
                        return;
                    }
                    // パースの一時データで空いたこのスレッドのページを OS へ返す
                    kumo_mem::release_free_memory();
                }
            });
        }
        Self { queue }
    }
}

impl Drop for FetchWorkers {
    fn drop(&mut self) {
        self.queue.close();
    }
}

/// 標準入出力で Browser とやりとりする（Content プロセスのエントリポイント）。
pub fn serve_stdio(fetcher: Arc<dyn Fetcher>, private_bytes: fn() -> u64) -> io::Result<()> {
    let (tx, rx) = mpsc::channel();
    let reader_tx = tx.clone();
    std::thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        loop {
            match read_message::<ToContent>(&mut stdin) {
                Ok(Some(message)) => {
                    if reader_tx.send(Event::Message(message)).is_err() {
                        return;
                    }
                }
                Ok(None) | Err(_) => {
                    let _ = reader_tx.send(Event::Closed);
                    return;
                }
            }
        }
    });
    let mut out = BufWriter::new(io::stdout().lock());
    let engine = ContentEngine::new(fetcher, private_bytes);
    run(engine, &rx, &tx, |message| {
        write_message(&mut out, &message).is_ok()
    });
    Ok(())
}
