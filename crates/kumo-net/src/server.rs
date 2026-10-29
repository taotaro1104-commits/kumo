//! Network プロセスのサーバ側。接続ごとにワーカーで並行に取得し、大きな本文は共有メモリのリングで渡す。

use std::collections::HashMap;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use kumo_ipc::{RingAllocator, SharedMemory};

use crate::protocol::{
    DownloadEvent, FetchRequest, FetchResponse, FromNetwork, ToNetwork, read_message, write_message,
};
use crate::service::NetworkService;

/// 接続ごとの同時取得数。
const WORKERS: usize = 4;
/// 接続ごとの本文リングの大きさ。
pub const RING_CAPACITY: usize = 4 << 20;
/// これ以上の本文をリングで渡す（小さいものはメッセージに入れたほうが安い）。
pub const SHARED_BODY_MIN: usize = 16 << 10;

/// 本文リング（書き手側）。共有メモリと領域管理の組。
struct BodyRing {
    memory: SharedMemory,
    regions: RingAllocator,
}

impl BodyRing {
    fn create() -> io::Result<Self> {
        let path = kumo_ipc::unique_path("net-ring");
        Ok(Self {
            memory: SharedMemory::create(&path, RING_CAPACITY)?,
            regions: RingAllocator::new(RING_CAPACITY),
        })
    }

    fn hello(&self) -> FromNetwork {
        FromNetwork::Hello {
            ring_path: self.memory.path().to_string_lossy().into_owned(),
            ring_capacity: self.memory.len() as u64,
        }
    }

    /// 本文を書いて位置を返す。空きがなければ `None`（呼び出し側はメッセージで送る）。
    fn put(&mut self, body: &[u8]) -> Option<u64> {
        let pos = self.regions.alloc(body.len())?;
        let offset = self.regions.offset(pos);
        if self.memory.write_at(offset, body).is_err() {
            self.regions.release(pos);
            return None;
        }
        Some(pos)
    }
}

type SharedRing = Option<Arc<Mutex<BodyRing>>>;
/// 実行中のダウンロードの中止フラグ（ID ごと）。
type Downloads = Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>;

/// 1つの接続を処理する。`input` が閉じるか Shutdown を受けたら戻る。
/// `shared_ring` なら最初に本文リングを作って [`FromNetwork::Hello`] を送る。
/// `private_bytes` はこのプロセスのメモリ使用量（統計用）を返す関数。
pub fn serve(
    service: Arc<NetworkService>,
    mut input: impl Read,
    output: impl Write + Send + 'static,
    private_bytes: fn() -> u64,
    shared_ring: bool,
) -> io::Result<()> {
    let output = Arc::new(Mutex::new(BufWriter::new(output)));
    let ring = if shared_ring {
        create_ring(&output)?
    } else {
        None
    };
    let downloads = Downloads::default();
    let (tx, rx) = mpsc::channel::<FetchRequest>();
    let rx = Arc::new(Mutex::new(rx));
    let workers: Vec<JoinHandle<()>> = (0..WORKERS)
        .map(|_| {
            let (rx, service, output, ring) =
                (rx.clone(), service.clone(), output.clone(), ring.clone());
            std::thread::spawn(move || worker(&rx, &service, &output, ring.as_deref()))
        })
        .collect();
    while let Some(message) = read_message::<ToNetwork>(&mut input)? {
        match message {
            ToNetwork::Fetch(request) => {
                if tx.send(request).is_err() {
                    break;
                }
            }
            ToNetwork::Stats { id } => {
                let mut stats = service.stats();
                stats.private_bytes = private_bytes();
                send(&output, &FromNetwork::Stats { id, stats })?;
            }
            ToNetwork::Download {
                id,
                url,
                dir,
                source_url,
            } => start_download(&service, &output, &downloads, id, url, dir, source_url),
            ToNetwork::CancelDownload { id } => {
                if let Ok(map) = downloads.lock()
                    && let Some(flag) = map.get(&id)
                {
                    flag.store(true, Ordering::Relaxed);
                }
            }
            ToNetwork::Release { pos } => {
                if let Some(ring) = &ring
                    && let Ok(mut ring) = ring.lock()
                {
                    ring.regions.release(pos);
                }
            }
            ToNetwork::Shutdown => break,
        }
    }
    drop(tx);
    // 接続が閉じたら、続けても受け取る相手がいないダウンロードは止める
    if let Ok(map) = downloads.lock() {
        map.values()
            .for_each(|flag| flag.store(true, Ordering::Relaxed));
    }
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}

/// ダウンロードを専用のスレッドで始める（取得用のワーカーを長時間ふさがない）。
fn start_download(
    service: &Arc<NetworkService>,
    output: &Arc<Mutex<BufWriter<impl Write + Send + 'static>>>,
    downloads: &Downloads,
    id: u64,
    url: String,
    dir: String,
    source_url: Option<String>,
) {
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = downloads.lock() {
        map.insert(id, cancel.clone());
    }
    let (service, output, downloads) = (service.clone(), output.clone(), downloads.clone());
    std::thread::spawn(move || {
        let mut report = |event: DownloadEvent| {
            let _ = send(&output, &FromNetwork::Download(event));
        };
        service.download(
            id,
            &url,
            Path::new(&dir),
            source_url.as_deref(),
            &cancel,
            &mut report,
        );
        if let Ok(mut map) = downloads.lock() {
            map.remove(&id);
        }
    });
}

/// リングを作って Hello を送る。共有メモリが作れない環境ではリングなしで続ける。
fn create_ring(output: &Mutex<impl Write>) -> io::Result<SharedRing> {
    match BodyRing::create() {
        Ok(ring) => {
            send(output, &ring.hello())?;
            Ok(Some(Arc::new(Mutex::new(ring))))
        }
        Err(e) => {
            eprintln!("[network] 本文リングなしで続けます: {e}");
            Ok(None)
        }
    }
}

fn worker(
    rx: &Mutex<Receiver<FetchRequest>>,
    service: &NetworkService,
    output: &Mutex<impl Write>,
    ring: Option<&Mutex<BodyRing>>,
) {
    loop {
        let request = match rx.lock() {
            Ok(guard) => guard.recv(),
            Err(_) => return,
        };
        let Ok(request) = request else { return };
        let id = request.id;
        let message = match service.fetch(&request) {
            Ok(response) => share_body(ring, response),
            Err(error) => FromNetwork::Failed { id, error },
        };
        let sent = send(output, &message);
        drop(message);
        // 大きな本文の確保・解放で空いたこのスレッドのページを OS へ返す
        kumo_mem::release_free_memory();
        if sent.is_err() {
            return; // クライアントが閉じた
        }
    }
}

/// 大きな本文はリングに置き、応答からは外す。リングが満杯ならそのまま送る。
fn share_body(ring: Option<&Mutex<BodyRing>>, response: FetchResponse) -> FromNetwork {
    let len = response.body.len();
    if len < SHARED_BODY_MIN {
        return FromNetwork::Response(response);
    }
    let pos = ring
        .and_then(|ring| ring.lock().ok())
        .and_then(|mut ring| ring.put(&response.body));
    match pos {
        Some(pos) => FromNetwork::SharedResponse {
            response: FetchResponse {
                body: Vec::new(),
                ..response
            },
            pos,
            len: len as u64,
        },
        None => FromNetwork::Response(response),
    }
}

fn send(output: &Mutex<impl Write>, message: &FromNetwork) -> io::Result<()> {
    let mut out = output
        .lock()
        .map_err(|_| io::Error::other("出力ロックが汚染された"))?;
    write_message(&mut *out, message)
}

/// ローカルソケット `name` で待ち受ける（子プロセスのエントリポイント）。
/// 待ち受けを始めたら標準出力に `ready` を書く。最初の接続（起動した親）が閉じたら戻る。
/// 後から来た接続（Content プロセスなど）は別スレッドで同じ [`NetworkService`] を共有して処理する。
pub fn serve_listener(
    service: NetworkService,
    name: &str,
    private_bytes: fn() -> u64,
) -> io::Result<()> {
    let listener = kumo_ipc::listen(name)?;
    {
        let mut stdout = io::stdout().lock();
        writeln!(stdout, "ready")?;
        stdout.flush()?;
    }
    let service = Arc::new(service);
    let owner = kumo_ipc::accept(&listener)?;
    let others = service.clone();
    std::thread::spawn(move || {
        while let Ok(conn) = kumo_ipc::accept(&listener) {
            let service = others.clone();
            std::thread::spawn(move || {
                if let Err(e) = serve(service, conn.clone(), conn, private_bytes, true) {
                    eprintln!("[network] 接続エラー: {e}");
                }
            });
        }
    });
    let result = serve(service.clone(), owner.clone(), owner, private_bytes, true);
    service.persist();
    result
}
