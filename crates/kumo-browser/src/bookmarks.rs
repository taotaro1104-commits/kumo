//! ブックマーク（履歴と同じ redb に保存する）。

use std::time::{SystemTime, UNIX_EPOCH};

use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::history::{HistoryDb, HistoryError, err};

pub(crate) const BOOKMARKS: TableDefinition<&str, &[u8]> = TableDefinition::new("bookmarks");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub url: String,
    pub title: String,
    /// 追加した時刻（UNIX 秒）
    pub added: u64,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl HistoryDb {
    /// ブックマークを追加する（同じ URL ならタイトルだけ更新し、追加時刻は保つ）。
    pub fn add_bookmark(&self, url: &str, title: &str) -> Result<(), HistoryError> {
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(BOOKMARKS).map_err(err)?;
            let added = table
                .get(url)
                .map_err(err)?
                .and_then(|v| postcard::from_bytes::<Bookmark>(v.value()).ok())
                .map_or_else(now, |b| b.added);
            let bookmark = Bookmark {
                url: url.to_owned(),
                title: title.to_owned(),
                added,
            };
            let bytes = postcard::to_stdvec(&bookmark).map_err(err)?;
            table.insert(url, bytes.as_slice()).map_err(err)?;
        }
        txn.commit().map_err(err)
    }

    pub fn remove_bookmark(&self, url: &str) -> Result<(), HistoryError> {
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(BOOKMARKS).map_err(err)?;
            table.remove(url).map_err(err)?;
        }
        txn.commit().map_err(err)
    }

    pub fn is_bookmarked(&self, url: &str) -> bool {
        let Ok(txn) = self.db.begin_read() else {
            return false;
        };
        let Ok(table) = txn.open_table(BOOKMARKS) else {
            return false;
        };
        table.get(url).is_ok_and(|v| v.is_some())
    }

    /// 追加の新しい順。
    pub fn bookmarks(&self) -> Vec<Bookmark> {
        let Ok(txn) = self.db.begin_read() else {
            return Vec::new();
        };
        let Ok(table) = txn.open_table(BOOKMARKS) else {
            return Vec::new();
        };
        let Ok(rows) = table.iter() else {
            return Vec::new();
        };
        let mut list: Vec<Bookmark> = rows
            .filter_map(Result::ok)
            .filter_map(|(_, v)| postcard::from_bytes(v.value()).ok())
            .collect();
        list.sort_by(|a, b| b.added.cmp(&a.added).then_with(|| a.url.cmp(&b.url)));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_remove_and_persist() {
        let path = std::env::temp_dir().join(format!("kumo-bookmarks-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let db = HistoryDb::open(&path).unwrap();
            assert!(!db.is_bookmarked("https://a.example/"));
            db.add_bookmark("https://a.example/", "A").unwrap();
            db.add_bookmark("https://b.example/", "B").unwrap();
            db.add_bookmark("https://a.example/", "A2").unwrap();
            db.remove_bookmark("https://b.example/").unwrap();
            db.remove_bookmark("https://none.example/").unwrap();
        }
        let db = HistoryDb::open(&path).unwrap();
        assert!(db.is_bookmarked("https://a.example/"));
        assert!(!db.is_bookmarked("https://b.example/"));
        let list = db.bookmarks();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "A2", "タイトルは更新される");
        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
