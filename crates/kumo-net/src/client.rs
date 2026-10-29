//! Network プロセスのクライアント側。ID で応答を対応づけ、共有リングの本文を受け取って返却する。

use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use kumo_ipc::{Connection, SharedView};

use crate::process::{FetchResult, Fetcher, LISTEN_ARG, post_method, script_method};
use crate::protocol::{
    DownloadEvent, FetchError, FetchRequest, FetchResponse, FromNetwork, NetworkStats,
    RequestMethod, ResourceKind, ToNetwork, read_message, write_message,
};

/// 1件の応答を待つ上限（取得自体のタイムアウトより長くとる）。
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(90);

static SOCKET_COUNTER: AtomicU64 = AtomicU64::new(0);

type Pending = Arc<Mutex<HashMap<u64, Sender<FromNetwork>>>>;
type Writer = Arc<Mutex<Option<Connection>>>;
type DownloadQueue = Arc<Mutex<VecDeque<DownloadEvent>>>;

/// Network プロセスへの接続。
pub struct NetworkClient {
    writer: Writer,
    /// 自分で起動した子プロセス（[`NetworkClient::connect`] で繋いだ場合は `None`）
    child: Option<Mutex<Child>>,
    socket_name: String,
    pending: Pending,
    connected: Arc<AtomicBool>,
    shared: Arc<AtomicU64>,
    next_id: AtomicU64,
    reader: Option<JoinHandle<()>>,
    /// ダウンロードの経過（受信スレッドが積み、[`Fetcher::take_download_events`] で取り出す）
    downloads: DownloadQueue,
}

impl NetworkClient {
    /// `command`（Network プロセスとして動く実行ファイルと引数）を起動して接続する。
    /// 子プロセスには待ち受けるソケット名を渡し、`ready` を待ってから接続する。
    pub fn spawn(mut command: Command) -> io::Result<Self> {
        let n = SOCKET_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = kumo_ipc::socket_name(&format!("net{n}"));
        let mut child = command
            .arg(LISTEN_ARG)
            .arg(&name)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("stdout がない"))?;
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line)?;
        if line.trim() != "ready" {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("Network プロセスが待ち受けを始めなかった"));
        }
        let connection = kumo_ipc::connect(&name)?;
        Ok(Self::with_connection(connection, Some(child), name))
    }

    /// 既に動いている Network プロセスへ接続する（Content プロセスから）。
    pub fn connect(socket_name: &str) -> io::Result<Self> {
        let connection = kumo_ipc::connect(socket_name)?;
        Ok(Self::with_connection(
            connection,
            None,
            socket_name.to_owned(),
        ))
    }

    fn with_connection(connection: Connection, child: Option<Child>, socket_name: String) -> Self {
        let writer: Writer = Arc::new(Mutex::new(Some(connection.clone())));
        let pending: Pending = Arc::default();
        let connected = Arc::new(AtomicBool::new(true));
        let shared = Arc::new(AtomicU64::new(0));
        let downloads = DownloadQueue::default();
        let reader = {
            let mut state = ReaderState {
                pending: pending.clone(),
                connected: connected.clone(),
                writer: writer.clone(),
                shared: shared.clone(),
                ring: None,
                downloads: downloads.clone(),
            };
            std::thread::spawn(move || state.run(BufReader::new(connection)))
        };
        Self {
            writer,
            child: child.map(Mutex::new),
            socket_name,
            pending,
            connected,
            shared,
            next_id: AtomicU64::new(1),
            reader: Some(reader),
            downloads,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    /// 他のプロセスが [`NetworkClient::connect`] に渡すソケット名。
    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    /// 子プロセスの ID（計測用）。自分で起動していなければ 0。
    pub fn process_id(&self) -> u32 {
        self.child
            .as_ref()
            .and_then(|c| c.lock().ok().map(|c| c.id()))
            .unwrap_or(0)
    }

    /// 共有リング経由で受け取った本文の数（計測・テスト用）。
    pub fn shared_responses(&self) -> u64 {
        self.shared.load(Ordering::Relaxed)
    }

    /// メッセージを送り、応答を受け取るチャネルを返す。
    fn submit(
        &self,
        make: impl FnOnce(u64) -> ToNetwork,
    ) -> Result<Receiver<FromNetwork>, FetchError> {
        if !self.is_connected() {
            return Err(FetchError::Disconnected);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        self.pending
            .lock()
            .map_err(|_| FetchError::Disconnected)?
            .insert(id, tx);
        if send(&self.writer, &make(id)).is_err() {
            self.connected.store(false, Ordering::Release);
            if let Ok(mut pending) = self.pending.lock() {
                pending.remove(&id);
            }
            return Err(FetchError::Disconnected);
        }
        Ok(rx)
    }

    fn wait(rx: &Receiver<FromNetwork>) -> FetchResult {
        match rx.recv_timeout(RESPONSE_TIMEOUT) {
            Ok(FromNetwork::Response(response)) => Ok(response),
            Ok(FromNetwork::Failed { error, .. }) => Err(error),
            Ok(_) => Err(FetchError::Network("予期しない応答".into())),
            Err(_) => Err(FetchError::Disconnected),
        }
    }

    fn request(
        url: &str,
        method: RequestMethod,
        kind: ResourceKind,
        source_url: Option<&str>,
    ) -> impl FnOnce(u64) -> ToNetwork {
        let (url, source_url) = (url.to_owned(), source_url.map(str::to_owned));
        move |id| {
            ToNetwork::Fetch(FetchRequest {
                id,
                url,
                method,
                kind,
                source_url,
            })
        }
    }
}

fn send(writer: &Writer, message: &ToNetwork) -> io::Result<()> {
    let mut guard = writer
        .lock()
        .map_err(|_| io::Error::other("送信ロックが汚染された"))?;
    let conn = guard
        .as_mut()
        .ok_or_else(|| io::Error::other("閉じている"))?;
    write_message(conn, message)
}

/// 受信スレッドの状態。
struct ReaderState {
    pending: Pending,
    connected: Arc<AtomicBool>,
    writer: Writer,
    shared: Arc<AtomicU64>,
    ring: Option<SharedView>,
    downloads: DownloadQueue,
}

impl ReaderState {
    fn run(&mut self, mut input: impl Read) {
        while let Ok(Some(message)) = read_message::<FromNetwork>(&mut input) {
            if let Some(message) = self.resolve(message) {
                self.deliver(message);
            }
            // 前の応答（受け取った側がもう解放したもの）で空いたこのスレッドのページを返す
            kumo_mem::release_free_memory();
        }
        // 切断：待っている全リクエストを Disconnected で終わらせる（送信側を落とす）
        self.connected.store(false, Ordering::Release);
        if let Ok(mut p) = self.pending.lock() {
            p.clear();
        }
    }

    /// Hello を処理し、共有リングの本文を複製して通常の応答にする。
    fn resolve(&mut self, message: FromNetwork) -> Option<FromNetwork> {
        match message {
            FromNetwork::Hello { ring_path, .. } => {
                match SharedView::open(Path::new(&ring_path)) {
                    Ok(ring) => self.ring = Some(ring),
                    Err(e) => eprintln!("[network] 本文リングを開けません: {e}"),
                }
                None
            }
            FromNetwork::SharedResponse { response, pos, len } => {
                Some(self.take_shared(response, pos, len))
            }
            other => Some(other),
        }
    }

    fn take_shared(&self, response: FetchResponse, pos: u64, len: u64) -> FromNetwork {
        let id = response.id;
        let body = self.ring.as_ref().map(|ring| {
            let offset = (pos % ring.len() as u64) as usize;
            ring.read_at(offset, len as usize)
        });
        // 複製したら（失敗しても）すぐ返却して、Network 側が次の本文に使えるようにする
        let _ = send(&self.writer, &ToNetwork::Release { pos });
        match body {
            Some(Ok(body)) => {
                self.shared.fetch_add(1, Ordering::Relaxed);
                FromNetwork::Response(FetchResponse { body, ..response })
            }
            _ => FromNetwork::Failed {
                id,
                error: FetchError::Network("共有メモリの本文を読めません".into()),
            },
        }
    }

    fn deliver(&self, message: FromNetwork) {
        let id = match &message {
            FromNetwork::Download(event) => {
                if let Ok(mut queue) = self.downloads.lock() {
                    queue.push_back(event.clone());
                }
                return;
            }
            FromNetwork::Response(r) => r.id,
            FromNetwork::Failed { id, .. } | FromNetwork::Stats { id, .. } => *id,
            FromNetwork::Hello { .. } | FromNetwork::SharedResponse { .. } => return,
        };
        let sender = self.pending.lock().ok().and_then(|mut p| p.remove(&id));
        if let Some(sender) = sender {
            let _ = sender.send(message);
        }
    }
}

impl Fetcher for NetworkClient {
    fn send(
        &self,
        method: &str,
        url: &str,
        kind: ResourceKind,
        content_type: Option<&str>,
        body: Vec<u8>,
        source_url: Option<&str>,
    ) -> FetchResult {
        let method = script_method(method, content_type, body);
        let rx = self.submit(Self::request(url, method, kind, source_url))?;
        Self::wait(&rx)
    }

    fn fetch(&self, url: &str, kind: ResourceKind, source_url: Option<&str>) -> FetchResult {
        let rx = self.submit(Self::request(url, RequestMethod::Get, kind, source_url))?;
        Self::wait(&rx)
    }

    fn post(
        &self,
        url: &str,
        content_type: &str,
        body: Vec<u8>,
        source_url: Option<&str>,
    ) -> FetchResult {
        let method = post_method(content_type, body);
        let rx = self.submit(Self::request(
            url,
            method,
            ResourceKind::Document,
            source_url,
        ))?;
        Self::wait(&rx)
    }

    fn fetch_all(
        &self,
        requests: &[(String, ResourceKind)],
        source_url: Option<&str>,
    ) -> Vec<FetchResult> {
        // 先に全部送ってから待つので、Network プロセス側で並行に取得される
        let receivers: Vec<_> = requests
            .iter()
            .map(|(url, kind)| {
                self.submit(Self::request(url, RequestMethod::Get, *kind, source_url))
            })
            .collect();
        receivers
            .into_iter()
            .map(|rx| rx.and_then(|rx| Self::wait(&rx)))
            .collect()
    }

    fn start_download(&self, url: &str, dir: &Path, source_url: Option<&str>) -> Option<u64> {
        if !self.is_connected() {
            return None;
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let message = ToNetwork::Download {
            id,
            url: url.to_owned(),
            dir: dir.to_string_lossy().into_owned(),
            source_url: source_url.map(str::to_owned),
        };
        send(&self.writer, &message).ok().map(|()| id)
    }

    fn cancel_download(&self, id: u64) {
        let _ = send(&self.writer, &ToNetwork::CancelDownload { id });
    }

    fn take_download_events(&self) -> Vec<DownloadEvent> {
        self.downloads
            .lock()
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default()
    }

    fn stats(&self) -> Option<NetworkStats> {
        let rx = self.submit(|id| ToNetwork::Stats { id }).ok()?;
        match rx.recv_timeout(RESPONSE_TIMEOUT) {
            Ok(FromNetwork::Stats { stats, .. }) => Some(stats),
            _ => None,
        }
    }
}

impl Drop for NetworkClient {
    fn drop(&mut self) {
        // Shutdown を受けたサーバは接続を閉じるので、受信スレッドも終わる
        let _ = send(&self.writer, &ToNetwork::Shutdown);
        if let Ok(mut writer) = self.writer.lock() {
            *writer = None;
        }
        if let Some(child) = &self.child
            && let Ok(mut child) = child.lock()
        {
            // 正常終了を少し待ち、だめなら強制終了する
            for _ in 0..50 {
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
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}
