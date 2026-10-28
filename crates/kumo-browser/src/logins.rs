//! 保存したログイン情報（履歴と同じ redb に、パスワードだけ暗号化して置く）。
//! キーは `origin NUL ユーザー名`。値は `Vault` で包んだ [`Secret`]（キーを認証データにして、
//! 別の行へ移し替えても開けないようにする）。

use std::time::{SystemTime, UNIX_EPOCH};

use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::history::{HistoryDb, HistoryError, err};
use crate::vault::Vault;

pub(crate) const LOGINS: TableDefinition<&str, &[u8]> = TableDefinition::new("logins");
/// 「このサイトでは保存しない」の origin。
pub(crate) const NEVER_SAVE: TableDefinition<&str, ()> = TableDefinition::new("login_never");

#[derive(Serialize, Deserialize)]
struct Secret {
    password: String,
    /// 保存した時刻（UNIX 秒）
    saved: u64,
}

/// 一覧に出す情報（パスワードは含めない）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginSummary {
    pub origin: String,
    pub username: String,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// ページの origin（`https://host[:port]`）。http(s) 以外は `None`。
pub fn origin_of(url: &Url) -> Option<String> {
    matches!(url.scheme(), "http" | "https")
        .then(|| url.origin().ascii_serialization())
        .filter(|o| o != "null")
}

fn row_key(origin: &str, username: &str) -> String {
    format!("{origin}\0{username}")
}

fn split_key(key: &str) -> Option<(&str, &str)> {
    key.split_once('\0')
}

impl HistoryDb {
    /// ログイン情報を保存する（同じ origin・ユーザー名なら上書き）。
    pub fn save_login(
        &self,
        vault: &Vault,
        origin: &str,
        username: &str,
        password: &str,
    ) -> Result<(), HistoryError> {
        let key = row_key(origin, username);
        let secret = Secret {
            password: password.to_owned(),
            saved: now(),
        };
        let plain = postcard::to_stdvec(&secret).map_err(err)?;
        let sealed = vault.seal(&plain, key.as_bytes()).map_err(err)?;
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(LOGINS).map_err(err)?;
            table.insert(key.as_str(), sealed.as_slice()).map_err(err)?;
        }
        txn.commit().map_err(err)
    }

    /// 保存したパスワード（なければ・復号できなければ `None`）。
    pub fn login_password(&self, vault: &Vault, origin: &str, username: &str) -> Option<String> {
        let key = row_key(origin, username);
        let txn = self.db.begin_read().ok()?;
        let table = txn.open_table(LOGINS).ok()?;
        let row = table.get(key.as_str()).ok()??;
        let plain = vault.open(row.value(), key.as_bytes()).ok()?;
        postcard::from_bytes::<Secret>(&plain)
            .ok()
            .map(|s| s.password)
    }

    /// その origin に保存したログイン情報があるか（鍵がなくても調べられる）。
    pub fn has_logins(&self, origin: &str) -> bool {
        !self.logins_of(origin).is_empty()
    }

    /// その origin のユーザー名（保存の新しい順は付けない。キー順）。
    pub fn logins_of(&self, origin: &str) -> Vec<String> {
        self.logins()
            .into_iter()
            .filter(|l| l.origin == origin)
            .map(|l| l.username)
            .collect()
    }

    /// すべてのログイン情報（パスワードなし）。origin、ユーザー名の順。
    pub fn logins(&self) -> Vec<LoginSummary> {
        let Ok(txn) = self.db.begin_read() else {
            return Vec::new();
        };
        let Ok(table) = txn.open_table(LOGINS) else {
            return Vec::new();
        };
        let Ok(rows) = table.iter() else {
            return Vec::new();
        };
        rows.filter_map(Result::ok)
            .filter_map(|(k, _)| {
                let (origin, username) = split_key(k.value())?;
                Some(LoginSummary {
                    origin: origin.to_owned(),
                    username: username.to_owned(),
                })
            })
            .collect()
    }

    pub fn remove_login(&self, origin: &str, username: &str) -> Result<(), HistoryError> {
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(LOGINS).map_err(err)?;
            table
                .remove(row_key(origin, username).as_str())
                .map_err(err)?;
        }
        txn.commit().map_err(err)
    }

    /// この origin では保存を勧めない／勧める。
    pub fn set_never_save(&self, origin: &str, never: bool) -> Result<(), HistoryError> {
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(NEVER_SAVE).map_err(err)?;
            if never {
                table.insert(origin, ()).map_err(err)?;
            } else {
                table.remove(origin).map_err(err)?;
            }
        }
        txn.commit().map_err(err)
    }

    pub fn is_never_save(&self, origin: &str) -> bool {
        let Ok(txn) = self.db.begin_read() else {
            return false;
        };
        let Ok(table) = txn.open_table(NEVER_SAVE) else {
            return false;
        };
        table.get(origin).is_ok_and(|v| v.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(name: &str) -> (HistoryDb, std::path::PathBuf) {
        let path =
            std::env::temp_dir().join(format!("kumo-logins-{name}-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        (HistoryDb::open(&path).unwrap(), path)
    }

    #[test]
    fn origin_keeps_scheme_host_and_port_only() {
        let o = |s: &str| origin_of(&Url::parse(s).unwrap());
        assert_eq!(
            o("https://a.example/x?y#z").as_deref(),
            Some("https://a.example")
        );
        assert_eq!(
            o("http://a.example:8080/").as_deref(),
            Some("http://a.example:8080")
        );
        assert_ne!(o("http://a.example/"), o("https://a.example/"));
        assert_eq!(o("about:settings"), None);
        assert_eq!(o("file:///c:/x.html"), None);
    }

    #[test]
    fn passwords_are_encrypted_at_rest_and_persist() {
        let (db, path) = db("rest");
        let vault = Vault::from_key([5; 32]);
        db.save_login(&vault, "https://a.example", "alice", "s3cret-pw")
            .unwrap();
        db.save_login(&vault, "https://a.example", "alice", "s3cret-pw2")
            .unwrap();
        db.save_login(&vault, "https://b.example", "bob", "x")
            .unwrap();
        assert_eq!(
            db.login_password(&vault, "https://a.example", "alice")
                .as_deref(),
            Some("s3cret-pw2"),
            "同じ組は上書き"
        );
        assert_eq!(db.logins().len(), 2);
        assert!(db.has_logins("https://b.example") && !db.has_logins("https://c.example"));
        assert_eq!(db.logins_of("https://a.example"), vec!["alice".to_owned()]);
        let other = Vault::from_key([6; 32]);
        assert_eq!(
            db.login_password(&other, "https://a.example", "alice"),
            None
        );
        drop(db);
        let raw = std::fs::read(&path).unwrap();
        assert!(
            !raw.windows(9).any(|w| w == b"s3cret-pw"),
            "ファイルに平文が残らない"
        );
        let db = HistoryDb::open(&path).unwrap();
        assert_eq!(
            db.login_password(&vault, "https://b.example", "bob")
                .as_deref(),
            Some("x")
        );
        db.remove_login("https://b.example", "bob").unwrap();
        assert!(!db.has_logins("https://b.example"));
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_row_cannot_be_moved_to_another_site() {
        let (db, path) = db("move");
        let vault = Vault::from_key([5; 32]);
        db.save_login(&vault, "https://good.example", "u", "pw")
            .unwrap();
        // 別の origin のキーへ暗号文だけ写しても、認証データが違うので開けない
        let key = row_key("https://good.example", "u");
        let sealed = {
            let txn = db.db.begin_read().unwrap();
            let table = txn.open_table(LOGINS).unwrap();
            table.get(key.as_str()).unwrap().unwrap().value().to_vec()
        };
        let txn = db.db.begin_write().unwrap();
        {
            let mut table = txn.open_table(LOGINS).unwrap();
            let evil = row_key("https://evil.example", "u");
            table.insert(evil.as_str(), sealed.as_slice()).unwrap();
        }
        txn.commit().unwrap();
        assert_eq!(db.login_password(&vault, "https://evil.example", "u"), None);
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn never_save_is_remembered_per_origin() {
        let (db, path) = db("never");
        assert!(!db.is_never_save("https://a.example"));
        db.set_never_save("https://a.example", true).unwrap();
        assert!(db.is_never_save("https://a.example") && !db.is_never_save("https://b.example"));
        db.set_never_save("https://a.example", false).unwrap();
        assert!(!db.is_never_save("https://a.example"));
        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
