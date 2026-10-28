//! 設定（キーと文字列の組を履歴と同じ redb に保存する）。いまはホーム URL だけ。

use redb::{ReadableDatabase, TableDefinition};

use crate::history::{HistoryDb, HistoryError, err};

pub(crate) const SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("settings");
const HOME_KEY: &str = "home";
const LEVEL_KEY: &str = "load_level";
const IMAGES_KEY: &str = "images";
const PASSWORDS_KEY: &str = "passwords";
const LANGUAGE_KEY: &str = "language";

impl HistoryDb {
    fn setting(&self, key: &str) -> Option<String> {
        let txn = self.db.begin_read().ok()?;
        let table = txn.open_table(SETTINGS).ok()?;
        let value = table.get(key).ok()??;
        Some(value.value().to_owned())
    }

    fn set_setting(&self, key: &str, value: Option<&str>) -> Result<(), HistoryError> {
        let txn = self.db.begin_write().map_err(err)?;
        {
            let mut table = txn.open_table(SETTINGS).map_err(err)?;
            match value {
                Some(v) => table.insert(key, v).map_err(err)?.is_some(),
                None => table.remove(key).map_err(err)?.is_some(),
            };
        }
        txn.commit().map_err(err)
    }

    /// 保存した読み込みの負荷レベルの番号（未設定・不正なら `None`）。
    pub fn load_level_number(&self) -> Option<u8> {
        self.setting(LEVEL_KEY)?.parse().ok()
    }

    pub fn set_load_level_number(&self, n: u8) -> Result<(), HistoryError> {
        self.set_setting(LEVEL_KEY, Some(&n.to_string()))
    }

    /// 画像を読み込むか（未設定なら読み込む）。
    pub fn images_enabled(&self) -> bool {
        self.setting(IMAGES_KEY).is_none_or(|v| v != "off")
    }

    pub fn set_images_enabled(&self, on: bool) -> Result<(), HistoryError> {
        self.set_setting(IMAGES_KEY, Some(if on { "on" } else { "off" }))
    }

    /// パスワードを保存・自動入力するか（未設定なら使う）。
    pub fn passwords_enabled(&self) -> bool {
        self.setting(PASSWORDS_KEY).is_none_or(|v| v != "off")
    }

    pub fn set_passwords_enabled(&self, on: bool) -> Result<(), HistoryError> {
        self.set_setting(PASSWORDS_KEY, Some(if on { "on" } else { "off" }))
    }

    /// 保存した表示言語（未設定・不明なら `None`）。
    pub fn language(&self) -> Option<kumo_content::Lang> {
        kumo_content::Lang::from_code(&self.setting(LANGUAGE_KEY)?)
    }

    pub fn set_language(&self, lang: kumo_content::Lang) -> Result<(), HistoryError> {
        self.set_setting(LANGUAGE_KEY, Some(lang.code()))
    }

    /// 設定したホーム URL（未設定なら `None`）。
    pub fn home_url(&self) -> Option<String> {
        self.setting(HOME_KEY).filter(|u| !u.is_empty())
    }

    /// ホーム URL を設定する（`None` で解除）。
    pub fn set_home_url(&self, url: Option<&str>) -> Result<(), HistoryError> {
        self.set_setting(HOME_KEY, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_url_persists_and_clears() {
        let path = std::env::temp_dir().join(format!("kumo-settings-{}.redb", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let db = HistoryDb::open(&path).unwrap();
            assert_eq!(db.home_url(), None);
            db.set_home_url(Some("https://a.example/")).unwrap();
            assert_eq!(db.load_level_number(), None);
            db.set_load_level_number(5).unwrap();
        }
        let db = HistoryDb::open(&path).unwrap();
        assert_eq!(db.home_url().as_deref(), Some("https://a.example/"));
        assert_eq!(db.load_level_number(), Some(5));
        assert!(db.images_enabled(), "既定は読み込む");
        assert_eq!(db.language(), None);
        db.set_language(kumo_content::Lang::Ko).unwrap();
        assert_eq!(db.language(), Some(kumo_content::Lang::Ko));
        db.set_images_enabled(false).unwrap();
        assert!(!db.images_enabled());
        db.set_home_url(None).unwrap();
        assert_eq!(db.home_url(), None);
        drop(db);
        let _ = std::fs::remove_file(&path);
    }
}
