//! 閲覧履歴（純Rustの組み込みDB redb に保存。設計書の永続化方針）。

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

const VISITS: TableDefinition<&str, &[u8]> = TableDefinition::new("visits");
/// redb のページキャッシュ（履歴は小さいので最小限）。
const REDB_CACHE_BYTES: usize = 512 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Visit {
    pub title: String,
    pub last_visit: u64,
    pub count: u32,
}

/// 履歴の候補（URL 欄の補完用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub url: String,
    pub visit: Visit,
}

pub struct HistoryDb {
    pub(crate) db: Database,
}

#[derive(Debug)]
pub struct HistoryError(String);

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "履歴: {}", self.0)
    }
}

impl std::error::Error for HistoryError {}

pub(crate) fn err(e: impl std::fmt::Display) -> HistoryError {
    HistoryError(e.to_string())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl HistoryDb {
    pub fn open(path: &Path) -> Result<Self, HistoryError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        let db = Database::builder()
            .set_cache_size(REDB_CACHE_BYTES)
            .create(path)
            .map_err(err)?;
        let txn = db.begin_write().map_err(err)?;
        txn.open_table(VISITS).map_err(err)?;
        txn.open_table(crate::bookmarks::BOOKMARKS).map_err(err)?;
        txn.open_table(crate::settings::SETTINGS).map_err(err)?;
        txn.open_table(crate::logins::LOGINS).map_err(err)?;
        txn.open_table(crate::logins::NEVER_SAVE).map_err(err)?;
        txn.commit().map_err(err)?;
        Ok(Self { db })
    }

    /// 訪問を記録する（同じ URL なら回数とタイトルを更新）。
    pub fn record(&self, url: &str, title: &str) -> Result<(), HistoryError> {
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(VISITS).map_err(err)?;
            let previous = table
                .get(url)
                .map_err(err)?
                .and_then(|v| postcard::from_bytes::<Visit>(v.value()).ok());
            let visit = Visit {
                title: title.to_owned(),
                last_visit: now(),
                count: previous.map_or(1, |p| p.count.saturating_add(1)),
            };
            let bytes = postcard::to_stdvec(&visit).map_err(err)?;
            table.insert(url, bytes.as_slice()).map_err(err)?;
        }
        txn.commit().map_err(err)
    }

    pub fn get(&self, url: &str) -> Option<Visit> {
        let txn = self.db.begin_read().ok()?;
        let table = txn.open_table(VISITS).ok()?;
        let value = table.get(url).ok()??;
        postcard::from_bytes(value.value()).ok()
    }

    /// URL かタイトルに `query` を含む履歴を、訪問回数・新しさの順で最大 `limit` 件。
    pub fn search(&self, query: &str, limit: usize) -> Vec<HistoryEntry> {
        let query = query.to_lowercase();
        let Ok(txn) = self.db.begin_read() else {
            return Vec::new();
        };
        let Ok(table) = txn.open_table(VISITS) else {
            return Vec::new();
        };
        let Ok(rows) = table.iter() else {
            return Vec::new();
        };
        let mut found: Vec<HistoryEntry> = rows
            .filter_map(Result::ok)
            .filter_map(|(key, value)| {
                let url = key.value().to_owned();
                let visit: Visit = postcard::from_bytes(value.value()).ok()?;
                let hit = url.to_lowercase().contains(&query)
                    || visit.title.to_lowercase().contains(&query);
                hit.then_some(HistoryEntry { url, visit })
            })
            .collect();
        found.sort_by(|a, b| {
            b.visit
                .count
                .cmp(&a.visit.count)
                .then(b.visit.last_visit.cmp(&a.visit.last_visit))
        });
        found.truncate(limit);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_searches_visits() {
        let path = std::env::temp_dir().join(format!("kumo-history-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let db = HistoryDb::open(&path).unwrap();
            db.record("https://www.rust-lang.org/", "Rust").unwrap();
            db.record("https://en.wikipedia.org/wiki/Rust", "Rust - Wikipedia")
                .unwrap();
            db.record(
                "https://en.wikipedia.org/wiki/Rust",
                "Rust (programming language)",
            )
            .unwrap();
            db.record("https://example.com/", "Example").unwrap();
        }
        let db = HistoryDb::open(&path).unwrap();
        let wiki = db.get("https://en.wikipedia.org/wiki/Rust").unwrap();
        assert_eq!(wiki.count, 2);
        assert_eq!(wiki.title, "Rust (programming language)");
        let hits = db.search("rust", 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0].url, "https://en.wikipedia.org/wiki/Rust",
            "回数の多い順"
        );
        assert_eq!(
            db.search("EXAMPLE", 10).len(),
            1,
            "大文字小文字を区別しない"
        );
        assert_eq!(db.search("rust", 1).len(), 1);
        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
