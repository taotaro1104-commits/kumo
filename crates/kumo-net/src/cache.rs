//! HTTP キャッシュ。メモリ上は予算付き LRU、あふれた分はディスク（本文はファイル、索引は redb）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// メモリ上のキャッシュ予算（設計書：8MB）。
pub const MEMORY_BUDGET: usize = 8 * 1024 * 1024;
/// ディスクキャッシュの予算。
pub const DISK_BUDGET: u64 = 256 * 1024 * 1024;
/// redb 自身のページキャッシュ（索引は小さいので最小限）。
const REDB_CACHE_BYTES: usize = 1024 * 1024;

const ENTRIES: TableDefinition<&str, &[u8]> = TableDefinition::new("entries");

/// 保存しているレスポンスの属性。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedMeta {
    pub final_url: String,
    pub content_type: Option<String>,
    /// 新鮮でいられる期限（UNIX 秒）。`None` は毎回再検証
    pub fresh_until: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub size: u64,
    /// 最後に保存・再検証した時刻（UNIX 秒）。ディスクの追い出し順に使う
    pub stored_at: u64,
}

impl CachedMeta {
    pub fn is_fresh(&self, now: SystemTime) -> bool {
        self.fresh_until.is_some_and(|until| unix(now) < until)
    }

    pub fn has_validator(&self) -> bool {
        self.etag.is_some() || self.last_modified.is_some()
    }
}

#[derive(Clone, Debug)]
pub struct CachedEntry {
    pub meta: CachedMeta,
    pub body: Arc<[u8]>,
}

pub fn unix(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub fn from_unix(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

struct MemoryEntry {
    entry: CachedEntry,
    last_used: u64,
}

struct Disk {
    db: Database,
    bodies: PathBuf,
    bytes: u64,
    budget: u64,
}

struct Inner {
    memory: HashMap<String, MemoryEntry>,
    memory_bytes: usize,
    memory_budget: usize,
    tick: u64,
    disk: Option<Disk>,
}

/// スレッド安全な HTTP キャッシュ。
pub struct HttpCache {
    inner: Mutex<Inner>,
}

#[derive(Debug)]
pub struct CacheError(pub String);

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "キャッシュ: {}", self.0)
    }
}

impl std::error::Error for CacheError {}

fn err(e: impl std::fmt::Display) -> CacheError {
    CacheError(e.to_string())
}

impl HttpCache {
    /// メモリのみ（ディスクに書かない）。
    pub fn in_memory(memory_budget: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                memory: HashMap::new(),
                memory_bytes: 0,
                memory_budget,
                tick: 0,
                disk: None,
            }),
        }
    }

    /// `dir` にディスクキャッシュを開く（なければ作る）。
    pub fn open(dir: &Path, memory_budget: usize, disk_budget: u64) -> Result<Self, CacheError> {
        let bodies = dir.join("bodies");
        std::fs::create_dir_all(&bodies).map_err(err)?;
        let db = Database::builder()
            .set_cache_size(REDB_CACHE_BYTES)
            .create(dir.join("index.redb"))
            .map_err(err)?;
        let bytes = {
            let txn = db.begin_write().map_err(err)?;
            let total = {
                let table = txn.open_table(ENTRIES).map_err(err)?;
                let mut total = 0u64;
                for row in table.iter().map_err(err)? {
                    let (_, value) = row.map_err(err)?;
                    if let Ok(meta) = postcard::from_bytes::<CachedMeta>(value.value()) {
                        total += meta.size;
                    }
                }
                total
            };
            txn.commit().map_err(err)?;
            total
        };
        let cache = Self::in_memory(memory_budget);
        cache.lock().disk = Some(Disk {
            db,
            bodies,
            bytes,
            budget: disk_budget,
        });
        Ok(cache)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // 他スレッドの panic でロックが汚染されても、キャッシュは捨てて続行できる
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn memory_bytes(&self) -> usize {
        self.lock().memory_bytes
    }

    pub fn disk_bytes(&self) -> u64 {
        self.lock().disk.as_ref().map_or(0, |d| d.bytes)
    }

    /// メモリ → ディスクの順に探す。ディスクで見つかればメモリに載せる。
    pub fn get(&self, url: &str) -> Option<CachedEntry> {
        let mut inner = self.lock();
        inner.tick += 1;
        let tick = inner.tick;
        if let Some(found) = inner.memory.get_mut(url) {
            found.last_used = tick;
            return Some(found.entry.clone());
        }
        let entry = inner.disk.as_ref().and_then(|disk| disk.read(url))?;
        inner.insert_memory(url, entry.clone());
        Some(entry)
    }

    /// レスポンスを保存する。
    pub fn put(&self, url: &str, meta: CachedMeta, body: &[u8]) {
        // キャッシュに残る本文は長命データなので、呼び出し側の一時ヒープではなく通常ヒープに置く
        let entry = kumo_mem::with_long_lived(|| CachedEntry {
            meta,
            body: Arc::from(body),
        });
        let mut inner = self.lock();
        if let Some(disk) = inner.disk.as_mut()
            && let Err(e) = disk.write(url, &entry)
        {
            eprintln!("[cache] ディスクへの保存に失敗: {e}");
        }
        kumo_mem::with_long_lived(|| inner.insert_memory(url, entry));
    }

    /// 再検証（304）の結果で属性だけを更新する。
    pub fn refresh(&self, url: &str, meta: CachedMeta) {
        let mut inner = self.lock();
        if let Some(found) = inner.memory.get_mut(url) {
            found.entry.meta = meta.clone();
        }
        if let Some(disk) = inner.disk.as_mut()
            && let Err(e) = disk.write_meta(url, &meta)
        {
            eprintln!("[cache] 索引の更新に失敗: {e}");
        }
    }
}

impl Inner {
    fn insert_memory(&mut self, url: &str, entry: CachedEntry) {
        let size = entry.body.len();
        if size > self.memory_budget / 4 {
            return; // 大きすぎるものはディスクだけに置く
        }
        self.tick += 1;
        if let Some(old) = self.memory.insert(
            url.to_owned(),
            MemoryEntry {
                entry,
                last_used: self.tick,
            },
        ) {
            self.memory_bytes -= old.entry.body.len();
        }
        self.memory_bytes += size;
        while self.memory_bytes > self.memory_budget {
            let Some(oldest) = self
                .memory
                .iter()
                .min_by_key(|(_, e)| e.last_used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            if let Some(removed) = self.memory.remove(&oldest) {
                self.memory_bytes -= removed.entry.body.len();
            }
        }
    }
}

fn body_name(url: &str) -> String {
    let digest = Sha256::digest(url.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

impl Disk {
    fn body_path(&self, url: &str) -> PathBuf {
        self.bodies.join(body_name(url))
    }

    fn read_meta(&self, url: &str) -> Option<CachedMeta> {
        let txn = self.db.begin_read().ok()?;
        let table = txn.open_table(ENTRIES).ok()?;
        let value = table.get(url).ok()??;
        postcard::from_bytes(value.value()).ok()
    }

    fn read(&self, url: &str) -> Option<CachedEntry> {
        let meta = self.read_meta(url)?;
        let body = std::fs::read(self.body_path(url)).ok()?;
        (body.len() as u64 == meta.size).then(|| CachedEntry {
            meta,
            body: Arc::from(body),
        })
    }

    fn write(&mut self, url: &str, entry: &CachedEntry) -> Result<(), CacheError> {
        let old = self.read_meta(url).map_or(0, |m| m.size);
        std::fs::write(self.body_path(url), &entry.body).map_err(err)?;
        self.write_meta(url, &entry.meta)?;
        self.bytes = self.bytes - old.min(self.bytes) + entry.meta.size;
        if self.bytes > self.budget {
            self.evict()?;
        }
        Ok(())
    }

    fn write_meta(&mut self, url: &str, meta: &CachedMeta) -> Result<(), CacheError> {
        let bytes = postcard::to_stdvec(meta).map_err(err)?;
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(ENTRIES).map_err(err)?;
            table.insert(url, bytes.as_slice()).map_err(err)?;
        }
        txn.commit().map_err(err)
    }

    /// 古いものから予算の 90% まで追い出す。
    fn evict(&mut self) -> Result<(), CacheError> {
        let target = self.budget / 10 * 9;
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(ENTRIES).map_err(err)?;
            let mut all: Vec<(String, CachedMeta)> = Vec::new();
            for row in table.iter().map_err(err)? {
                let (key, value) = row.map_err(err)?;
                if let Ok(meta) = postcard::from_bytes::<CachedMeta>(value.value()) {
                    all.push((key.value().to_owned(), meta));
                }
            }
            all.sort_by_key(|(_, meta)| meta.stored_at);
            for (url, meta) in all {
                if self.bytes <= target {
                    break;
                }
                table.remove(url.as_str()).map_err(err)?;
                let _ = std::fs::remove_file(self.body_path(&url));
                self.bytes = self.bytes.saturating_sub(meta.size);
            }
        }
        txn.commit().map_err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(size: usize, stored_at: u64) -> CachedMeta {
        CachedMeta {
            final_url: "u".into(),
            content_type: Some("text/css".into()),
            fresh_until: Some(stored_at + 60),
            etag: None,
            last_modified: None,
            size: size as u64,
            stored_at,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kumo-cache-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn memory_lru_respects_budget() {
        let cache = HttpCache::in_memory(1000);
        for i in 0..10 {
            cache.put(&format!("u{i}"), meta(200, i), &[0u8; 200]);
        }
        assert!(cache.memory_bytes() <= 1000);
        assert!(cache.get("u9").is_some(), "最新は残る");
        assert!(cache.get("u0").is_none(), "最古は追い出される");
    }

    #[test]
    fn disk_cache_survives_reopen_and_evicts() {
        let dir = temp_dir("disk");
        {
            let cache = HttpCache::open(&dir, 1000, 1500).unwrap();
            cache.put("a", meta(600, 1), &[1u8; 600]);
            cache.put("b", meta(600, 2), &[2u8; 600]);
            cache.put("c", meta(600, 3), &[3u8; 600]);
            assert!(cache.disk_bytes() <= 1500);
        }
        let reopened = HttpCache::open(&dir, 1000, 1500).unwrap();
        assert!(reopened.get("a").is_none(), "古いものは追い出し済み");
        let c = reopened.get("c").expect("ディスクから読める");
        assert_eq!(&c.body[..3], &[3, 3, 3]);
        let mut refreshed = c.meta.clone();
        refreshed.fresh_until = Some(999);
        reopened.refresh("c", refreshed);
        drop(reopened);
        let again = HttpCache::open(&dir, 1000, 1500).unwrap();
        assert_eq!(again.get("c").unwrap().meta.fresh_until, Some(999));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn freshness() {
        let m = meta(1, 100);
        assert!(m.is_fresh(from_unix(150)));
        assert!(!m.is_fresh(from_unix(160)));
        assert!(!m.has_validator());
    }
}
