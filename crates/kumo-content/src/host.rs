//! Content のイベントループ。要求と取得結果を1つのキューで受け、溜まった分をまとめて処理してから1回描く。
//! 取得は別スレッドで動かすので、読み込み中も他のタブの描画や入力は止まらない。

use std::io::{self, BufWriter};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Instant;

use kumo_net::process::Fetcher;
use kumo_net::protocol::{read_message, write_message};

use crate::engine::ContentEngine;
use crate::jobs::JobQueue;
use crate::protocol::{FromContent, ToContent};
use crate::work::Done;

/// 同時に取得・パースするページ数（文書内の CSS・画像は Network 側で並行に取得する）。
/// 2 以上にすると 10 タブで約 12MB 増える（スレッドごとのヒープに DOM が分かれるため）ので、
/// 省メモリを優先して 1 にする。読み込みは順番になるが、UI と表示中のタブは止まらない。
const FETCH_WORKERS: usize = 1;

pub enum Event {
    Message(ToContent),
    /// 取得スレッドの仕事が終わった
    Done(Box<Done>),
    /// Browser との接続が切れた
    Closed,
}

/// 溜まった取得の依頼を取得スレッドへ回す。表示中のタブの依頼は先頭に入れる。
fn flush_jobs(engine: &mut ContentEngine, workers: &FetchWorkers, active: Option<u64>) {
    for job in engine.take_jobs() {
        let urgent = active == Some(job.tab());
        workers.queue.push(job, urgent);
    }
}

/// エンジンのスレッドのスタック。Boa の構文解析と実行は深く再帰する（大きなスクリプトで 1〜2MB の
/// 既定のスタックを溢れさせる）ので大きく取る。使った分だけがコミットされる。
pub const ENGINE_STACK_BYTES: usize = 256 * 1024 * 1024;
/// 取得スレッド（HTML の解析）のスタック。
const FETCH_STACK_BYTES: usize = 32 * 1024 * 1024;

/// 名前とスタックの大きさを指定してスレッドを起こす。
pub(crate) fn spawn_thread(
    name: &str,
    stack: usize,
    f: impl FnOnce() + Send + 'static,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name(name.to_owned())
        .stack_size(stack)
        .spawn(f)
        .expect("スレッドを起こせません")
}

/// `events` が閉じるか Shutdown を受けるまで動く。`send` が `false` を返したら（相手が閉じた）終わる。
/// 要求・取得結果・タイマーの期限のどれかが来るたびに、溜まった分をまとめて処理してから1回描く。
pub fn run(
    mut engine: ContentEngine,
    events: &Receiver<Event>,
    events_tx: &Sender<Event>,
    mut send: impl FnMut(FromContent) -> bool,
) {
    let workers = FetchWorkers::spawn(engine.fetcher(), events_tx);
    let started = Instant::now();
    // 表示中のタブ（その読み込みを先に回す）
    let mut active = None;
    loop {
        engine.set_time(started.elapsed());
        let first = match engine.next_wake() {
            Some(wait) => match events.recv_timeout(wait) {
                Ok(event) => Some(event),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            },
            None => match events.recv() {
                Ok(event) => Some(event),
                Err(_) => return,
            },
        };
        engine.set_time(started.elapsed());
        // 溜まっている分を全部処理してから描く（連続した入力を1回の再レイアウトにまとめる）
        let mut next = first;
        while let Some(event) = next {
            match event {
                Event::Message(ToContent::Shutdown) | Event::Closed => return,
                Event::Message(message) => {
                    match &message {
                        ToContent::Show { tab } => {
                            active = Some(*tab);
                            workers.queue.promote(*tab);
                        }
                        ToContent::Close { tab } => workers.queue.cancel(*tab),
                        _ => {}
                    }
                    engine.handle(message);
                }
                Event::Done(done) => engine.done(*done),
            }
            flush_jobs(&mut engine, &workers, active);
            next = events.try_recv().ok();
        }
        engine.tick();
        flush_jobs(&mut engine, &workers, active);
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
            spawn_thread("kumo-fetch", FETCH_STACK_BYTES, move || {
                while let Some(job) = queue.pop() {
                    if let Some(done) = job.run(&*fetcher)
                        && events.send(Event::Done(Box::new(done))).is_err()
                    {
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
    // エンジンは大きなスタックのスレッドで動かし、このスレッドは終わるのを待つ
    let engine_thread = spawn_thread("kumo-content", ENGINE_STACK_BYTES, move || {
        let mut out = BufWriter::new(io::stdout().lock());
        let engine = ContentEngine::new(fetcher, private_bytes);
        run(engine, &rx, &tx, |message| {
            write_message(&mut out, &message).is_ok()
        });
    });
    engine_thread
        .join()
        .map_err(|_| io::Error::other("Content のエンジンが異常終了しました"))
}
